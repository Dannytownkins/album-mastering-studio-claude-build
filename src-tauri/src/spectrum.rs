//! Live FFT spectrum for the EQ panel (UI_LAYOUT_REVISION_1600x940 L4b).
//!
//! Architecture: `MasteringSource` (on the audio thread) pushes the mono
//! mix of each post-chain output frame into `SpectrumRing` via lock-free
//! atomic stores. The audio thread's snapshot loop runs
//! `SpectrumAnalyzer::compute()` once per snapshot tick (~50 ms), which
//! snapshots the ring, applies a Hann window + 2048-point real-FFT,
//! log-bins the magnitudes into 32 bands, smooths exponentially in dB,
//! and packs the result into `PlaybackSnapshot::spectrum_db`. The
//! frontend reads it through the existing `playback:tick` event channel
//! and renders bars under the EQ response curve.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use rustfft::num_complex::Complex;

pub const SPECTRUM_N_SAMPLES: usize = 2048;
pub const SPECTRUM_N_BINS: usize = 32;
const SPECTRUM_F_MIN_HZ: f32 = 20.0;
const SPECTRUM_F_MAX_HZ: f32 = 20_000.0;
const SPECTRUM_FLOOR_DB: f32 = -60.0;
const SPECTRUM_CEIL_DB: f32 = 6.0;
const SPECTRUM_SMOOTHING_ALPHA: f32 = 0.55; // new-sample weight

/// Lock-free ring buffer of recent post-chain mono samples.
/// Per-slot atomic f32-bits + atomic cursor — the audio thread can
/// push at full sample rate with Relaxed ordering, and the snapshot
/// thread reads a coherent snapshot of the cursor + slot values
/// without ever blocking the audio thread.
pub struct SpectrumRing {
    samples: Vec<AtomicU32>,
    cursor: AtomicUsize,
}

impl SpectrumRing {
    pub fn new() -> Self {
        let samples = (0..SPECTRUM_N_SAMPLES).map(|_| AtomicU32::new(0)).collect();
        Self {
            samples,
            cursor: AtomicUsize::new(0),
        }
    }

    /// Audio thread — append one mono sample.
    pub fn push(&self, sample: f32) {
        let idx = self.cursor.fetch_add(1, Ordering::Relaxed) % SPECTRUM_N_SAMPLES;
        self.samples[idx].store(sample.to_bits(), Ordering::Relaxed);
    }

    /// Snapshot the ring's current contents into `out` (length must be
    /// `SPECTRUM_N_SAMPLES`). Time-ordered so [0] is the oldest sample
    /// in the window and [N-1] is the most recent.
    fn snapshot_into(&self, out: &mut [f32]) {
        debug_assert_eq!(out.len(), SPECTRUM_N_SAMPLES);
        let start = self.cursor.load(Ordering::Relaxed) % SPECTRUM_N_SAMPLES;
        for i in 0..SPECTRUM_N_SAMPLES {
            let src = (start + i) % SPECTRUM_N_SAMPLES;
            out[i] = f32::from_bits(self.samples[src].load(Ordering::Relaxed));
        }
    }
}

impl Default for SpectrumRing {
    fn default() -> Self {
        Self::new()
    }
}

/// Holds the FFT planner + reusable scratch buffers + per-bin
/// smoothing state. Owned by `AudioThreadState`; runs on the audio
/// thread's snapshot tick path.
pub struct SpectrumAnalyzer {
    fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    time_domain: Vec<f32>,
    window: Vec<f32>,
    bin_starts: Vec<usize>,
    bin_ends: Vec<usize>,
    prev_db: Vec<f32>,
}

impl SpectrumAnalyzer {
    pub fn new(sample_rate: u32) -> Self {
        let mut planner = rustfft::FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(SPECTRUM_N_SAMPLES);
        // Hann window — cheap rolloff at the buffer edges so the FFT
        // doesn't read low-level discontinuity noise as broadband content.
        let window: Vec<f32> = (0..SPECTRUM_N_SAMPLES)
            .map(|i| {
                let x = (i as f32) / ((SPECTRUM_N_SAMPLES - 1) as f32);
                0.5 - 0.5 * (2.0 * std::f32::consts::PI * x).cos()
            })
            .collect();
        // Log-spaced bin edges in FFT bin index. Each visual bar
        // averages magnitude across its assigned FFT bins.
        let nyquist = (sample_rate as f32) / 2.0;
        let f_max = SPECTRUM_F_MAX_HZ.min(nyquist);
        let log_min = SPECTRUM_F_MIN_HZ.log10();
        let log_max = f_max.log10();
        let n_half = SPECTRUM_N_SAMPLES / 2;
        let mut bin_starts = vec![0usize; SPECTRUM_N_BINS];
        let mut bin_ends = vec![0usize; SPECTRUM_N_BINS];
        for b in 0..SPECTRUM_N_BINS {
            let t_lo = (b as f32) / (SPECTRUM_N_BINS as f32);
            let t_hi = ((b + 1) as f32) / (SPECTRUM_N_BINS as f32);
            let f_lo = 10.0_f32.powf(log_min + (log_max - log_min) * t_lo);
            let f_hi = 10.0_f32.powf(log_min + (log_max - log_min) * t_hi);
            let bin_lo = ((f_lo / (sample_rate as f32)) * (SPECTRUM_N_SAMPLES as f32)) as usize;
            let bin_hi = ((f_hi / (sample_rate as f32)) * (SPECTRUM_N_SAMPLES as f32)) as usize;
            let s = bin_lo.clamp(1, n_half.saturating_sub(1));
            let e = bin_hi.clamp(s + 1, n_half);
            bin_starts[b] = s;
            bin_ends[b] = e;
        }
        Self {
            fft,
            scratch: vec![Complex::new(0.0, 0.0); SPECTRUM_N_SAMPLES],
            time_domain: vec![0.0; SPECTRUM_N_SAMPLES],
            window,
            bin_starts,
            bin_ends,
            prev_db: vec![SPECTRUM_FLOOR_DB; SPECTRUM_N_BINS],
        }
    }

    /// Run one analysis pass and return a fresh Vec<f32> of N_BINS dB
    /// values (`SPECTRUM_FLOOR_DB` floor, `SPECTRUM_CEIL_DB` ceil),
    /// smoothed against the previous tick via exponential filter.
    pub fn compute(&mut self, ring: &SpectrumRing) -> Vec<f32> {
        ring.snapshot_into(&mut self.time_domain);
        // Window + pack into complex scratch.
        for i in 0..SPECTRUM_N_SAMPLES {
            self.scratch[i] = Complex::new(self.time_domain[i] * self.window[i], 0.0);
        }
        self.fft.process(&mut self.scratch);
        // Normalization: divide the magnitude sum by N_SAMPLES / 2 so a
        // sine of amplitude 1.0 reads near 0 dB at its bin.
        let inv_norm = 2.0 / (SPECTRUM_N_SAMPLES as f32);
        let mut out = vec![SPECTRUM_FLOOR_DB; SPECTRUM_N_BINS];
        for b in 0..SPECTRUM_N_BINS {
            let s = self.bin_starts[b];
            let e = self.bin_ends[b];
            let count = (e - s).max(1) as f32;
            let mut sum_pow = 0.0_f32;
            for k in s..e {
                let c = self.scratch[k];
                sum_pow += c.re * c.re + c.im * c.im;
            }
            // RMS magnitude of the bin, scaled.
            let rms = ((sum_pow / count).sqrt()) * inv_norm;
            let db = if rms > 1.0e-12 { 20.0 * rms.log10() } else { SPECTRUM_FLOOR_DB };
            // Exponential smoothing in dB so the visual stays calm.
            let prev = self.prev_db[b];
            let smoothed = prev * (1.0 - SPECTRUM_SMOOTHING_ALPHA) + db * SPECTRUM_SMOOTHING_ALPHA;
            let clamped = smoothed.clamp(SPECTRUM_FLOOR_DB, SPECTRUM_CEIL_DB);
            self.prev_db[b] = clamped;
            out[b] = clamped;
        }
        out
    }

    /// Silent (all-floor) spectrum used when no MasteringSource is
    /// producing samples (Original playback or idle).
    pub fn silent() -> Vec<f32> {
        vec![SPECTRUM_FLOOR_DB; SPECTRUM_N_BINS]
    }

    /// Reset smoothing state — called when a new playback starts so
    /// the bars don't bleed over from the previous track.
    pub fn reset(&mut self) {
        for v in &mut self.prev_db {
            *v = SPECTRUM_FLOOR_DB;
        }
    }
}

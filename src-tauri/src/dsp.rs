use crate::types::*;
use std::f32::consts::PI;

// ============================================================================
// Biquad — direct form II transposed
// Coefficients computed per RBJ Audio EQ Cookbook
// ============================================================================

#[derive(Debug, Clone, Copy)]
pub struct BiquadCoeffs {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl BiquadCoeffs {
    pub fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }

    pub fn low_shelf(sample_rate: f32, freq_hz: f32, gain_db: f32, slope: f32) -> Self {
        if gain_db.abs() < 1.0e-4 {
            return Self::identity();
        }
        let a = 10.0_f32.powf(gain_db / 40.0);
        let omega = 2.0 * PI * freq_hz / sample_rate;
        let cos_o = omega.cos();
        let sin_o = omega.sin();
        let alpha = sin_o / 2.0 * ((a + 1.0 / a) * (1.0 / slope - 1.0) + 2.0).sqrt();
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_o + two_sqrt_a_alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_o);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_o - two_sqrt_a_alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos_o + two_sqrt_a_alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_o);
        let a2 = (a + 1.0) + (a - 1.0) * cos_o - two_sqrt_a_alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    pub fn high_shelf(sample_rate: f32, freq_hz: f32, gain_db: f32, slope: f32) -> Self {
        if gain_db.abs() < 1.0e-4 {
            return Self::identity();
        }
        let a = 10.0_f32.powf(gain_db / 40.0);
        let omega = 2.0 * PI * freq_hz / sample_rate;
        let cos_o = omega.cos();
        let sin_o = omega.sin();
        let alpha = sin_o / 2.0 * ((a + 1.0 / a) * (1.0 / slope - 1.0) + 2.0).sqrt();
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_o + two_sqrt_a_alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_o);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_o - two_sqrt_a_alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_o + two_sqrt_a_alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_o);
        let a2 = (a + 1.0) - (a - 1.0) * cos_o - two_sqrt_a_alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    pub fn peaking(sample_rate: f32, freq_hz: f32, q: f32, gain_db: f32) -> Self {
        if gain_db.abs() < 1.0e-4 {
            return Self::identity();
        }
        let a = 10.0_f32.powf(gain_db / 40.0);
        let omega = 2.0 * PI * freq_hz / sample_rate;
        let cos_o = omega.cos();
        let alpha = omega.sin() / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_o;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_o;
        let a2 = 1.0 - alpha / a;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BiquadState {
    z1: f32,
    z2: f32,
}

impl BiquadState {
    pub fn process(&mut self, c: &BiquadCoeffs, x: f32) -> f32 {
        let y = c.b0 * x + self.z1;
        self.z1 = c.b1 * x - c.a1 * y + self.z2;
        self.z2 = c.b2 * x - c.a2 * y;
        y
    }
}

// ============================================================================
// MasteringChain — gain → 3-band EQ → optional saturation → soft-clip ceiling
// Phase 4.1 only: no real compressor, no lookahead limiter, no true-peak.
// Those land in Phase 11 (DSP audit).
// ============================================================================

#[derive(Debug, Clone, Copy)]
pub struct ChainCoeffs {
    pub low: BiquadCoeffs,
    pub mid: BiquadCoeffs,
    pub high: BiquadCoeffs,
    pub input_gain_lin: f32,
    pub saturation_amount: f32,
    pub ceiling_lin: f32,
}

impl ChainCoeffs {
    pub fn from_settings(sample_rate: u32, settings: &MasteringSettings) -> Self {
        let sr = sample_rate as f32;
        let low = BiquadCoeffs::low_shelf(sr, 200.0, settings.eq_low_db, 0.7);
        let mid = BiquadCoeffs::peaking(sr, 1500.0, 0.8, settings.eq_mid_db);
        let high = BiquadCoeffs::high_shelf(sr, 6000.0, settings.eq_high_db, 0.7);

        let intensity = settings.intensity.clamp(0.0, 1.0);
        let preset_gain_db = match settings.preset {
            Preset::Universal => 1.5,
            Preset::Clarity => 1.5,
            Preset::Tape => 1.0,
            Preset::Spatial => 1.5,
            Preset::Oomph => 2.0,
            Preset::Warmth => 1.0,
            Preset::Punch => 2.0,
            Preset::Loud => 3.5,
            Preset::Custom { .. } => 1.5,
        };
        let input_gain_db = preset_gain_db + intensity * 4.5;
        let input_gain_lin = 10.0_f32.powf(input_gain_db / 20.0);

        let saturation_amount = match settings.preset {
            Preset::Tape => 0.35 + intensity * 0.25,
            Preset::Warmth => 0.15 + intensity * 0.15,
            _ => 0.0,
        };

        let ceiling_db = settings
            .advanced
            .ceiling_dbtp
            .unwrap_or(-1.0)
            .clamp(-6.0, 0.0);
        let ceiling_lin = 10.0_f32.powf(ceiling_db / 20.0);

        Self {
            low,
            mid,
            high,
            input_gain_lin,
            saturation_amount,
            ceiling_lin,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChannelState {
    low: BiquadState,
    mid: BiquadState,
    high: BiquadState,
}

// ============================================================================
// Limiter — linked-stereo brick-wall limiter with lookahead.
// Phase 11.2.a: sample-peak detection (not yet true-peak), instant attack,
// exponential release. Phase 11.2.b can upgrade to 4× oversampled true-peak.
// ============================================================================

#[derive(Debug, Clone)]
pub struct Limiter {
    channels: usize,
    ceiling_lin: f32,
    lookahead_frames: usize,
    release_coef: f32,
    /// Ring buffer of interleaved samples sized `lookahead_frames * channels`.
    buffer: Vec<f32>,
    /// Index of the next frame to overwrite (also the oldest frame in the buffer).
    head_frame: usize,
    /// How many frames have been written so far (capped at `lookahead_frames`).
    filled_frames: usize,
    /// Current linear gain reduction (1.0 = no reduction).
    gain: f32,
    /// Reusable scratch buffer for the oldest-frame read; preallocated to avoid
    /// per-frame allocations on the audio thread.
    oldest_frame_buf: Vec<f32>,
}

impl Limiter {
    pub fn new(
        sample_rate: u32,
        channels: usize,
        ceiling_dbfs: f32,
        lookahead_ms: f32,
        release_ms: f32,
    ) -> Self {
        let ch = channels.max(1);
        let lookahead_frames =
            (((lookahead_ms / 1000.0) * sample_rate as f32).round() as usize).max(1);
        let release_coef = if release_ms > 0.0 {
            (-1.0_f32 / (release_ms / 1000.0 * sample_rate as f32)).exp()
        } else {
            0.0
        };
        let ceiling_lin = 10.0_f32.powf(ceiling_dbfs / 20.0);
        Self {
            channels: ch,
            ceiling_lin,
            lookahead_frames,
            release_coef,
            buffer: vec![0.0; lookahead_frames * ch],
            head_frame: 0,
            filled_frames: 0,
            gain: 1.0,
            oldest_frame_buf: vec![0.0; ch],
        }
    }

    /// Process one frame in place. `frame.len()` must equal `self.channels`
    /// (smaller frames are zero-padded internally).
    pub fn process_frame_inplace(&mut self, frame: &mut [f32]) {
        let ch = self.channels;
        if ch == 0 {
            return;
        }
        let head_base = self.head_frame * ch;
        // Read the OLDEST frame from the ring before overwriting.
        for i in 0..ch {
            self.oldest_frame_buf[i] = self.buffer[head_base + i];
        }
        // Write the new frame into the ring.
        for i in 0..ch {
            let s = if i < frame.len() { frame[i] } else { 0.0 };
            self.buffer[head_base + i] = s;
        }
        self.head_frame = (self.head_frame + 1) % self.lookahead_frames;
        if self.filled_frames < self.lookahead_frames {
            self.filled_frames += 1;
        }

        // Scan the buffer for the peak (linked stereo — single max across all
        // channels). Cost: O(lookahead_frames * channels) per frame. For 3 ms
        // lookahead at 44.1 kHz stereo that's ~264 comparisons/frame ≈ 23 M
        // comparisons/sec — well within budget.
        let mut peak: f32 = 0.0;
        for &s in &self.buffer {
            let a = s.abs();
            if a > peak {
                peak = a;
            }
        }

        let required = if peak > self.ceiling_lin {
            self.ceiling_lin / peak.max(1.0e-9)
        } else {
            1.0
        };

        if required < self.gain {
            // Instant attack — the lookahead gives us time to ramp the OUTPUT
            // down before the peak hits the read pointer, so an instantaneous
            // gain change here translates to a smooth dip in the audible output.
            self.gain = required;
        } else {
            // Exponential release toward `required` (which is 1.0 when no
            // reduction is currently needed).
            self.gain = required - (required - self.gain) * self.release_coef;
        }

        // Output the OLDEST frame * current gain.
        for i in 0..frame.len().min(ch) {
            frame[i] = self.oldest_frame_buf[i] * self.gain;
        }
    }

    pub fn reset(&mut self) {
        for s in self.buffer.iter_mut() {
            *s = 0.0;
        }
        self.head_frame = 0;
        self.filled_frames = 0;
        self.gain = 1.0;
    }
}

pub struct MasteringChain {
    pub coeffs: ChainCoeffs,
    pub states: Vec<ChannelState>,
    pub limiter: Limiter,
}

const LIMITER_LOOKAHEAD_MS: f32 = 3.0;
const LIMITER_RELEASE_MS: f32 = 50.0;

impl MasteringChain {
    pub fn new(sample_rate: u32, channels: usize, settings: &MasteringSettings) -> Self {
        let coeffs = ChainCoeffs::from_settings(sample_rate, settings);
        let states = (0..channels).map(|_| ChannelState::default()).collect();
        let ceiling_dbfs = settings
            .advanced
            .ceiling_dbtp
            .unwrap_or(-1.0)
            .clamp(-6.0, 0.0);
        let limiter = Limiter::new(
            sample_rate,
            channels,
            ceiling_dbfs,
            LIMITER_LOOKAHEAD_MS,
            LIMITER_RELEASE_MS,
        );
        Self {
            coeffs,
            states,
            limiter,
        }
    }

    /// Build a sibling chain that inherits the current filter + limiter state
    /// but uses fresh coefficients. Used by `MasteringSource` to crossfade
    /// between old and new coefficients without re-ringing the filters or
    /// dropping the limiter's gain envelope from zero state.
    pub fn with_coeffs_inheriting_state(coeffs: ChainCoeffs, prior: &Self) -> Self {
        Self {
            coeffs,
            states: prior.states.clone(),
            limiter: prior.limiter.clone(),
        }
    }

    /// Process one interleaved frame in place. Runs gain → 3-band EQ →
    /// saturation per channel, then the linked-stereo lookahead limiter
    /// across the frame.
    pub fn process_frame_inplace(&mut self, frame: &mut [f32]) {
        let channels = frame.len().min(self.states.len());
        if channels == 0 {
            return;
        }
        for ch in 0..channels {
            let state = &mut self.states[ch];
            let mut y = frame[ch] * self.coeffs.input_gain_lin;
            y = state.low.process(&self.coeffs.low, y);
            y = state.mid.process(&self.coeffs.mid, y);
            y = state.high.process(&self.coeffs.high, y);
            if self.coeffs.saturation_amount > 0.0 {
                let drive = 1.0 + self.coeffs.saturation_amount * 2.0;
                y = (y * drive).tanh() / drive.tanh().max(1.0e-3);
            }
            frame[ch] = y;
        }
        self.limiter.process_frame_inplace(frame);
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32], channels: usize) {
        if channels == 0 || self.states.is_empty() {
            return;
        }
        for frame in samples.chunks_mut(channels) {
            self.process_frame_inplace(frame);
        }
    }

    /// Per-sample API. Bypasses the linked-stereo limiter (which needs a full
    /// frame to compute peaks). Used only as a legacy path for callers that
    /// haven't been migrated to `process_frame_inplace`; the soft-clip ceiling
    /// stays in place as a degraded fallback.
    pub fn process_sample(&mut self, sample: f32, channel: usize) -> f32 {
        let idx = if self.states.is_empty() {
            return sample;
        } else {
            channel.min(self.states.len() - 1)
        };
        let state = &mut self.states[idx];
        let mut y = sample * self.coeffs.input_gain_lin;
        y = state.low.process(&self.coeffs.low, y);
        y = state.mid.process(&self.coeffs.mid, y);
        y = state.high.process(&self.coeffs.high, y);
        if self.coeffs.saturation_amount > 0.0 {
            let drive = 1.0 + self.coeffs.saturation_amount * 2.0;
            y = (y * drive).tanh() / drive.tanh().max(1.0e-3);
        }
        let ceiling = self.coeffs.ceiling_lin;
        if y.abs() > ceiling {
            let over = y.abs() - ceiling;
            let shaped = ceiling + over.tanh() * 0.05;
            y = y.signum() * shaped;
        }
        y
    }

    pub fn reset_states(&mut self) {
        for state in self.states.iter_mut() {
            *state = ChannelState::default();
        }
        self.limiter.reset();
    }
}

use crate::types::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

/// Sentinel dBFS value reported when the peak window saw no signal. JSON can't
/// round-trip -inf, so we use a finite "well below audible" floor instead.
pub const SILENCE_DBFS: f32 = -120.0;

/// Convert a non-negative linear sample magnitude to dBFS, with a silence
/// sentinel for inputs at or below 0. Caller is responsible for ensuring the
/// input is finite (filter NaN/inf before calling).
fn linear_to_dbfs(linear: f32) -> f32 {
    if linear > 0.0 {
        20.0 * linear.log10()
    } else {
        SILENCE_DBFS
    }
}

use std::collections::HashMap;

use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use std::sync::atomic::AtomicUsize;
use rustfft::num_complex::Complex;

/// UI_LAYOUT_REVISION_1600x940 L4b — live FFT spectrum for the EQ panel.
///
/// Architecture: MasteringSource (on the audio thread) pushes the mono
/// mix of each post-chain output frame into `SpectrumRing` via
/// lock-free atomic stores. The audio thread's own snapshot loop runs
/// `SpectrumAnalyzer::compute()` once per snapshot tick (~50 ms), which
/// snapshots the ring, applies a Hann window + 2048-point real-FFT,
/// log-bins the magnitudes into 32 bands, smooths exponentially in dB,
/// and packs the result into `PlaybackSnapshot::spectrum_db`. The
/// frontend reads it through the existing `playback:tick` event channel
/// and renders bars under the EQ response curve.
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

const DEFAULT_TARGET_PIXELS: u32 = 1000;
const MIN_TARGET_PIXELS: u32 = 64;

#[tauri::command]
pub async fn prepare_waveform(
    track_id: TrackId,
    track_path: String,
    target_pixels: Option<u32>,
) -> CommandResult<WaveformPeaks> {
    if track_path.is_empty() {
        return Err(CommandError::InvalidPath("empty path".to_string()));
    }
    let path = Path::new(&track_path);
    if crate::files::has_parent_dir_component(path) {
        return Err(CommandError::InvalidPath(format!(
            "path traversal not allowed: {track_path}"
        )));
    }
    let pixels = target_pixels
        .unwrap_or(DEFAULT_TARGET_PIXELS)
        .max(MIN_TARGET_PIXELS);
    let decoded = decode_to_peaks(path, pixels)?;
    Ok(WaveformPeaks {
        track_id,
        channels: decoded.channels,
        samples_per_pixel: decoded.samples_per_pixel,
        total_samples: decoded.total_samples,
        sample_rate: decoded.sample_rate,
    })
}

#[tauri::command]
pub async fn play_track(
    track_id: TrackId,
    track_path: String,
    start_position_sec: Option<f64>,
    player: tauri::State<'_, Arc<AudioPlayer>>,
) -> CommandResult<()> {
    if track_path.is_empty() {
        return Err(CommandError::InvalidPath("empty path".to_string()));
    }
    let path = Path::new(&track_path);
    if crate::files::has_parent_dir_component(path) {
        return Err(CommandError::InvalidPath(format!(
            "path traversal not allowed: {track_path}"
        )));
    }
    player.play_track(track_id, path, start_position_sec.unwrap_or(0.0))
}

#[tauri::command]
pub async fn play_master(
    track_id: TrackId,
    track_path: String,
    settings: MasteringSettings,
    start_position_sec: Option<f64>,
    preview_lufs_landing: Option<bool>,
    player: tauri::State<'_, Arc<AudioPlayer>>,
) -> CommandResult<()> {
    if track_path.is_empty() {
        return Err(CommandError::InvalidPath("empty path".to_string()));
    }
    let path = Path::new(&track_path);
    if crate::files::has_parent_dir_component(path) {
        return Err(CommandError::InvalidPath(format!(
            "path traversal not allowed: {track_path}"
        )));
    }
    player.play_master(
        track_id,
        path,
        settings,
        start_position_sec.unwrap_or(0.0),
        preview_lufs_landing.unwrap_or(true),
    )
}

#[tauri::command]
pub async fn update_chain(
    settings: MasteringSettings,
    preview_lufs_landing: Option<bool>,
    player: tauri::State<'_, Arc<AudioPlayer>>,
) -> CommandResult<()> {
    player.update_chain(settings, preview_lufs_landing.unwrap_or(true))
}

#[tauri::command]
pub async fn pause_playback(
    player: tauri::State<'_, Arc<AudioPlayer>>,
) -> CommandResult<()> {
    player.pause();
    Ok(())
}

#[tauri::command]
pub async fn resume_playback(
    player: tauri::State<'_, Arc<AudioPlayer>>,
) -> CommandResult<()> {
    player.resume();
    Ok(())
}

#[tauri::command]
pub async fn stop_playback(
    player: tauri::State<'_, Arc<AudioPlayer>>,
) -> CommandResult<()> {
    player.stop();
    Ok(())
}

#[tauri::command]
pub async fn seek_playback(
    position_sec: f64,
    player: tauri::State<'_, Arc<AudioPlayer>>,
) -> CommandResult<()> {
    if !position_sec.is_finite() || position_sec < 0.0 {
        return Err(CommandError::Other(format!(
            "invalid seek position: {position_sec}"
        )));
    }
    player.seek(position_sec)
}

#[tauri::command]
pub async fn set_loop_region(
    region: Option<LoopRegion>,
    player: tauri::State<'_, Arc<AudioPlayer>>,
) -> CommandResult<()> {
    if let Some(r) = region {
        if !r.start_sec.is_finite() || !r.end_sec.is_finite() {
            return Err(CommandError::Other("loop region must be finite".to_string()));
        }
        if r.start_sec < 0.0 {
            return Err(CommandError::Other(format!(
                "loop start must be >= 0, got {}",
                r.start_sec
            )));
        }
        if r.end_sec <= r.start_sec {
            return Err(CommandError::Other(format!(
                "loop end ({}) must be > start ({})",
                r.end_sec, r.start_sec
            )));
        }
    }
    player.set_loop(region)
}

pub struct DecodedPeaks {
    pub channels: Vec<Vec<f32>>,
    pub samples_per_pixel: u32,
    pub total_samples: u64,
    pub sample_rate: u32,
}

#[derive(Debug, Clone)]
pub struct DecodedPcm {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

pub fn decode_full(path: &Path) -> CommandResult<DecodedPcm> {
    let file = std::fs::File::open(path).map_err(|e| CommandError::Io(e.to_string()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| CommandError::Decode(e.to_string()))?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| CommandError::Decode("no decodable track".to_string()))?;
    let stream_track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44_100);
    let channel_count = track
        .codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(2)
        .max(1) as u16;
    let estimated_capacity = track
        .codec_params
        .n_frames
        .unwrap_or(0)
        .saturating_mul(channel_count as u64) as usize;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| CommandError::Decode(e.to_string()))?;

    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut samples: Vec<f32> = Vec::with_capacity(estimated_capacity);

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::ResetRequired) => break,
            Err(e) => return Err(CommandError::Decode(e.to_string())),
        };
        if packet.track_id() != stream_track_id {
            continue;
        }
        let decoded: AudioBufferRef = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::IoError(_)) => continue,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(CommandError::Decode(e.to_string())),
        };
        if sample_buf.is_none() {
            let spec = *decoded.spec();
            let duration = decoded.capacity() as u64;
            sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
        }
        let sbuf = sample_buf.as_mut().unwrap();
        sbuf.copy_interleaved_ref(decoded);
        samples.extend_from_slice(sbuf.samples());
    }

    Ok(DecodedPcm {
        samples,
        sample_rate,
        channels: channel_count,
    })
}

pub fn decode_to_peaks(path: &Path, target_pixels: u32) -> CommandResult<DecodedPeaks> {
    let file = std::fs::File::open(path).map_err(|e| CommandError::Io(e.to_string()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| CommandError::Decode(e.to_string()))?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| CommandError::Decode("no decodable track".to_string()))?;
    let stream_track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44_100);
    let channel_count = track
        .codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(2)
        .max(1);
    let total_frames = track.codec_params.n_frames.unwrap_or(0);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| CommandError::Decode(e.to_string()))?;

    let samples_per_pixel = if total_frames > 0 {
        ((total_frames as f64 / target_pixels as f64).ceil() as u32).max(1)
    } else {
        (sample_rate / 50).max(1)
    };

    let mut channel_peaks: Vec<Vec<f32>> =
        vec![Vec::with_capacity(target_pixels as usize); channel_count];
    let mut running_max: Vec<f32> = vec![0.0; channel_count];
    let mut window_frames: u64 = 0;
    let mut total_decoded_frames: u64 = 0;
    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::ResetRequired) => break,
            Err(e) => return Err(CommandError::Decode(e.to_string())),
        };
        if packet.track_id() != stream_track_id {
            continue;
        }
        let decoded: AudioBufferRef = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::IoError(_)) => continue,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(CommandError::Decode(e.to_string())),
        };
        if sample_buf.is_none() {
            let spec = *decoded.spec();
            let duration = decoded.capacity() as u64;
            sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
        }
        let sbuf = sample_buf.as_mut().unwrap();
        sbuf.copy_interleaved_ref(decoded);
        let samples = sbuf.samples();
        let frames = samples.len() / channel_count.max(1);
        total_decoded_frames += frames as u64;

        for frame in 0..frames {
            for ch in 0..channel_count {
                let v = samples[frame * channel_count + ch].abs();
                if v > running_max[ch] {
                    running_max[ch] = v;
                }
            }
            window_frames += 1;
            if window_frames >= u64::from(samples_per_pixel) {
                for ch in 0..channel_count {
                    channel_peaks[ch].push(running_max[ch]);
                    running_max[ch] = 0.0;
                }
                window_frames = 0;
            }
        }
    }

    if window_frames > 0 {
        for ch in 0..channel_count {
            channel_peaks[ch].push(running_max[ch]);
        }
    }

    Ok(DecodedPeaks {
        channels: channel_peaks,
        samples_per_pixel,
        total_samples: total_decoded_frames,
        sample_rate,
    })
}

// ============================================================================
// AudioPlayer — a Send + Sync handle to a dedicated audio thread that owns
// the rodio OutputStream + Sink (which are !Send). Commands flow over an
// mpsc channel; the current playback snapshot is shared via Arc<RwLock<_>>.
// ============================================================================

enum AudioCommand {
    Play {
        track_id: TrackId,
        path: PathBuf,
        start_position_sec: f64,
        reply: Sender<Result<(), String>>,
    },
    PlayMaster {
        track_id: TrackId,
        path: PathBuf,
        settings: MasteringSettings,
        start_position_sec: f64,
        preview_lufs_landing: bool,
        reply: Sender<Result<(), String>>,
    },
    UpdateChain {
        settings: MasteringSettings,
        preview_lufs_landing: bool,
    },
    Pause,
    Resume,
    Stop,
    Seek {
        position_sec: f64,
        reply: Sender<Result<(), String>>,
    },
    SetLoop(Option<LoopRegion>),
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct PlaybackSnapshot {
    pub track_id: Option<TrackId>,
    pub position_sec: f64,
    pub is_playing: bool,
    pub is_loaded: bool,
    /// Post-output-gain peak across all channels since the last snapshot tick.
    /// `SILENCE_DBFS` when there was no signal in the window (e.g. source
    /// playback, idle, or pure silence). Computed inside the audio thread by
    /// swap-and-converting the shared peak atomic.
    pub peak_dbfs: f32,
    /// Phase 12.2 — per-band compressor gain reduction (in dB, negative)
    /// since the last snapshot tick. `SILENCE_DBFS` when the window had no
    /// reduction or no signal.
    pub gr_low_db: f32,
    pub gr_mid_db: f32,
    pub gr_high_db: f32,
    /// Phase 12.2 P3 — BS.1770 momentary LUFS readout (400 ms window).
    /// Computed inside MasteringSource by the K-weighted prefilter +
    /// 400 ms sliding mean-square. `SILENCE_DBFS` while the meter is
    /// still priming or when input is silent.
    pub lufs_momentary: f32,
    /// Phase 12.2 P3+ — BS.1770-4 integrated LUFS over the current playback
    /// session. Updates every 100 ms as new 400 ms blocks complete. Resets
    /// to `SILENCE_DBFS` on each new `play_master`.
    pub lufs_integrated: f32,
    /// L4b — live FFT spectrum, log-binned to `SPECTRUM_N_BINS` dB
    /// values from `SPECTRUM_FLOOR_DB` (~-60) to `SPECTRUM_CEIL_DB`
    /// (~+6). Populated by both Original's pass-through metered source
    /// and Mastered's DSP source so A/B metering compares like with like.
    /// Idle states return all-floor. The frontend draws the bins as a
    /// filled area under the EQ response curve.
    pub spectrum_db: Vec<f32>,
}

impl Default for PlaybackSnapshot {
    fn default() -> Self {
        Self {
            track_id: None,
            position_sec: 0.0,
            is_playing: false,
            is_loaded: false,
            peak_dbfs: SILENCE_DBFS,
            gr_low_db: SILENCE_DBFS,
            gr_mid_db: SILENCE_DBFS,
            gr_high_db: SILENCE_DBFS,
            lufs_momentary: SILENCE_DBFS,
            lufs_integrated: SILENCE_DBFS,
            spectrum_db: SpectrumAnalyzer::silent(),
        }
    }
}

pub struct AudioPlayer {
    tx: Mutex<Option<Sender<AudioCommand>>>,
    snapshot: Arc<RwLock<PlaybackSnapshot>>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let snapshot = Arc::new(RwLock::new(PlaybackSnapshot::default()));
        let (tx, rx) = mpsc::channel::<AudioCommand>();
        let snap_for_thread = snapshot.clone();
        std::thread::Builder::new()
            .name("audio-player".to_string())
            .spawn(move || audio_thread(rx, snap_for_thread))
            .expect("spawn audio thread");
        Self {
            tx: Mutex::new(Some(tx)),
            snapshot,
        }
    }

    pub fn play_track(
        &self,
        track_id: TrackId,
        path: &Path,
        start_position_sec: f64,
    ) -> CommandResult<()> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send(AudioCommand::Play {
            track_id,
            path: path.to_path_buf(),
            start_position_sec: start_position_sec.max(0.0),
            reply: reply_tx,
        })
        .map_err(CommandError::Other)?;
        match reply_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(CommandError::Other(e)),
            Err(_) => Err(CommandError::Other(
                "audio thread reply timeout".to_string(),
            )),
        }
    }

    pub fn play_master(
        &self,
        track_id: TrackId,
        path: &Path,
        settings: MasteringSettings,
        start_position_sec: f64,
        preview_lufs_landing: bool,
    ) -> CommandResult<()> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send(AudioCommand::PlayMaster {
            track_id,
            path: path.to_path_buf(),
            settings,
            start_position_sec: start_position_sec.max(0.0),
            preview_lufs_landing,
            reply: reply_tx,
        })
        .map_err(CommandError::Other)?;
        match reply_rx.recv_timeout(Duration::from_secs(15)) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(CommandError::Other(e)),
            Err(_) => Err(CommandError::Other(
                "audio thread reply timeout".to_string(),
            )),
        }
    }

    pub fn update_chain(
        &self,
        settings: MasteringSettings,
        preview_lufs_landing: bool,
    ) -> CommandResult<()> {
        self.send(AudioCommand::UpdateChain {
            settings,
            preview_lufs_landing,
        })
        .map_err(CommandError::Other)
    }

    pub fn pause(&self) {
        let _ = self.send(AudioCommand::Pause);
    }

    pub fn resume(&self) {
        let _ = self.send(AudioCommand::Resume);
    }

    pub fn stop(&self) {
        let _ = self.send(AudioCommand::Stop);
    }

    pub fn seek(&self, position_sec: f64) -> CommandResult<()> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send(AudioCommand::Seek {
            position_sec,
            reply: reply_tx,
        })
        .map_err(CommandError::Other)?;
        match reply_rx.recv_timeout(Duration::from_secs(2)) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(CommandError::Other(e)),
            Err(_) => Err(CommandError::Other(
                "audio seek reply timeout".to_string(),
            )),
        }
    }

    pub fn set_loop(&self, region: Option<LoopRegion>) -> CommandResult<()> {
        self.send(AudioCommand::SetLoop(region))
            .map_err(CommandError::Other)
    }

    pub fn snapshot(&self) -> PlaybackSnapshot {
        self.snapshot.read().expect("snapshot read").clone()
    }

    fn send(&self, cmd: AudioCommand) -> Result<(), String> {
        let guard = self.tx.lock().map_err(|_| "audio tx mutex poisoned".to_string())?;
        let tx = guard
            .as_ref()
            .ok_or_else(|| "audio thread offline".to_string())?;
        tx.send(cmd)
            .map_err(|_| "audio thread disconnected".to_string())
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.tx.lock() {
            if let Some(tx) = guard.take() {
                let _ = tx.send(AudioCommand::Shutdown);
            }
        }
    }
}

struct AudioThreadState {
    _stream: rodio::OutputStream,
    handle: rodio::OutputStreamHandle,
    sink: rodio::Sink,
    current_track: Option<TrackId>,
    loop_region: Option<LoopRegion>,
    live_coeffs_tx: Option<Sender<crate::dsp::ChainCoeffs>>,
    live_sample_rate: u32,
    /// Phase 12.1 decode cache — keyed by canonical path + mtime. Speeds up
    /// repeated `play_master` calls on the same file (e.g. Original/Mastered
    /// toggles) from ~1–2 s on a multi-minute WAV down to a sub-100 ms swap.
    /// Single-entry LRU is sufficient because the typical Track Master flow
    /// hammers one fixture; album mode keeps the most-recently-played track.
    decoded_cache: Option<DecodedCacheEntry>,
    /// Export-landing-gain cache. Keyed by settings hash, scoped to the
    /// currently-loaded decoded PCM. Cleared whenever
    /// `handle_play_master` swaps in a different canonical path.
    landing_gain_cache: PreviewLandingCache,
    /// Shared post-output-gain peak slot. `MasteringSource` writes via
    /// `fetch_max` per frame; the audio thread `swap`s to 0 each snapshot
    /// cycle to compute "peak since last tick." Bits are an f32 magnitude;
    /// valid because we only ever store non-negative finite values, where
    /// IEEE 754 bit ordering matches numeric ordering.
    peak_linear: Arc<AtomicU32>,
    /// Phase 12.2 — per-band GR snapshot slots. Mirror of `peak_linear`'s
    /// pattern: `MasteringSource` (via the contained `MasteringChain`)
    /// fetch_max's |reduction_db| * 100 as u32 per frame; the audio thread
    /// swaps to 0 each tick and converts to negative dB. 0 = no reduction in
    /// the window.
    gr_low: Arc<AtomicU32>,
    gr_mid: Arc<AtomicU32>,
    gr_high: Arc<AtomicU32>,
    /// Phase 12.2 P3 — live BS.1770 momentary LUFS, stored as LUFS×100 in
    /// AtomicI32 (signed for negative LUFS).  `i32::MIN` = silence sentinel.
    /// MasteringSource overwrites per frame; the audio thread reads (no swap
    /// — we want the current value, not a since-last-tick aggregate).
    lufs_x100: Arc<AtomicI32>,
    /// Phase 12.2 P3+ — live BS.1770-4 integrated LUFS over the current
    /// listen-through. Same storage convention as `lufs_x100`.  Resets to
    /// `i32::MIN` on each `handle_play` / `handle_play_master` so each new
    /// playback session integrates from zero.
    integrated_lufs_x100: Arc<AtomicI32>,
    /// L4b — lock-free ring of mono samples shared with both metered
    /// playback sources. Original feeds source PCM; Mastered feeds
    /// post-chain output.
    spectrum_ring: Arc<SpectrumRing>,
    /// L4b — FFT analyzer that runs once per snapshot tick and turns
    /// the ring into 32 log-binned dB values for the EQ panel.
    spectrum_analyzer: SpectrumAnalyzer,
}

#[derive(Clone)]
struct DecodedCacheEntry {
    canonical_path: PathBuf,
    mtime: Option<std::time::SystemTime>,
    pcm: DecodedPcm,
}

/// Returns the cached PCM if the entry's key matches the given canonical
/// path + mtime. Extracted from handle_play_master so the cache logic is
/// testable without spinning up a real audio device.
fn decode_cache_lookup(
    cache: Option<&DecodedCacheEntry>,
    canonical: &Path,
    mtime: Option<std::time::SystemTime>,
) -> Option<DecodedPcm> {
    cache
        .filter(|entry| entry.canonical_path == canonical && entry.mtime == mtime)
        .map(|entry| entry.pcm.clone())
}

/// Cache of computed export-landing-gain values, keyed by a hash of
/// the settings fields that affect the chain output and the LUFS
/// target. Lifetime is tied to the currently-loaded decoded PCM —
/// when `handle_play_master` swaps in a different track, the cache
/// is cleared because the cached per-setting values were computed
/// against the OLD PCM and are now stale. Original/Mastered toggles
/// on the SAME track preserve the cache because the PCM is unchanged.
///
/// Compounds with the audio-command-loop coalescing: coalescing drops
/// redundant intermediate UpdateChains so only the survivor pays a
/// measurement; caching makes that survivor free when its settings
/// hash has already been seen for this PCM. Typical knob-twiddle
/// A → B → A goes from 3 measurements to 2.
#[derive(Debug, Default)]
struct PreviewLandingCache {
    by_hash: HashMap<u64, f32>,
}

impl PreviewLandingCache {
    fn new() -> Self {
        Self {
            by_hash: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        self.by_hash.clear();
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.by_hash.len()
    }

    /// Lookup the cached landing gain for `settings`. On miss, invoke
    /// `compute` with the settings, store its result, and return it.
    /// `compute` is `FnOnce` so callers can borrow other state from
    /// the audio thread inside the closure without lifetime trouble.
    fn get_or_compute<F>(&mut self, settings: &MasteringSettings, compute: F) -> f32
    where
        F: FnOnce(&MasteringSettings) -> f32,
    {
        let hash = settings_landing_hash(settings);
        if let Some(&cached) = self.by_hash.get(&hash) {
            return cached;
        }
        let result = compute(settings);
        self.by_hash.insert(hash, result);
        result
    }
}

/// Hash of the `MasteringSettings` fields that affect the export
/// landing gain. Uses serde_json for ergonomic correctness — every
/// f32 / enum / Option boundary is handled by serde without bespoke
/// bit-twiddling — at ~50–100 μs per call vs the ~20 ms measurement
/// the cache prevents, the hash cost is negligible.
///
/// Two fields are STRIPPED before hashing so they don't bust the
/// cache when they change without affecting the landing gain:
///
///   * `volume_match` — the measurement always runs with VM off
///     (see `export_landing_gain_lin_for_preview`), so toggling VM
///     in the UI doesn't invalidate the cached value.
///   * `source_lufs_integrated` — affects the VM cap downstream of
///     the chain measurement but not the measurement itself. Stripped
///     so analysis updates (which inject this field) don't bust the
///     cache.
fn settings_landing_hash(settings: &MasteringSettings) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut to_hash = settings.clone();
    to_hash.volume_match = false;
    to_hash.source_lufs_integrated = None;
    let json = serde_json::to_string(&to_hash).unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    json.hash(&mut hasher);
    hasher.finish()
}

fn export_landing_gain_lin_for_preview(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    settings: &MasteringSettings,
) -> Result<f32, String> {
    let mut render_settings = settings.clone();
    render_settings.volume_match = false;
    let Some(target_lufs) = render_settings.effective_target_lufs() else {
        return Ok(1.0);
    };
    if !target_lufs.is_finite() {
        return Ok(1.0);
    }

    // Perf: measure a middle window of the track instead of the full
    // PCM. Full chain + BS.1770 on a 3 min stereo 48 kHz track is
    // ~17M samples = ~200-500 ms per call. Every settings change
    // triggers one of these (when the Export LUFS toggle is on), so
    // knob spam queues 5-10 expensive measurements behind a seek and
    // the audio thread hits the "audio seek reply timeout" 15 s window
    // (Dan, observed on aggressive tweaking). An 8 s window cuts cost
    // by ~15-20x while staying long enough for BS.1770 integrated
    // gating to behave (multiple 400 ms blocks needed); the result
    // sits within ~0.5 dB of full-track for normal music, which is
    // tighter than the chain push estimate's own error budget.
    //
    // Caching by settings hash and async measurement on a worker
    // thread are bigger wins but deferred — this is the smallest
    // change that removes the audible cliff.
    const PREVIEW_WINDOW_SECS: f32 = 8.0;
    let channels_usize = channels.max(1) as usize;
    let safe_channels = channels_usize.max(1);
    let total_frames = samples.len() / safe_channels;
    let window_frames =
        ((PREVIEW_WINDOW_SECS * sample_rate as f32) as usize).min(total_frames);
    let start_frame = total_frames.saturating_sub(window_frames) / 2;
    let start = start_frame * safe_channels;
    let end = ((start_frame + window_frames) * safe_channels).min(samples.len());
    let mut rendered = samples[start..end].to_vec();
    let mut chain = crate::dsp::MasteringChain::new(sample_rate, channels_usize, &render_settings);
    chain.process_interleaved(&mut rendered, channels_usize);

    // Measure post-chain integrated LUFS and BS.1770 true-peak. Both are
    // needed for the ceiling-bounded landing math at engine.rs (see the
    // long comment block there for the rationale). Replicates the same
    // ebur128 call shape the export path uses at engine.rs:1607-1644.
    let channels_u32 = u32::from(channels.max(1));
    let mut ebu = ebur128::EbuR128::new(
        channels_u32,
        sample_rate,
        ebur128::Mode::I | ebur128::Mode::TRUE_PEAK,
    )
    .map_err(|e| format!("ebur128 init: {e}"))?;
    ebu.add_frames_f32(&rendered)
        .map_err(|e| format!("ebur128 feed: {e}"))?;
    let measured = ebu
        .loudness_global()
        .map_err(|e| format!("ebur128 global: {e}"))? as f32;
    if !measured.is_finite() || measured <= -70.0 {
        return Ok(1.0);
    }
    let mut peak_lin: f64 = 0.0;
    for ch in 0..channels_u32 {
        let tp = ebu
            .true_peak(ch)
            .map_err(|e| format!("ebur128 tp: {e}"))?;
        if tp > peak_lin {
            peak_lin = tp;
        }
    }
    let measured_true_peak_dbtp = if peak_lin > 0.0 {
        (20.0 * peak_lin.log10()) as f32
    } else {
        -60.0
    };

    let delta_db = target_lufs - measured;
    let ceiling_dbtp = render_settings.effective_ceiling_dbtp();
    let headroom_db = (ceiling_dbtp - measured_true_peak_dbtp).max(0.0);
    let applied_delta_db = if delta_db < 0.0 {
        delta_db
    } else {
        delta_db.min(headroom_db)
    };

    if applied_delta_db.abs() > 1.0e-4 {
        Ok(10.0_f32.powf(applied_delta_db / 20.0))
    } else {
        Ok(1.0)
    }
}

/// Cache-less variant of the audio thread's UpdateChain coefficient
/// build. The audio thread itself routes through `PreviewLandingCache`
/// now (see the `UpdateChain` branch in `process_audio_command`), but
/// this helper is preserved for tests that want to exercise the raw
/// chain-coefficient + landing-gain composition without setting up a
/// full cache + AudioThreadState.
#[cfg(test)]
fn live_preview_coeffs(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    settings: &MasteringSettings,
    preview_lufs_landing: bool,
) -> Result<crate::dsp::ChainCoeffs, String> {
    let mut coeffs = crate::dsp::ChainCoeffs::from_settings(sample_rate, settings);
    if preview_lufs_landing {
        coeffs.export_landing_gain_lin =
            export_landing_gain_lin_for_preview(samples, sample_rate, channels, settings)?;
    }
    Ok(coeffs)
}

/// Split a buffered batch of audio commands into (in-order non-UpdateChain
/// commands, latest UpdateChain). Used by the audio command loop to
/// coalesce knob-spam: redundant intermediate UpdateChains have stale
/// settings the user has already moved past, and dropping them keeps
/// latency-sensitive commands (Seek / Play / Pause) from being trapped
/// behind preview-LUFS measurements.
///
/// Properties:
/// * Non-UpdateChain commands retain their submission order.
/// * Among UpdateChains, only the LAST one in the buffer survives (its
///   payload is the freshest settings snapshot).
/// * Empty input yields `(vec![], None)`.
///
/// Exposed at module scope (rather than inline in `audio_thread`) so
/// the partition logic is testable in isolation without spinning up
/// a real audio device.
fn partition_for_coalescing(
    buffered: Vec<AudioCommand>,
) -> (Vec<AudioCommand>, Option<AudioCommand>) {
    let mut latest_update: Option<AudioCommand> = None;
    let mut in_order: Vec<AudioCommand> = Vec::with_capacity(buffered.len());
    for c in buffered {
        if matches!(c, AudioCommand::UpdateChain { .. }) {
            latest_update = Some(c);
        } else {
            in_order.push(c);
        }
    }
    (in_order, latest_update)
}

/// Dispatch a single audio command. Returns `true` when Shutdown is
/// received so the caller can break the loop. Extracted from the
/// original inline match so the command loop can buffer + coalesce
/// queued commands before dispatching (see `audio_thread` for the
/// drain pattern).
fn process_audio_command(
    cmd: AudioCommand,
    state: &mut Option<AudioThreadState>,
) -> bool {
    match cmd {
        AudioCommand::Play {
            track_id,
            path,
            start_position_sec,
            reply,
        } => {
            let outcome = handle_play(state, track_id, &path, start_position_sec);
            let _ = reply.send(outcome);
        }
        AudioCommand::PlayMaster {
            track_id,
            path,
            settings,
            start_position_sec,
            preview_lufs_landing,
            reply,
        } => {
            let outcome = handle_play_master(
                state,
                track_id,
                &path,
                &settings,
                start_position_sec,
                preview_lufs_landing,
            );
            let _ = reply.send(outcome);
        }
        AudioCommand::UpdateChain {
            settings,
            preview_lufs_landing,
        } => {
            if let Some(s) = state.as_mut() {
                // Split-borrow the AudioThreadState fields we touch:
                // `landing_gain_cache` mutably (for cache insert) and
                // `decoded_cache` / `live_sample_rate` / `live_coeffs_tx`
                // immutably. Rust permits this when each disjoint field
                // is named explicitly through `&mut s.x` / `&s.y`.
                let sample_rate = s.live_sample_rate;
                let landing_cache = &mut s.landing_gain_cache;
                let decoded_cache = s.decoded_cache.as_ref();
                let tx = s.live_coeffs_tx.as_ref();

                if let Some(tx) = tx {
                    let mut coeffs =
                        crate::dsp::ChainCoeffs::from_settings(sample_rate, &settings);
                    if preview_lufs_landing {
                        if let Some(cache_entry) = decoded_cache {
                            // Cache-aware landing-gain computation.
                            // Cache miss → run the full measurement
                            // through `export_landing_gain_lin_for_preview`
                            // (~20 ms on the 8 s window) and store the
                            // result. Cache hit → 0 ms; return the prior
                            // value computed against the same settings
                            // and PCM.
                            let samples_ref = cache_entry.pcm.samples.as_slice();
                            let channels = cache_entry.pcm.channels;
                            coeffs.export_landing_gain_lin =
                                landing_cache.get_or_compute(&settings, |s_arg| {
                                    export_landing_gain_lin_for_preview(
                                        samples_ref,
                                        sample_rate,
                                        channels,
                                        s_arg,
                                    )
                                    .unwrap_or(1.0)
                                });
                        }
                        // No decoded PCM cached yet → leave landing
                        // gain at 1.0. The next play_master will
                        // populate the decode cache and the next
                        // UpdateChain will compute through the cache.
                    }
                    let _ = tx.send(coeffs);
                }
            }
        }
        AudioCommand::Pause => {
            if let Some(s) = state.as_ref() {
                s.sink.pause();
            }
        }
        AudioCommand::Resume => {
            if let Some(s) = state.as_ref() {
                s.sink.play();
            }
        }
        AudioCommand::Stop => {
            if let Some(s) = state.as_mut() {
                s.sink.stop();
                s.current_track = None;
            }
        }
        AudioCommand::Seek { position_sec, reply } => {
            let outcome = match state.as_ref() {
                Some(s) => s
                    .sink
                    .try_seek(Duration::from_secs_f64(position_sec.max(0.0)))
                    .map_err(|e| e.to_string()),
                None => Err("no track loaded".to_string()),
            };
            let _ = reply.send(outcome);
        }
        AudioCommand::SetLoop(region) => {
            if let Some(s) = state.as_mut() {
                s.loop_region = region.filter(|r| r.end_sec > r.start_sec);
            }
        }
        AudioCommand::Shutdown => return true,
    }
    false
}

fn audio_thread(rx: mpsc::Receiver<AudioCommand>, snapshot: Arc<RwLock<PlaybackSnapshot>>) {
    let mut state: Option<AudioThreadState> = None;
    loop {
        // Wait for at least one command (50 ms tick matches the prior
        // poll cadence so loop-region / snapshot housekeeping below
        // still runs every ~50 ms even when no commands arrive).
        let first_cmd: Option<AudioCommand> =
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(c) => Some(c),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            };

        let mut shutdown_requested = false;

        if let Some(first) = first_cmd {
            // Drain any additional immediately-available commands so
            // we can coalesce UpdateChain dispatches in front of
            // latency-sensitive commands like Seek / Pause / Resume.
            //
            // Each UpdateChain costs ~20 ms even after the 8 s preview
            // window perf fix (full chain over the window + BS.1770
            // measurement when Export LUFS preview is on). Knob spam
            // can pile 5-10 of these into the queue, and any one of
            // them sitting in front of a Seek delays the seek-reply
            // past the frontend's 2 s timeout (Dan: "audio seek reply
            // timeout" toast; Codex pinpointed this loop as the
            // bottleneck).
            //
            // Strategy: pull every command currently in the queue,
            // separate the LATEST UpdateChain from everything else,
            // process the in-order non-UpdateChain commands FIRST
            // (so seeks land immediately), then process the single
            // coalesced UpdateChain last (so live playback ends up at
            // the freshest settings rather than an intermediate
            // state). Intermediate UpdateChains are dropped — the
            // user has already moved past them, their coefficient
            // payloads are obsolete.
            let mut buffered: Vec<AudioCommand> = Vec::with_capacity(4);
            buffered.push(first);
            while let Ok(more) = rx.try_recv() {
                buffered.push(more);
            }

            let (in_order, latest_update) = partition_for_coalescing(buffered);

            for c in in_order {
                if process_audio_command(c, &mut state) {
                    shutdown_requested = true;
                }
            }
            if let Some(c) = latest_update {
                if process_audio_command(c, &mut state) {
                    shutdown_requested = true;
                }
            }
        }

        if shutdown_requested {
            break;
        }

        // Loop enforcement: if a region is set and the playhead has crossed the
        // end point, jump back to start. ~50 ms loop poll latency is acceptable
        // for region listening; tightening lands in Phase 11.
        if let Some(s) = state.as_ref() {
            if let Some(region) = s.loop_region {
                let pos = s.sink.get_pos().as_secs_f64();
                if pos >= region.end_sec {
                    let _ = s
                        .sink
                        .try_seek(Duration::from_secs_f64(region.start_sec.max(0.0)));
                }
            }
        }

        let next_snap = match state.as_mut() {
            Some(s) if s.current_track.is_some() => {
                // Atomic swap consumes the "peak since last tick" and resets
                // the slot to 0 in one step — the writer (MasteringSource) and
                // reader can't race to drop a sample's peak. NaN-poisoned bits
                // (which the source-side guard already filters) would only
                // surface as a one-tick anomaly here, never persistent state.
                let peak_bits = s.peak_linear.swap(0, Ordering::Relaxed);
                let peak_linear = f32::from_bits(peak_bits);
                let peak_dbfs = if peak_linear.is_finite() {
                    linear_to_dbfs(peak_linear)
                } else {
                    SILENCE_DBFS
                };
                // Phase 12.2 — per-band GR snapshot conversion. Atomics hold
                // |reduction_db| * 100 as u32; 0 = no reduction. Convert to
                // negative dB (reduction direction); 0 maps to SILENCE_DBFS
                // so the UI's GR meter reads as idle when nothing is fighting
                // the compressor.
                let gr_u = |a: &Arc<AtomicU32>| a.swap(0, Ordering::Relaxed);
                let to_gr_db = |u: u32| -> f32 {
                    if u == 0 {
                        SILENCE_DBFS
                    } else {
                        -(u as f32) / 100.0
                    }
                };
                let to_lufs = |raw: i32| -> f32 {
                    if raw == i32::MIN {
                        SILENCE_DBFS
                    } else {
                        (raw as f32) / 100.0
                    }
                };
                let lufs_momentary = to_lufs(s.lufs_x100.load(Ordering::Relaxed));
                let lufs_integrated =
                    to_lufs(s.integrated_lufs_x100.load(Ordering::Relaxed));
                let is_playing = !s.sink.is_paused() && !s.sink.empty();
                // L4b — run the FFT analyzer over the ring's current
                // contents. When idle / paused / playing Original (no
                // MasteringSource pushing), the ring still holds the
                // last batch of samples but the FFT reads them as
                // stale; we return the floor instead so the EQ
                // panel's bars decay to silence rather than freezing.
                let spectrum_db = if is_playing {
                    s.spectrum_analyzer.compute(&s.spectrum_ring)
                } else {
                    SpectrumAnalyzer::silent()
                };
                PlaybackSnapshot {
                    track_id: s.current_track.clone(),
                    position_sec: s.sink.get_pos().as_secs_f64(),
                    is_playing,
                    is_loaded: true,
                    peak_dbfs,
                    gr_low_db: to_gr_db(gr_u(&s.gr_low)),
                    gr_mid_db: to_gr_db(gr_u(&s.gr_mid)),
                    gr_high_db: to_gr_db(gr_u(&s.gr_high)),
                    lufs_momentary,
                    lufs_integrated,
                    spectrum_db,
                }
            }
            _ => PlaybackSnapshot::default(),
        };
        if let Ok(mut w) = snapshot.write() {
            *w = next_snap;
        }
    }
}

fn handle_play(
    state: &mut Option<AudioThreadState>,
    track_id: TrackId,
    path: &Path,
    start_position_sec: f64,
) -> Result<(), String> {
    let canonical = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());
    let mtime = std::fs::metadata(&canonical).ok().and_then(|m| m.modified().ok());
    let cache_hit_pcm: Option<DecodedPcm> = decode_cache_lookup(
        state.as_ref().and_then(|s| s.decoded_cache.as_ref()),
        &canonical,
        mtime,
    );

    let pcm = match cache_hit_pcm {
        Some(p) => p,
        None => {
            let decoded =
                crate::audio::decode_full(path).map_err(|e| format!("{e}"))?;
            if decoded.samples.is_empty() {
                return Err("no samples decoded for source playback".to_string());
            }
            decoded
        }
    };

    if state.is_none() {
        let (stream, handle) = rodio::OutputStream::try_default()
            .map_err(|e| format!("audio device unavailable: {e}"))?;
        let sink = rodio::Sink::try_new(&handle).map_err(|e| e.to_string())?;
        *state = Some(AudioThreadState {
            _stream: stream,
            handle,
            sink,
            current_track: None,
            loop_region: None,
            live_coeffs_tx: None,
            live_sample_rate: pcm.sample_rate,
            decoded_cache: None,
            landing_gain_cache: PreviewLandingCache::new(),
            peak_linear: Arc::new(AtomicU32::new(0)),
            gr_low: Arc::new(AtomicU32::new(0)),
            gr_mid: Arc::new(AtomicU32::new(0)),
            gr_high: Arc::new(AtomicU32::new(0)),
            lufs_x100: Arc::new(AtomicI32::new(i32::MIN)),
            integrated_lufs_x100: Arc::new(AtomicI32::new(i32::MIN)),
            spectrum_ring: Arc::new(SpectrumRing::new()),
            spectrum_analyzer: SpectrumAnalyzer::new(pcm.sample_rate),
        });
    }
    let s = state.as_mut().expect("state just inserted");
    s.sink.stop();
    s.spectrum_analyzer.reset();

    s.decoded_cache = Some(DecodedCacheEntry {
        canonical_path: canonical,
        mtime,
        pcm: pcm.clone(),
    });

    s.peak_linear.store(0, Ordering::Relaxed);
    s.gr_low.store(0, Ordering::Relaxed);
    s.gr_mid.store(0, Ordering::Relaxed);
    s.gr_high.store(0, Ordering::Relaxed);
    s.lufs_x100.store(i32::MIN, Ordering::Relaxed);
    s.integrated_lufs_x100.store(i32::MIN, Ordering::Relaxed);
    s.spectrum_analyzer = SpectrumAnalyzer::new(pcm.sample_rate);

    let sample_rate = pcm.sample_rate;
    let source = MeteredPcmSource::new(
        pcm.samples,
        pcm.channels,
        sample_rate,
        s.peak_linear.clone(),
        s.lufs_x100.clone(),
        s.integrated_lufs_x100.clone(),
        s.spectrum_ring.clone(),
    );

    let new_sink = rodio::Sink::try_new(&s.handle).map_err(|e| e.to_string())?;
    new_sink.append(source);
    if start_position_sec > 0.0 {
        // Best-effort seek; for some formats this can fail and we fall back to start.
        let _ = new_sink.try_seek(Duration::from_secs_f64(start_position_sec));
    }
    new_sink.play();
    s.sink = new_sink;
    s.current_track = Some(track_id);
    s.live_coeffs_tx = None;
    s.live_sample_rate = sample_rate;
    Ok(())
}

fn handle_play_master(
    state: &mut Option<AudioThreadState>,
    track_id: TrackId,
    path: &Path,
    settings: &MasteringSettings,
    start_position_sec: f64,
    preview_lufs_landing: bool,
) -> Result<(), String> {
    // Phase 12.1 perf — decode cache. Resolve the canonical path and
    // mtime to use as the cache key. If the cache holds a matching entry,
    // reuse the PCM directly; otherwise decode and store. Skipping
    // `decode_full` on a ~244 s WAV cuts the Original→Mastered toggle
    // latency from ~1–2 s down to a sub-100 ms swap.
    let canonical = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());
    let mtime = std::fs::metadata(&canonical).ok().and_then(|m| m.modified().ok());
    let cache_hit_pcm: Option<DecodedPcm> = decode_cache_lookup(
        state.as_ref().and_then(|s| s.decoded_cache.as_ref()),
        &canonical,
        mtime,
    );

    let pcm = match cache_hit_pcm {
        Some(p) => p,
        None => {
            let decoded =
                crate::audio::decode_full(path).map_err(|e| format!("{e}"))?;
            if decoded.samples.is_empty() {
                return Err("no samples decoded for master playback".to_string());
            }
            decoded
        }
    };

    // Cache invalidation: clear the landing-gain cache when the
    // canonical path differs from the prior decoded cache entry's.
    // Same-path replays (Original/Mastered toggle, repeated play on
    // the same track) preserve the cache because the PCM is
    // unchanged. New track → entries were computed against the OLD
    // PCM and would mis-land the new one.
    let path_changed = state
        .as_ref()
        .and_then(|s| s.decoded_cache.as_ref())
        .map_or(true, |entry| entry.canonical_path != canonical);
    if path_changed {
        if let Some(s) = state.as_mut() {
            s.landing_gain_cache.clear();
        }
    }

    if state.is_none() {
        let (stream, handle) = rodio::OutputStream::try_default()
            .map_err(|e| format!("audio device unavailable: {e}"))?;
        let sink = rodio::Sink::try_new(&handle).map_err(|e| e.to_string())?;
        *state = Some(AudioThreadState {
            _stream: stream,
            handle,
            sink,
            current_track: None,
            loop_region: None,
            live_coeffs_tx: None,
            live_sample_rate: pcm.sample_rate,
            decoded_cache: None,
            landing_gain_cache: PreviewLandingCache::new(),
            peak_linear: Arc::new(AtomicU32::new(0)),
            gr_low: Arc::new(AtomicU32::new(0)),
            gr_mid: Arc::new(AtomicU32::new(0)),
            gr_high: Arc::new(AtomicU32::new(0)),
            lufs_x100: Arc::new(AtomicI32::new(i32::MIN)),
            integrated_lufs_x100: Arc::new(AtomicI32::new(i32::MIN)),
            spectrum_ring: Arc::new(SpectrumRing::new()),
            spectrum_analyzer: SpectrumAnalyzer::new(pcm.sample_rate),
        });
    }
    let s = state.as_mut().expect("state just inserted");
    s.sink.stop();
    s.spectrum_analyzer.reset();

    // Update the cache (replace any prior entry — single-slot LRU is fine
    // for the typical "one or two fixtures" Track Master workflow).
    s.decoded_cache = Some(DecodedCacheEntry {
        canonical_path: canonical,
        mtime,
        pcm: pcm.clone(),
    });

    // Reset the peak slot so the meter starts fresh for this playback. Without
    // this, a swap from a prior session would leak its tail peak into the
    // first tick of the new one.
    s.peak_linear.store(0, Ordering::Relaxed);
    s.gr_low.store(0, Ordering::Relaxed);
    s.gr_mid.store(0, Ordering::Relaxed);
    s.gr_high.store(0, Ordering::Relaxed);
    s.lufs_x100.store(i32::MIN, Ordering::Relaxed);
    s.integrated_lufs_x100.store(i32::MIN, Ordering::Relaxed);

    let (coeffs_tx, coeffs_rx) = mpsc::channel::<crate::dsp::ChainCoeffs>();
    let gr_slots = crate::dsp::GrSnapshotSlots {
        low: s.gr_low.clone(),
        mid: s.gr_mid.clone(),
        high: s.gr_high.clone(),
    };
    let mut chain = crate::dsp::MasteringChain::new_with_gr_snapshots(
        pcm.sample_rate,
        pcm.channels as usize,
        settings,
        gr_slots,
    );
    if preview_lufs_landing {
        chain.coeffs.export_landing_gain_lin = export_landing_gain_lin_for_preview(
            &pcm.samples,
            pcm.sample_rate,
            pcm.channels,
            settings,
        )?;
    }
    let mastering_source = MasteringSource::new(
        pcm.samples,
        pcm.channels,
        pcm.sample_rate,
        chain,
        coeffs_rx,
        s.peak_linear.clone(),
        s.lufs_x100.clone(),
        s.integrated_lufs_x100.clone(),
        s.spectrum_ring.clone(),
    );

    let new_sink = rodio::Sink::try_new(&s.handle).map_err(|e| e.to_string())?;
    new_sink.append(mastering_source);
    if start_position_sec > 0.0 {
        let _ = new_sink.try_seek(Duration::from_secs_f64(start_position_sec));
    }
    new_sink.play();
    s.sink = new_sink;
    s.current_track = Some(track_id);
    s.live_coeffs_tx = Some(coeffs_tx);
    s.live_sample_rate = pcm.sample_rate;
    Ok(())
}

// ============================================================================
// MasteringSource — a rodio Source that streams interleaved PCM through the
// DSP chain. Coefficient updates flow in via mpsc; samples picked up at most
// every COEFFS_CHECK_INTERVAL samples (~6 ms at 44.1 kHz / 2ch).
// ============================================================================

/// How many frames to process before draining the coefficient channel. At
/// 44.1 kHz this is ~3 ms — well below the perception threshold for parameter
/// changes.
const COEFFS_CHECK_INTERVAL_FRAMES: usize = 128;
/// Crossfade length between old and new chain when coefficients change.
/// 512 frames ≈ 12 ms at 44.1 kHz. Long enough to mask filter-state transients
/// on preset/intensity changes; short enough to feel instantaneous.
const COEFFS_CROSSFADE_FRAMES: usize = 512;

/// Pass-through source for Original playback that still feeds the same peak,
/// LUFS, and spectrum meter path as Mastered playback. This keeps A/B metering
/// honest without routing Original through any mastering DSP.
struct MeteredPcmSource {
    samples: Vec<f32>,
    position: usize,
    channels: u16,
    sample_rate: u32,
    frame: Vec<f32>,
    frame_out_pos: usize,
    peak_linear: Arc<AtomicU32>,
    lufs_meter: crate::dsp::MomentaryLufs,
    lufs_x100: Arc<AtomicI32>,
    integrated_lufs_meter: crate::dsp::IntegratedLufs,
    integrated_lufs_x100: Arc<AtomicI32>,
    spectrum_ring: Arc<SpectrumRing>,
}

impl MeteredPcmSource {
    fn new(
        samples: Vec<f32>,
        channels: u16,
        sample_rate: u32,
        peak_linear: Arc<AtomicU32>,
        lufs_x100: Arc<AtomicI32>,
        integrated_lufs_x100: Arc<AtomicI32>,
        spectrum_ring: Arc<SpectrumRing>,
    ) -> Self {
        let channels_usize = channels.max(1) as usize;
        Self {
            samples,
            position: 0,
            channels,
            sample_rate,
            frame: vec![0.0; channels_usize],
            frame_out_pos: channels_usize,
            peak_linear,
            lufs_meter: crate::dsp::MomentaryLufs::new(sample_rate),
            lufs_x100,
            integrated_lufs_meter: crate::dsp::IntegratedLufs::new(sample_rate),
            integrated_lufs_x100,
            spectrum_ring,
        }
    }
}

impl Iterator for MeteredPcmSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let channels = self.channels.max(1) as usize;
        if self.frame_out_pos >= channels {
            if self.position >= self.samples.len() {
                return None;
            }

            for i in 0..channels {
                self.frame[i] = if self.position + i < self.samples.len() {
                    self.samples[self.position + i]
                } else {
                    0.0
                };
            }
            self.position += channels;

            let mut frame_peak = 0.0f32;
            for v in &self.frame[..channels] {
                let abs = v.abs();
                if abs.is_finite() && abs > frame_peak {
                    frame_peak = abs;
                }
            }
            self.peak_linear
                .fetch_max(frame_peak.to_bits(), Ordering::Relaxed);

            let l = self.frame.first().copied().unwrap_or(0.0);
            let r = if channels >= 2 { self.frame[1] } else { l };
            let to_x100 = |lufs: f32| -> i32 {
                if lufs.is_finite() && lufs > -120.0 {
                    (lufs * 100.0) as i32
                } else {
                    i32::MIN
                }
            };
            let momentary = self.lufs_meter.process_frame(l, r);
            self.lufs_x100.store(to_x100(momentary), Ordering::Relaxed);
            let integrated = self.integrated_lufs_meter.process_frame(l, r);
            self.integrated_lufs_x100
                .store(to_x100(integrated), Ordering::Relaxed);

            let mono = (l + r) * 0.5;
            if mono.is_finite() {
                self.spectrum_ring.push(mono);
            }

            self.frame_out_pos = 0;
        }

        let out = self.frame[self.frame_out_pos];
        self.frame_out_pos += 1;
        Some(out)
    }
}

impl rodio::Source for MeteredPcmSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels.max(1)
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let total_frames = self.samples.len() / self.channels.max(1) as usize;
        if self.sample_rate == 0 {
            None
        } else {
            Some(Duration::from_secs_f64(
                total_frames as f64 / self.sample_rate as f64,
            ))
        }
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        let channels = self.channels.max(1) as usize;
        let target_frame = (pos.as_secs_f64() * self.sample_rate as f64) as usize;
        let target_sample = target_frame.saturating_mul(channels);
        self.position = target_sample.min(self.samples.len());
        self.lufs_meter = crate::dsp::MomentaryLufs::new(self.sample_rate);
        self.integrated_lufs_meter = crate::dsp::IntegratedLufs::new(self.sample_rate);
        self.frame_out_pos = channels;
        Ok(())
    }
}

struct MasteringSource {
    samples: Vec<f32>,
    position: usize,
    channels: u16,
    sample_rate: u32,
    chain: crate::dsp::MasteringChain,
    pending_chain: Option<crate::dsp::MasteringChain>,
    crossfade_remaining: usize,
    crossfade_total: usize,
    coeffs_rx: mpsc::Receiver<crate::dsp::ChainCoeffs>,
    frames_since_check: usize,
    // Frame-level scratch buffers; preallocated to avoid heap traffic on the
    // audio thread.
    frame_in: Vec<f32>,
    frame_main: Vec<f32>,
    frame_pending: Vec<f32>,
    frame_out_pos: usize,
    /// Shared post-output-gain peak slot. Per-frame max of |frame_main[i]| is
    /// atomic-max'd into this slot. The audio thread consumes it via swap.
    peak_linear: Arc<AtomicU32>,
    /// Live BS.1770 momentary LUFS meter (K-weighted, 400 ms window).
    lufs_meter: crate::dsp::MomentaryLufs,
    /// Shared atomic slot for the audio thread to read the latest LUFS value.
    /// Stored as LUFS×100 in an i32. `i32::MIN` = silent / pre-prime.
    lufs_x100: Arc<AtomicI32>,
    /// BS.1770-4 integrated LUFS meter — aggregates the whole listen-through
    /// with absolute (-70 LUFS) and relative (-10 LU from ungated mean) gates.
    integrated_lufs_meter: crate::dsp::IntegratedLufs,
    /// Shared atomic slot for the integrated readout. Same storage convention
    /// as `lufs_x100`.
    integrated_lufs_x100: Arc<AtomicI32>,
    /// L4b — lock-free ring of post-chain mono mix samples. The audio
    /// thread pushes one sample per output frame; the snapshot tick
    /// reads it and runs an FFT to produce the EQ panel's live
    /// spectrum.
    spectrum_ring: Arc<SpectrumRing>,
}

impl MasteringSource {
    fn new(
        samples: Vec<f32>,
        channels: u16,
        sample_rate: u32,
        chain: crate::dsp::MasteringChain,
        coeffs_rx: mpsc::Receiver<crate::dsp::ChainCoeffs>,
        peak_linear: Arc<AtomicU32>,
        lufs_x100: Arc<AtomicI32>,
        integrated_lufs_x100: Arc<AtomicI32>,
        spectrum_ring: Arc<SpectrumRing>,
    ) -> Self {
        let channels_usize = channels.max(1) as usize;
        Self {
            samples,
            position: 0,
            channels,
            sample_rate,
            chain,
            pending_chain: None,
            crossfade_remaining: 0,
            crossfade_total: 0,
            coeffs_rx,
            frames_since_check: 0,
            frame_in: vec![0.0; channels_usize],
            frame_main: vec![0.0; channels_usize],
            frame_pending: vec![0.0; channels_usize],
            // Setting to `channels_usize` triggers the fetch on the first
            // `next()` call rather than requiring a separate "primed" flag.
            frame_out_pos: channels_usize,
            peak_linear,
            lufs_meter: crate::dsp::MomentaryLufs::new(sample_rate),
            lufs_x100,
            integrated_lufs_meter: crate::dsp::IntegratedLufs::new(sample_rate),
            integrated_lufs_x100,
            spectrum_ring,
        }
    }
}

impl Iterator for MasteringSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let channels = self.channels.max(1) as usize;
        if self.frame_out_pos >= channels {
            // Time to fetch + process the next input frame.
            if self.position >= self.samples.len() {
                return None;
            }

            // Pull one frame out of the source PCM. If we're short at the end
            // of the file, zero-pad — keeps the limiter happy.
            for i in 0..channels {
                self.frame_in[i] = if self.position + i < self.samples.len() {
                    self.samples[self.position + i]
                } else {
                    0.0
                };
            }
            self.position += channels;

            // Coefficient check / crossfade arming.
            self.frames_since_check += 1;
            if self.frames_since_check >= COEFFS_CHECK_INTERVAL_FRAMES {
                self.frames_since_check = 0;
                let mut latest: Option<crate::dsp::ChainCoeffs> = None;
                while let Ok(c) = self.coeffs_rx.try_recv() {
                    latest = Some(c);
                }
                if let Some(new_coeffs) = latest {
                    self.pending_chain =
                        Some(crate::dsp::MasteringChain::with_coeffs_inheriting_state(
                            new_coeffs,
                            &self.chain,
                        ));
                    self.crossfade_remaining = COEFFS_CROSSFADE_FRAMES;
                    self.crossfade_total = COEFFS_CROSSFADE_FRAMES;
                }
            }

            // Process the main chain into frame_main.
            for i in 0..channels {
                self.frame_main[i] = self.frame_in[i];
            }
            self.chain.process_frame_inplace(&mut self.frame_main[..channels]);

            // Process pending chain into frame_pending and mix.
            if self.pending_chain.is_some() && self.crossfade_total > 0 {
                for i in 0..channels {
                    self.frame_pending[i] = self.frame_in[i];
                }
                let pending = self
                    .pending_chain
                    .as_mut()
                    .expect("pending_chain just checked");
                pending.process_frame_inplace(&mut self.frame_pending[..channels]);
                let t = 1.0
                    - (self.crossfade_remaining as f32 / self.crossfade_total as f32);
                let inv_t = 1.0 - t;
                for i in 0..channels {
                    self.frame_main[i] =
                        self.frame_main[i] * inv_t + self.frame_pending[i] * t;
                }
                self.crossfade_remaining = self.crossfade_remaining.saturating_sub(1);
                if self.crossfade_remaining == 0 {
                    self.chain = self
                        .pending_chain
                        .take()
                        .expect("pending_chain just checked");
                    self.crossfade_total = 0;
                }
            }

            // Phase 12.2 — fold the post-output-gain frame peak into the shared
            // atomic for the live clipping meter. Per-frame instead of
            // per-sample: cheaper, and the meter only needs ~50 ms resolution
            // (the snapshot loop's tick rate). NaN/inf are filtered so a DSP
            // bug can't poison the atomic with a non-finite value.
            let mut frame_peak = 0.0f32;
            for i in 0..channels {
                let v = self.frame_main[i].abs();
                if v.is_finite() && v > frame_peak {
                    frame_peak = v;
                }
            }
            // Bits comparison is safe here because we only ever store
            // non-negative finite f32, where IEEE 754 ordering matches numeric.
            self.peak_linear
                .fetch_max(frame_peak.to_bits(), Ordering::Relaxed);

            // Live BS.1770 LUFS meters — feed the post-output stereo frame
            // into both the momentary (400 ms K-weighted window) and the
            // integrated (whole-listen-through with BS.1770-4 gating) meters.
            // Mono input gets duplicated so the meters see a stereo pair
            // (matches BS.1770's stereo channel summation).
            let l = self.frame_main.first().copied().unwrap_or(0.0);
            let r = if channels >= 2 { self.frame_main[1] } else { l };
            let to_x100 = |lufs: f32| -> i32 {
                if lufs.is_finite() && lufs > -120.0 {
                    (lufs * 100.0) as i32
                } else {
                    i32::MIN
                }
            };
            let momentary = self.lufs_meter.process_frame(l, r);
            self.lufs_x100.store(to_x100(momentary), Ordering::Relaxed);
            let integrated = self.integrated_lufs_meter.process_frame(l, r);
            self.integrated_lufs_x100
                .store(to_x100(integrated), Ordering::Relaxed);

            // L4b — push post-chain mono mix into the spectrum ring.
            // Lock-free atomic store; the snapshot tick FFTs the latest
            // 2048 samples to drive the EQ panel's live bars.
            let mono = (l + r) * 0.5;
            if mono.is_finite() {
                self.spectrum_ring.push(mono);
            }

            self.frame_out_pos = 0;
        }

        let out = self.frame_main[self.frame_out_pos];
        self.frame_out_pos += 1;
        Some(out)
    }
}

impl rodio::Source for MasteringSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels.max(1)
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let total_frames = self.samples.len() / self.channels.max(1) as usize;
        if self.sample_rate == 0 {
            None
        } else {
            Some(Duration::from_secs_f64(
                total_frames as f64 / self.sample_rate as f64,
            ))
        }
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        let channels = self.channels.max(1) as usize;
        let target_frame = (pos.as_secs_f64() * self.sample_rate as f64) as usize;
        let target_sample = target_frame.saturating_mul(channels);
        self.position = target_sample.min(self.samples.len());
        // Drop accumulated biquad/limiter state to avoid clicks across
        // discontinuities. Also force a frame re-fetch on the next yield.
        self.chain.reset_states();
        self.pending_chain = None;
        self.crossfade_remaining = 0;
        self.crossfade_total = 0;
        self.frame_out_pos = channels;
        Ok(())
    }
}

// ============================================================================
// Tests — MasteringSource live-update path (Phase 12.1: Dan caught this gap
// during manual smoke and rightly pointed out it should have been automated
// before now). MasteringSource is private to this module, so the test lives
// here instead of in tests/contracts.rs.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::{ChainCoeffs, MasteringChain};

    fn settings_with_intensity(intensity: f32) -> MasteringSettings {
        // Phase A4: with the preset compressor wired in (engaged by
        // default at density 0.5), the live-coeff RMS jump test would
        // see the compressor eat part of the input-gain delta when
        // intensity climbs. The test is grading the live-coeff plumbing,
        // not the compressor, so we explicitly bypass compression here
        // (density 0) to keep the RMS comparison clean.
        let mut advanced = AdvancedSettings::default();
        advanced.compression_density = Some(0.0);
        MasteringSettings {
            preset: Preset::Universal,
            intensity,
            eq_low_db: 0.0,
            eq_low_mid_db: 0.0,
            eq_mid_db: 0.0,
            eq_high_db: 0.0,
            volume_match: false,
            source_lufs_integrated: None,
            input_gain_db: 0.0,
            output_gain_db: 0.0,
            delivery_profile: DeliveryProfile::Custom,
            album: None,
            advanced,
        }
    }

    fn sine_signal(frames: usize, sample_rate: u32, channels: u16) -> Vec<f32> {
        let mut samples = Vec::with_capacity(frames * channels as usize);
        for n in 0..frames {
            let v = 0.3
                * (n as f32 / sample_rate as f32
                    * 2.0
                    * std::f32::consts::PI
                    * 1000.0)
                    .sin();
            for _ in 0..channels {
                samples.push(v);
            }
        }
        samples
    }

    fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }

    #[test]
    fn live_preview_coeffs_apply_export_lufs_landing_gain() {
        let sample_rate = 48_000;
        let channels: u16 = 2;
        let samples = sine_signal(sample_rate as usize * 2, sample_rate, channels);
        let mut settings = settings_with_intensity(1.0);
        settings.delivery_profile = DeliveryProfile::BroadcastEu;

        let coeffs = live_preview_coeffs(sample_rate, channels, &samples, &settings, true)
            .expect("preview coeffs");
        assert!(
            coeffs.export_landing_gain_lin < 1.0,
            "expected live export preview to trim a loud render, got {}",
            coeffs.export_landing_gain_lin
        );

        let mut rendered = samples.clone();
        let mut chain = MasteringChain::new(sample_rate, channels as usize, &settings);
        chain.coeffs.export_landing_gain_lin = coeffs.export_landing_gain_lin;
        chain.process_interleaved(&mut rendered, channels as usize);
        let measured = crate::engine::measure_integrated_lufs(&rendered, sample_rate, channels)
            .expect("measure preview");
        assert!(
            (measured - -23.0).abs() < 0.25,
            "expected preview near -23 LUFS export target, got {measured}"
        );

        let raw_coeffs = live_preview_coeffs(sample_rate, channels, &samples, &settings, false)
            .expect("raw preview coeffs");
        assert_eq!(raw_coeffs.export_landing_gain_lin, 1.0);
    }

    // ========================================================================
    // PreviewLandingCache — mechanical gates for the cache that prevents
    // repeat measurements when the user nudges back to a settings hash
    // they've already paid for. Together with command-loop coalescing,
    // this is the cost-floor enforcement layer for live-preview perf.
    // ========================================================================

    /// Identical settings hash to the same u64. Two callers that build
    /// the same MasteringSettings independently should land in the same
    /// cache slot — otherwise a knob-nudge-and-back doesn't reuse the
    /// prior result and the cache is doing nothing.
    #[test]
    fn settings_landing_hash_is_stable_across_identical_settings() {
        let a = settings_with_intensity(0.5);
        let b = settings_with_intensity(0.5);
        assert_eq!(
            settings_landing_hash(&a),
            settings_landing_hash(&b),
            "two structurally-identical settings must hash to the same value"
        );
    }

    /// Different intensity → different hash. Settings fields that affect
    /// chain output MUST bust the cache, otherwise stale gains would
    /// apply across legitimate edits.
    #[test]
    fn settings_landing_hash_differs_on_intensity_change() {
        let a = settings_with_intensity(0.5);
        let b = settings_with_intensity(0.6);
        assert_ne!(
            settings_landing_hash(&a),
            settings_landing_hash(&b),
            "intensity change must produce a different hash"
        );
    }

    /// VM toggle MUST NOT bust the cache. The measurement always runs
    /// with VM stripped (see `export_landing_gain_lin_for_preview`), so
    /// the landing gain is independent of VM state. If toggling VM
    /// busts the cache, every Volume Match click pays for a re-measure
    /// that returns the identical value.
    #[test]
    fn settings_landing_hash_ignores_volume_match_toggle() {
        let mut a = settings_with_intensity(0.5);
        a.volume_match = false;
        let mut b = a.clone();
        b.volume_match = true;
        assert_eq!(
            settings_landing_hash(&a),
            settings_landing_hash(&b),
            "VM toggle must not invalidate the cache — the measurement \
             always runs VM-stripped"
        );
    }

    /// Source LUFS injection MUST NOT bust the cache either. Analysis
    /// completes asynchronously and injects `source_lufs_integrated`
    /// into settings after the playback chain has already been
    /// built — busting the cache on that injection would force a
    /// re-measure of a landing gain that doesn't depend on source LUFS.
    #[test]
    fn settings_landing_hash_ignores_source_lufs_injection() {
        let mut a = settings_with_intensity(0.5);
        a.source_lufs_integrated = None;
        let mut b = a.clone();
        b.source_lufs_integrated = Some(-13.4);
        assert_eq!(
            settings_landing_hash(&a),
            settings_landing_hash(&b),
            "source LUFS injection must not invalidate the cache"
        );
    }

    /// Cache miss invokes `compute` and stores the result; cache hit
    /// returns the stored value without invoking `compute`. This is
    /// the core perf property: a repeat call on identical settings
    /// must skip the expensive measurement.
    #[test]
    fn landing_cache_skips_compute_on_repeat_settings() {
        let mut cache = PreviewLandingCache::new();
        let settings = settings_with_intensity(0.5);

        let mut compute_calls = 0usize;
        let result_a = cache.get_or_compute(&settings, |_| {
            compute_calls += 1;
            0.42_f32
        });
        assert_eq!(compute_calls, 1, "first call must run compute");
        assert!((result_a - 0.42).abs() < f32::EPSILON);

        // Second call with IDENTICAL settings → compute MUST NOT run.
        let result_b = cache.get_or_compute(&settings, |_| {
            compute_calls += 1;
            0.99_f32 // would-be result; should never be observed
        });
        assert_eq!(
            compute_calls, 1,
            "second call with identical settings must hit the cache (got {} compute invocations)",
            compute_calls
        );
        assert!(
            (result_b - 0.42).abs() < f32::EPSILON,
            "cache hit must return the stored value, not the new closure's result"
        );
    }

    /// Different settings hash → cache miss → compute runs again with
    /// a fresh result. Verifies the cache discriminates between
    /// settings, not just collapses everything to the first stored
    /// value.
    #[test]
    fn landing_cache_recomputes_on_different_settings() {
        let mut cache = PreviewLandingCache::new();
        let a = settings_with_intensity(0.5);
        let b = settings_with_intensity(0.6);
        let _ = cache.get_or_compute(&a, |_| 0.42_f32);
        let mut compute_calls = 0usize;
        let r = cache.get_or_compute(&b, |_| {
            compute_calls += 1;
            0.84_f32
        });
        assert_eq!(compute_calls, 1, "different settings must invoke compute");
        assert!((r - 0.84).abs() < f32::EPSILON);
        assert_eq!(cache.len(), 2, "cache must hold both entries");
    }

    /// `clear()` drops all entries — verifies the track-change
    /// invalidation path in `handle_play_master` actually wipes the
    /// cached values so the next UpdateChain re-measures against the
    /// new PCM.
    #[test]
    fn landing_cache_clear_drops_all_entries() {
        let mut cache = PreviewLandingCache::new();
        let _ = cache.get_or_compute(&settings_with_intensity(0.4), |_| 0.4_f32);
        let _ = cache.get_or_compute(&settings_with_intensity(0.5), |_| 0.5_f32);
        let _ = cache.get_or_compute(&settings_with_intensity(0.6), |_| 0.6_f32);
        assert_eq!(cache.len(), 3);
        cache.clear();
        assert_eq!(cache.len(), 0);

        // Post-clear, a previously-cached settings hash must invoke
        // compute again (the cache was honestly wiped, not just
        // marked dirty).
        let mut compute_calls = 0usize;
        let _ = cache.get_or_compute(&settings_with_intensity(0.5), |_| {
            compute_calls += 1;
            0.5_f32
        });
        assert_eq!(compute_calls, 1, "clear() must force the next call to re-compute");
    }

    // ========================================================================
    // Coalescing partition — mechanical gate for the "audio seek reply
    // timeout" fix. partition_for_coalescing() is the entire knob-spam
    // protection layer; if it ever stops dropping intermediate UpdateChains
    // or starts losing non-UpdateChain commands, Seeks will stall behind
    // expensive preview-LUFS measurements again. These tests are the
    // regression gate, no human listening required.
    // ========================================================================

    /// Empty input never panics and returns the canonical empty result.
    #[test]
    fn partition_handles_empty_buffer() {
        let (in_order, latest_update) = partition_for_coalescing(vec![]);
        assert!(in_order.is_empty());
        assert!(latest_update.is_none());
    }

    /// A single non-UpdateChain command passes through unmodified.
    #[test]
    fn partition_single_non_update_chain_passes_through() {
        let (in_order, latest_update) = partition_for_coalescing(vec![AudioCommand::Pause]);
        assert!(latest_update.is_none());
        assert_eq!(in_order.len(), 1);
        assert!(matches!(in_order[0], AudioCommand::Pause));
    }

    /// A single UpdateChain ends up as latest_update with no
    /// non-UpdateChain output. Verifies the trivial case isn't
    /// accidentally dropping the command.
    #[test]
    fn partition_single_update_chain_becomes_latest() {
        let buffered = vec![AudioCommand::UpdateChain {
            settings: settings_with_intensity(0.42),
            preview_lufs_landing: true,
        }];
        let (in_order, latest_update) = partition_for_coalescing(buffered);
        assert!(in_order.is_empty());
        let latest = latest_update.expect("single UpdateChain should land in latest_update");
        match latest {
            AudioCommand::UpdateChain { settings, preview_lufs_landing } => {
                assert!((settings.intensity - 0.42).abs() < 1e-6);
                assert!(preview_lufs_landing);
            }
            _ => panic!("latest_update should be the UpdateChain"),
        }
    }

    /// The core coalescing contract: intermediate UpdateChains are dropped,
    /// the LATEST UpdateChain payload wins, and non-UpdateChain commands
    /// retain submission order. This is exactly the queue shape that
    /// produced the "audio seek reply timeout" toast — knob-spam +
    /// interleaved seeks — and verifies the drained queue dispatches
    /// in the order [non-UpdateChain commands in submission order,
    /// latest UpdateChain].
    #[test]
    fn partition_drops_intermediate_update_chains_and_keeps_seeks_in_order() {
        // Simulate Dan's repro: rapid knob nudges (UpdateChain payloads
        // with intensity 0.1 -> 0.5 -> 0.9) interleaved with a Seek and
        // a Pause. Pre-fix, all three UpdateChains plus the Seek and
        // Pause would dispatch in submission order, with the Seek waiting
        // behind two expensive preview-LUFS measurements before its
        // reply went out and the frontend's 2 s timeout fired. Post-fix,
        // the Seek and Pause go through first (immediate reply), and
        // only the LATEST UpdateChain (intensity 0.9, preview_lufs_landing
        // = true) survives.
        let (seek_reply_tx, _seek_reply_rx) = mpsc::channel();
        let buffered = vec![
            AudioCommand::UpdateChain {
                settings: settings_with_intensity(0.1),
                preview_lufs_landing: false,
            },
            AudioCommand::Pause,
            AudioCommand::UpdateChain {
                settings: settings_with_intensity(0.5),
                preview_lufs_landing: false,
            },
            AudioCommand::Seek {
                position_sec: 42.0,
                reply: seek_reply_tx,
            },
            AudioCommand::UpdateChain {
                settings: settings_with_intensity(0.9),
                preview_lufs_landing: true,
            },
        ];
        let (in_order, latest_update) = partition_for_coalescing(buffered);

        // Non-UpdateChain commands kept in submission order.
        assert_eq!(in_order.len(), 2);
        assert!(matches!(in_order[0], AudioCommand::Pause));
        match &in_order[1] {
            AudioCommand::Seek { position_sec, .. } => {
                assert!((*position_sec - 42.0).abs() < 1e-9);
            }
            _ => panic!("expected Seek at position 1, got a different command variant"),
        }

        // Latest UpdateChain wins — both settings.intensity AND the
        // preview_lufs_landing flag are from the FINAL queued payload,
        // not the first one.
        let latest = latest_update.expect("latest UpdateChain must survive coalescing");
        match latest {
            AudioCommand::UpdateChain { settings, preview_lufs_landing } => {
                assert!(
                    (settings.intensity - 0.9).abs() < 1e-6,
                    "coalescing must keep the LATEST settings; got intensity {}",
                    settings.intensity
                );
                assert!(
                    preview_lufs_landing,
                    "coalescing must keep the LATEST preview_lufs_landing flag (should be true)"
                );
            }
            _ => panic!("latest_update should be the LATEST UpdateChain"),
        }
    }

    /// Many UpdateChains in a row → only the last survives, regardless of
    /// queue depth. Bounds the knob-spam case where 10+ UpdateChains
    /// accumulate between scheduler ticks.
    #[test]
    fn partition_collapses_long_run_of_update_chains_to_last() {
        let buffered: Vec<AudioCommand> = (0..20)
            .map(|i| AudioCommand::UpdateChain {
                settings: settings_with_intensity(i as f32 / 20.0),
                preview_lufs_landing: i % 2 == 0,
            })
            .collect();
        let (in_order, latest_update) = partition_for_coalescing(buffered);

        assert!(in_order.is_empty(), "no non-UpdateChain commands fed in");
        let latest = latest_update.expect("at least one UpdateChain in the queue");
        match latest {
            AudioCommand::UpdateChain { settings, preview_lufs_landing } => {
                // i=19 → intensity 19/20 = 0.95, preview_lufs_landing = false (odd).
                assert!((settings.intensity - 0.95).abs() < 1e-6);
                assert!(!preview_lufs_landing);
            }
            _ => panic!("latest_update must be the final UpdateChain"),
        }
    }

    /// 8 s preview window still produces accurate landing on sources
    /// LONGER than the window. The perf optimization measures only the
    /// middle 8 s of the decoded PCM rather than the full PCM, trusting
    /// that the BS.1770 gating over multiple 400 ms blocks is
    /// representative enough for landing-gain computation. This test
    /// fixes the gate: a 16 s source must still land within target
    /// tolerance, proving the windowing didn't break landing accuracy
    /// for longer material.
    #[test]
    fn live_preview_landing_accurate_on_source_longer_than_window() {
        let sample_rate = 48_000;
        let channels: u16 = 2;
        // 16 s source: double the 8 s preview window so the windowed
        // measurement reads only the middle half of the signal. If the
        // chain produced different LUFS in the first/last 4 s than the
        // middle 8 s, the landing gain would be off — this test asserts
        // the gain computed from the window still lands the FULL signal
        // on target.
        let samples = sine_signal((sample_rate as usize) * 16, sample_rate, channels);
        let mut settings = settings_with_intensity(1.0);
        settings.delivery_profile = DeliveryProfile::BroadcastEu;

        let coeffs = live_preview_coeffs(sample_rate, channels, &samples, &settings, true)
            .expect("preview coeffs on long source");
        assert!(
            coeffs.export_landing_gain_lin < 1.0,
            "loud 16 s signal targeting BroadcastEU -23 should attenuate; got {}",
            coeffs.export_landing_gain_lin
        );

        // Apply preview gain to a fresh FULL-LENGTH chain render and
        // confirm the resulting integrated LUFS lands on target. This is
        // the property that gates "is the windowed measurement
        // representative?" — if the answer is no, the full-signal LUFS
        // would drift away from -23 by more than the 0.5 dB tolerance.
        let mut rendered = samples.clone();
        let mut chain = MasteringChain::new(sample_rate, channels as usize, &settings);
        chain.coeffs.export_landing_gain_lin = coeffs.export_landing_gain_lin;
        chain.process_interleaved(&mut rendered, channels as usize);
        let measured =
            crate::engine::measure_integrated_lufs(&rendered, sample_rate, channels)
                .expect("measure long-source preview");
        assert!(
            (measured - -23.0).abs() < 0.5,
            "16 s source with 8 s window measurement should still land at \
             BroadcastEU target -23 LUFS within ±0.5 dB; got {measured:.2}"
        );
    }

    /// Ceiling-bounded LUFS landing: when the chain leaves headroom below
    /// the user's true-peak ceiling and the target is above the chain's
    /// natural output, the preview should push upward toward target — not
    /// refuse-upward like the prior policy did.
    #[test]
    fn live_preview_coeffs_push_upward_bounded_by_ceiling_headroom() {
        let sample_rate = 48_000;
        let channels: u16 = 2;
        // Quiet sine (~-30 dBFS): chain at intensity 0.5 + Universal won't
        // drive the limiter, so plenty of headroom below the ceiling for
        // an upward LUFS push.
        let mut samples = sine_signal(sample_rate as usize * 2, sample_rate, channels);
        for s in samples.iter_mut() {
            *s *= 0.1;
        }
        let mut settings = settings_with_intensity(0.5);
        settings.delivery_profile = DeliveryProfile::LoudRock; // -10.5 LUFS / -1 dBTP

        let coeffs = live_preview_coeffs(sample_rate, channels, &samples, &settings, true)
            .expect("preview coeffs");
        assert!(
            coeffs.export_landing_gain_lin > 1.0,
            "expected upward LUFS landing when chain leaves headroom below \
             ceiling, got gain {}",
            coeffs.export_landing_gain_lin
        );

        // Apply preview gain to a fresh chain render and confirm the result
        // lands at or near target without crossing the ceiling.
        let mut rendered = samples.clone();
        let mut chain = MasteringChain::new(sample_rate, channels as usize, &settings);
        chain.coeffs.export_landing_gain_lin = coeffs.export_landing_gain_lin;
        chain.process_interleaved(&mut rendered, channels as usize);
        let measured = crate::engine::measure_integrated_lufs(&rendered, sample_rate, channels)
            .expect("measure preview");
        assert!(
            (measured - -10.5).abs() < 0.5,
            "expected preview near LoudRock target -10.5 LUFS with full headroom, \
             got {measured:.2}"
        );
        let peak = rendered
            .iter()
            .map(|s| s.abs())
            .fold(0.0_f32, f32::max);
        let peak_dbfs = if peak > 0.0 { 20.0 * peak.log10() } else { -120.0 };
        assert!(
            peak_dbfs <= -1.0 + 0.5,
            "ceiling-bounded preview peak should stay near or below -1 dBTP, \
             got {peak_dbfs:.2} dBFS"
        );
    }

    /// Feed a 1 kHz sine through a MasteringSource, send a new ChainCoeffs
    /// through its mpsc channel mid-stream, and verify the output's RMS in
    /// the post-update region reflects the new chain (higher input gain).
    /// This is the test that *should* have caught the bug Dan reported during
    /// Phase 12.1 listening — it isolates the live-update path from the audio
    /// device, from frontend state, and from Tauri IPC.
    #[test]
    fn mastering_source_applies_live_coeff_updates_via_channel() {
        let sample_rate = 44_100;
        let channels: u16 = 2;
        let total_frames = 16_384; // ~371 ms, well past warmup + crossfade
        let samples = sine_signal(total_frames, sample_rate, channels);

        // Initial chain: intensity 0.0 -> Universal gain push ≈ 0.6 dB.
        let initial_settings = settings_with_intensity(0.0);
        let initial_chain =
            MasteringChain::new(sample_rate, channels as usize, &initial_settings);
        let (coeffs_tx, coeffs_rx) = mpsc::channel::<ChainCoeffs>();
        let peak = Arc::new(AtomicU32::new(0));
        let lufs = Arc::new(AtomicI32::new(i32::MIN));
        let integrated_lufs = Arc::new(AtomicI32::new(i32::MIN));
        let mut source = MasteringSource::new(
            samples,
            channels,
            sample_rate,
            initial_chain,
            coeffs_rx,
            peak,
            lufs,
            integrated_lufs,
            Arc::new(SpectrumRing::new()),
        );

        // Drain the first half at the initial chain.
        let half = total_frames * channels as usize / 2;
        let mut output_initial: Vec<f32> = Vec::with_capacity(half);
        for _ in 0..half {
            if let Some(s) = source.next() {
                output_initial.push(s);
            } else {
                break;
            }
        }

        // Send updated chain (intensity 1.0 -> Universal gain ≈ 2.4 dB).
        let new_settings = settings_with_intensity(1.0);
        let new_coeffs = ChainCoeffs::from_settings(sample_rate, &new_settings);
        coeffs_tx.send(new_coeffs).expect("send new coeffs");

        // Drain the second half. The coeff check fires every 128 frames; the
        // crossfade then takes 512 frames. The new chain is fully active by
        // ~640 frames after the send (≈ 1280 samples at stereo).
        let mut output_updated: Vec<f32> = Vec::with_capacity(half);
        for _ in 0..half {
            if let Some(s) = source.next() {
                output_updated.push(s);
            } else {
                break;
            }
        }

        // Compare the steady-state region of each half. Skip generously past
        // the crossfade and filter-warmup transients (4096 samples = ~46 ms).
        let warmup_skip = 4096;
        let steady_initial = &output_initial[warmup_skip..];
        let steady_updated = &output_updated[warmup_skip..];
        let rms_initial = rms(steady_initial);
        let rms_updated = rms(steady_updated);

        assert!(
            rms_initial > 0.0,
            "initial RMS should be non-zero (got {rms_initial})"
        );
        assert!(
            rms_updated > 0.0,
            "updated RMS should be non-zero (got {rms_updated})"
        );
        // Universal at intensity 0.0 -> preset_scale 0.4 -> gain_db 0.6 -> gain 1.071x.
        // Universal at intensity 1.0 -> preset_scale 1.6 -> gain_db 2.4 -> gain 1.318x.
        // Expected RMS ratio ≈ 1.23. Allow loose threshold but require a
        // clearly-audible jump.
        let ratio = rms_updated / rms_initial;
        assert!(
            ratio > 1.10,
            "expected live coeff update to raise output RMS by >10% \
             (rms_initial={rms_initial:.4}, rms_updated={rms_updated:.4}, ratio={ratio:.3}). \
             If this fails, the MasteringSource is not picking up new coeffs from the channel."
        );
    }

    #[test]
    fn metered_pcm_source_feeds_original_playback_meters_without_dsp() {
        let sample_rate = 44_100;
        let channels: u16 = 2;
        let total_frames = 24_000;
        let samples = sine_signal(total_frames, sample_rate, channels);
        let peak = Arc::new(AtomicU32::new(0));
        let lufs = Arc::new(AtomicI32::new(i32::MIN));
        let integrated_lufs = Arc::new(AtomicI32::new(i32::MIN));
        let spectrum = Arc::new(SpectrumRing::new());
        let mut source = MeteredPcmSource::new(
            samples.clone(),
            channels,
            sample_rate,
            peak.clone(),
            lufs.clone(),
            integrated_lufs.clone(),
            spectrum,
        );

        let mut rendered = Vec::with_capacity(samples.len());
        for _ in 0..samples.len() {
            rendered.push(source.next().expect("source sample"));
        }

        assert_eq!(rendered.len(), samples.len());
        assert!(
            rendered
                .iter()
                .zip(samples.iter())
                .all(|(a, b)| (*a - *b).abs() < 1.0e-7),
            "Original metered source must pass PCM through unchanged"
        );
        let peak_linear = f32::from_bits(peak.load(Ordering::Relaxed));
        assert!(peak_linear > 0.25, "expected source peak meter to update");
        assert_ne!(
            lufs.load(Ordering::Relaxed),
            i32::MIN,
            "expected source momentary LUFS to update"
        );
        assert_ne!(
            integrated_lufs.load(Ordering::Relaxed),
            i32::MIN,
            "expected source integrated LUFS to update"
        );
    }

    #[test]
    fn decode_cache_lookup_returns_pcm_on_path_and_mtime_match() {
        let pcm = DecodedPcm {
            samples: vec![0.1, 0.2, 0.3, 0.4],
            sample_rate: 44_100,
            channels: 2,
        };
        let mtime = Some(std::time::SystemTime::UNIX_EPOCH);
        let entry = DecodedCacheEntry {
            canonical_path: PathBuf::from("/fake/canonical/track.wav"),
            mtime,
            pcm: pcm.clone(),
        };
        let hit = decode_cache_lookup(
            Some(&entry),
            &PathBuf::from("/fake/canonical/track.wav"),
            mtime,
        );
        assert!(hit.is_some());
        let got = hit.unwrap();
        assert_eq!(got.samples, pcm.samples);
        assert_eq!(got.sample_rate, pcm.sample_rate);
        assert_eq!(got.channels, pcm.channels);
    }

    #[test]
    fn decode_cache_lookup_misses_on_path_mismatch() {
        let entry = DecodedCacheEntry {
            canonical_path: PathBuf::from("/fake/canonical/a.wav"),
            mtime: Some(std::time::SystemTime::UNIX_EPOCH),
            pcm: DecodedPcm {
                samples: vec![0.0],
                sample_rate: 44_100,
                channels: 1,
            },
        };
        let miss = decode_cache_lookup(
            Some(&entry),
            &PathBuf::from("/fake/canonical/b.wav"),
            Some(std::time::SystemTime::UNIX_EPOCH),
        );
        assert!(miss.is_none());
    }

    #[test]
    fn decode_cache_lookup_misses_on_mtime_mismatch() {
        let entry = DecodedCacheEntry {
            canonical_path: PathBuf::from("/fake/canonical/a.wav"),
            mtime: Some(std::time::SystemTime::UNIX_EPOCH),
            pcm: DecodedPcm {
                samples: vec![0.0],
                sample_rate: 44_100,
                channels: 1,
            },
        };
        // Same path, different mtime — file was modified, cache must invalidate.
        let later = std::time::SystemTime::UNIX_EPOCH
            + std::time::Duration::from_secs(60);
        let miss = decode_cache_lookup(
            Some(&entry),
            &PathBuf::from("/fake/canonical/a.wav"),
            Some(later),
        );
        assert!(miss.is_none());
    }

    #[test]
    fn decode_cache_lookup_misses_on_empty_cache() {
        let miss = decode_cache_lookup(
            None,
            &PathBuf::from("/fake/path.wav"),
            Some(std::time::SystemTime::UNIX_EPOCH),
        );
        assert!(miss.is_none());
    }

    /// A second, simpler test: confirm that *something* changed in the output
    /// after the channel send, by comparing the post-update buffer to a
    /// re-run of the same initial chain on the same input. If they're
    /// byte-identical (or near it) the live-update path is dead.
    #[test]
    fn mastering_source_output_differs_after_live_update() {
        let sample_rate = 44_100;
        let channels: u16 = 2;
        let total_frames = 8_192;
        let samples_a = sine_signal(total_frames, sample_rate, channels);
        let samples_b = sine_signal(total_frames, sample_rate, channels);

        let initial_settings = settings_with_intensity(0.0);

        // Reference run: never send new coeffs.
        let ref_chain =
            MasteringChain::new(sample_rate, channels as usize, &initial_settings);
        let (_ref_tx, ref_rx) = mpsc::channel::<ChainCoeffs>();
        let ref_peak = Arc::new(AtomicU32::new(0));
        let ref_lufs = Arc::new(AtomicI32::new(i32::MIN));
        let ref_integrated_lufs = Arc::new(AtomicI32::new(i32::MIN));
        let mut ref_source = MasteringSource::new(
            samples_a,
            channels,
            sample_rate,
            ref_chain,
            ref_rx,
            ref_peak,
            ref_lufs,
            ref_integrated_lufs,
            Arc::new(SpectrumRing::new()),
        );
        let ref_output: Vec<f32> = (0..total_frames * channels as usize)
            .filter_map(|_| ref_source.next())
            .collect();

        // Live-update run: send new coeffs halfway.
        let live_chain =
            MasteringChain::new(sample_rate, channels as usize, &initial_settings);
        let (live_tx, live_rx) = mpsc::channel::<ChainCoeffs>();
        let live_peak = Arc::new(AtomicU32::new(0));
        let live_lufs = Arc::new(AtomicI32::new(i32::MIN));
        let live_integrated_lufs = Arc::new(AtomicI32::new(i32::MIN));
        let mut live_source = MasteringSource::new(
            samples_b,
            channels,
            sample_rate,
            live_chain,
            live_rx,
            live_peak,
            live_lufs,
            live_integrated_lufs,
            Arc::new(SpectrumRing::new()),
        );
        let half = total_frames * channels as usize / 2;
        let mut live_output: Vec<f32> = Vec::with_capacity(total_frames * channels as usize);
        for _ in 0..half {
            if let Some(s) = live_source.next() {
                live_output.push(s);
            }
        }
        let new_settings = settings_with_intensity(1.0);
        let new_coeffs = ChainCoeffs::from_settings(sample_rate, &new_settings);
        live_tx.send(new_coeffs).expect("send new coeffs");
        for _ in half..(total_frames * channels as usize) {
            if let Some(s) = live_source.next() {
                live_output.push(s);
            }
        }

        // First half should be ~identical between reference and live runs
        // (both running the initial chain). Sanity check that fixture.
        let first_half_diff: f32 = ref_output[..half]
            .iter()
            .zip(live_output[..half].iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(
            first_half_diff < 1e-3,
            "first half should match between ref and live (diff={first_half_diff}); fixture broken"
        );

        // Second half must differ materially — the live run got new coeffs.
        let warmup_skip = 4096;
        let second_half_ref = &ref_output[half + warmup_skip..];
        let second_half_live = &live_output[half + warmup_skip..];
        let mean_abs_diff: f32 = second_half_ref
            .iter()
            .zip(second_half_live.iter())
            .map(|(a, b)| (a - b).abs())
            .sum::<f32>()
            / second_half_ref.len() as f32;
        assert!(
            mean_abs_diff > 0.005,
            "second-half output should diverge after live update \
             (mean_abs_diff={mean_abs_diff:.5}). If this is near 0, MasteringSource \
             is silently dropping coeff updates."
        );
    }

    /// Phase 12.2 — the post-output-gain peak atomic must reflect clipping
    /// when a chain pushes the signal above 0 dBFS. Output gain is post-
    /// limiter, so dialing it well above 0 dB is the simplest way to force a
    /// clip from inside a deterministic test.
    #[test]
    fn mastering_source_peak_atomic_reflects_clipping_above_ceiling() {
        let sample_rate = 44_100;
        let channels: u16 = 2;
        let total_frames = 4_096;
        let samples = sine_signal(total_frames, sample_rate, channels);

        // +20 dB post-limiter output gain on a 0.3-amplitude sine guarantees
        // the peak ends up well above 1.0 linear (≈ +9.5 dBFS in steady state).
        let mut settings = settings_with_intensity(0.0);
        settings.output_gain_db = 20.0;

        let chain = MasteringChain::new(sample_rate, channels as usize, &settings);
        let (_tx, rx) = mpsc::channel::<ChainCoeffs>();
        let peak = Arc::new(AtomicU32::new(0));
        let lufs = Arc::new(AtomicI32::new(i32::MIN));
        let integrated_lufs = Arc::new(AtomicI32::new(i32::MIN));
        let mut source = MasteringSource::new(
            samples,
            channels,
            sample_rate,
            chain,
            rx,
            peak.clone(),
            lufs,
            integrated_lufs,
            Arc::new(SpectrumRing::new()),
        );

        for _ in 0..(total_frames * channels as usize) {
            if source.next().is_none() {
                break;
            }
        }

        let peak_linear = f32::from_bits(peak.load(Ordering::Relaxed));
        assert!(
            peak_linear.is_finite(),
            "peak must be finite, got {peak_linear}"
        );
        assert!(
            peak_linear > 1.0,
            "expected post-output peak > 1.0 (above 0 dBFS) with +20 dB output gain, \
             got {peak_linear}. If this fails, MasteringSource is not folding peak \
             into the shared atomic, or the fold is reading pre-output-gain."
        );
    }

    /// Companion to the clipping test: a clean chain at neutral settings must
    /// produce a non-zero peak (audio is flowing) but must stay below 0 dBFS
    /// (limiter holds the line). Catches the failure mode where the peak fold
    /// is silently writing 0.
    #[test]
    fn mastering_source_peak_atomic_reflects_clean_signal_below_ceiling() {
        let sample_rate = 44_100;
        let channels: u16 = 2;
        let total_frames = 4_096;
        // Modest amplitude; Universal at intensity 0 adds ~0.6 dB gain.
        let samples = sine_signal(total_frames, sample_rate, channels);

        let settings = settings_with_intensity(0.0);
        let chain = MasteringChain::new(sample_rate, channels as usize, &settings);
        let (_tx, rx) = mpsc::channel::<ChainCoeffs>();
        let peak = Arc::new(AtomicU32::new(0));
        let lufs = Arc::new(AtomicI32::new(i32::MIN));
        let integrated_lufs = Arc::new(AtomicI32::new(i32::MIN));
        let mut source = MasteringSource::new(
            samples,
            channels,
            sample_rate,
            chain,
            rx,
            peak.clone(),
            lufs,
            integrated_lufs,
            Arc::new(SpectrumRing::new()),
        );

        for _ in 0..(total_frames * channels as usize) {
            if source.next().is_none() {
                break;
            }
        }

        let peak_linear = f32::from_bits(peak.load(Ordering::Relaxed));
        assert!(
            peak_linear > 0.0,
            "expected non-zero peak on real signal, got {peak_linear}. \
             The atomic was never written — peak fold is broken."
        );
        assert!(
            peak_linear < 1.0,
            "expected peak below 0 dBFS on clean signal through Universal/intensity-0, \
             got {peak_linear}. Either the limiter is misbehaving or the fold is reading \
             a non-final stage."
        );
    }

    /// Swap-and-reset semantics: after consuming a window of audio, swapping
    /// the atomic must return the peak from that window AND leave the slot at
    /// zero so a follow-up window starts fresh.
    #[test]
    fn mastering_source_peak_atomic_resets_on_swap() {
        let sample_rate = 44_100;
        let channels: u16 = 2;
        let total_frames = 2_048;
        let samples = sine_signal(total_frames, sample_rate, channels);

        let settings = settings_with_intensity(0.0);
        let chain = MasteringChain::new(sample_rate, channels as usize, &settings);
        let (_tx, rx) = mpsc::channel::<ChainCoeffs>();
        let peak = Arc::new(AtomicU32::new(0));
        let lufs = Arc::new(AtomicI32::new(i32::MIN));
        let integrated_lufs = Arc::new(AtomicI32::new(i32::MIN));
        let mut source = MasteringSource::new(
            samples,
            channels,
            sample_rate,
            chain,
            rx,
            peak.clone(),
            lufs,
            integrated_lufs,
            Arc::new(SpectrumRing::new()),
        );

        for _ in 0..(total_frames * channels as usize) {
            if source.next().is_none() {
                break;
            }
        }

        let first = f32::from_bits(peak.swap(0, Ordering::Relaxed));
        let second = f32::from_bits(peak.load(Ordering::Relaxed));

        assert!(first > 0.0, "first window saw signal, expected >0, got {first}");
        assert_eq!(
            second.to_bits(),
            0u32,
            "after swap, the slot must be exactly zero so the next window starts fresh \
             (got {second} = bits {:#x})",
            second.to_bits()
        );
    }

    /// Tiny conversion sanity: linear→dBFS should hand back the silence
    /// sentinel for zero input (JSON can't carry -inf cleanly).
    #[test]
    fn linear_to_dbfs_returns_silence_sentinel_for_zero() {
        assert_eq!(linear_to_dbfs(0.0), SILENCE_DBFS);
        // Spot-check a known dBFS landmark: 1.0 linear == 0 dBFS exactly.
        assert!((linear_to_dbfs(1.0) - 0.0).abs() < 1e-5);
        // -6 dBFS ≈ 0.5012 linear (within rounding).
        assert!((linear_to_dbfs(0.5) - (-6.020599)).abs() < 1e-4);
    }
}

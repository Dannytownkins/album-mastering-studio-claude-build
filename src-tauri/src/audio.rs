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

use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

const DEFAULT_TARGET_PIXELS: u32 = 1000;
const MIN_TARGET_PIXELS: u32 = 64;

#[tauri::command]
pub async fn prepare_source_playback(
    track_id: TrackId,
    track_path: String,
) -> CommandResult<PlaybackHandle> {
    let _ = track_path;
    Ok(handle(track_id, PlaybackKind::Source))
}

#[tauri::command]
pub async fn prepare_master_playback(
    track_id: TrackId,
    track_path: String,
    settings: MasteringSettings,
) -> CommandResult<PlaybackHandle> {
    let _ = (track_path, settings);
    Ok(handle(track_id, PlaybackKind::Master))
}

#[tauri::command]
pub async fn prepare_ab_preview(
    track_id: TrackId,
    track_path: String,
    settings: MasteringSettings,
    volume_match: bool,
) -> CommandResult<AbPreview> {
    let _ = (track_path, settings);
    let source_handle = handle(track_id.clone(), PlaybackKind::Source);
    let master_handle = handle(track_id.clone(), PlaybackKind::Master);
    Ok(AbPreview {
        track_id,
        source_handle,
        master_handle,
        volume_match_offset_db: if volume_match { -2.4 } else { 0.0 },
    })
}

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
    player.play_master(track_id, path, settings, start_position_sec.unwrap_or(0.0))
}

#[tauri::command]
pub async fn update_chain(
    settings: MasteringSettings,
    player: tauri::State<'_, Arc<AudioPlayer>>,
) -> CommandResult<()> {
    player.update_chain(settings)
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

fn handle(track_id: TrackId, kind: PlaybackKind) -> PlaybackHandle {
    PlaybackHandle {
        id: uuid::Uuid::new_v4().to_string(),
        track_id,
        kind,
        duration_seconds: 180.0,
    }
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
        reply: Sender<Result<(), String>>,
    },
    UpdateChain {
        settings: MasteringSettings,
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
    /// to `SILENCE_DBFS` on each new `prepare_master_playback`.
    pub lufs_integrated: f32,
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
    ) -> CommandResult<()> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send(AudioCommand::PlayMaster {
            track_id,
            path: path.to_path_buf(),
            settings,
            start_position_sec: start_position_sec.max(0.0),
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

    pub fn update_chain(&self, settings: MasteringSettings) -> CommandResult<()> {
        self.send(AudioCommand::UpdateChain { settings })
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

fn audio_thread(rx: mpsc::Receiver<AudioCommand>, snapshot: Arc<RwLock<PlaybackSnapshot>>) {
    let mut state: Option<AudioThreadState> = None;
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(AudioCommand::Play {
                track_id,
                path,
                start_position_sec,
                reply,
            }) => {
                let outcome = handle_play(&mut state, track_id, &path, start_position_sec);
                let _ = reply.send(outcome);
            }
            Ok(AudioCommand::PlayMaster {
                track_id,
                path,
                settings,
                start_position_sec,
                reply,
            }) => {
                let outcome =
                    handle_play_master(&mut state, track_id, &path, &settings, start_position_sec);
                let _ = reply.send(outcome);
            }
            Ok(AudioCommand::UpdateChain { settings }) => {
                if let Some(s) = state.as_ref() {
                    if let Some(tx) = s.live_coeffs_tx.as_ref() {
                        let coeffs =
                            crate::dsp::ChainCoeffs::from_settings(s.live_sample_rate, &settings);
                        let _ = tx.send(coeffs);
                    }
                }
            }
            Ok(AudioCommand::Pause) => {
                if let Some(s) = state.as_ref() {
                    s.sink.pause();
                }
            }
            Ok(AudioCommand::Resume) => {
                if let Some(s) = state.as_ref() {
                    s.sink.play();
                }
            }
            Ok(AudioCommand::Stop) => {
                if let Some(s) = state.as_mut() {
                    s.sink.stop();
                    s.current_track = None;
                }
            }
            Ok(AudioCommand::Seek { position_sec, reply }) => {
                let outcome = match state.as_ref() {
                    Some(s) => s
                        .sink
                        .try_seek(Duration::from_secs_f64(position_sec.max(0.0)))
                        .map_err(|e| e.to_string()),
                    None => Err("no track loaded".to_string()),
                };
                let _ = reply.send(outcome);
            }
            Ok(AudioCommand::SetLoop(region)) => {
                if let Some(s) = state.as_mut() {
                    s.loop_region = region.filter(|r| r.end_sec > r.start_sec);
                }
            }
            Ok(AudioCommand::Shutdown) => break,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
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

        let next_snap = match state.as_ref() {
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
                PlaybackSnapshot {
                    track_id: s.current_track.clone(),
                    position_sec: s.sink.get_pos().as_secs_f64(),
                    is_playing: !s.sink.is_paused() && !s.sink.empty(),
                    is_loaded: true,
                    peak_dbfs,
                    gr_low_db: to_gr_db(gr_u(&s.gr_low)),
                    gr_mid_db: to_gr_db(gr_u(&s.gr_mid)),
                    gr_high_db: to_gr_db(gr_u(&s.gr_high)),
                    lufs_momentary,
                    lufs_integrated,
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
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let reader = std::io::BufReader::new(file);
    let source = rodio::Decoder::new(reader).map_err(|e| e.to_string())?;

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
            live_sample_rate: 44_100,
            decoded_cache: None,
            peak_linear: Arc::new(AtomicU32::new(0)),
            gr_low: Arc::new(AtomicU32::new(0)),
            gr_mid: Arc::new(AtomicU32::new(0)),
            gr_high: Arc::new(AtomicU32::new(0)),
            lufs_x100: Arc::new(AtomicI32::new(i32::MIN)),
            integrated_lufs_x100: Arc::new(AtomicI32::new(i32::MIN)),
        });
    }
    let s = state.as_mut().expect("state just inserted");
    s.sink.stop();
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
    // Source playback bypasses MasteringSource — clear the peak slot so the
    // meter doesn't keep showing whatever the prior master playback left.
    s.peak_linear.store(0, Ordering::Relaxed);
    s.gr_low.store(0, Ordering::Relaxed);
    s.gr_mid.store(0, Ordering::Relaxed);
    s.gr_high.store(0, Ordering::Relaxed);
    s.lufs_x100.store(i32::MIN, Ordering::Relaxed);
    s.integrated_lufs_x100.store(i32::MIN, Ordering::Relaxed);
    Ok(())
}

fn handle_play_master(
    state: &mut Option<AudioThreadState>,
    track_id: TrackId,
    path: &Path,
    settings: &MasteringSettings,
    start_position_sec: f64,
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
            peak_linear: Arc::new(AtomicU32::new(0)),
            gr_low: Arc::new(AtomicU32::new(0)),
            gr_mid: Arc::new(AtomicU32::new(0)),
            gr_high: Arc::new(AtomicU32::new(0)),
            lufs_x100: Arc::new(AtomicI32::new(i32::MIN)),
            integrated_lufs_x100: Arc::new(AtomicI32::new(i32::MIN)),
        });
    }
    let s = state.as_mut().expect("state just inserted");
    s.sink.stop();

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
    let chain = crate::dsp::MasteringChain::new_with_gr_snapshots(
        pcm.sample_rate,
        pcm.channels as usize,
        settings,
        gr_slots,
    );
    let mastering_source = MasteringSource::new(
        pcm.samples,
        pcm.channels,
        pcm.sample_rate,
        chain,
        coeffs_rx,
        s.peak_linear.clone(),
        s.lufs_x100.clone(),
        s.integrated_lufs_x100.clone(),
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
        MasteringSettings {
            preset: Preset::Universal,
            intensity,
            eq_low_db: 0.0,
            eq_low_mid_db: 0.0,
            eq_mid_db: 0.0,
            eq_high_db: 0.0,
            volume_match: false,
            input_gain_db: 0.0,
            output_gain_db: 0.0,
            delivery_profile: DeliveryProfile::Custom,
            album: None,
            advanced: AdvancedSettings::default(),
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

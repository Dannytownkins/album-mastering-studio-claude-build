use crate::types::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

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

#[derive(Debug, Clone, Default)]
pub struct PlaybackSnapshot {
    pub track_id: Option<TrackId>,
    pub position_sec: f64,
    pub is_playing: bool,
    pub is_loaded: bool,
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
            Some(s) if s.current_track.is_some() => PlaybackSnapshot {
                track_id: s.current_track.clone(),
                position_sec: s.sink.get_pos().as_secs_f64(),
                is_playing: !s.sink.is_paused() && !s.sink.empty(),
                is_loaded: true,
            },
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
    Ok(())
}

fn handle_play_master(
    state: &mut Option<AudioThreadState>,
    track_id: TrackId,
    path: &Path,
    settings: &MasteringSettings,
    start_position_sec: f64,
) -> Result<(), String> {
    let pcm = crate::audio::decode_full(path).map_err(|e| format!("{e}"))?;
    if pcm.samples.is_empty() {
        return Err("no samples decoded for master playback".to_string());
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
        });
    }
    let s = state.as_mut().expect("state just inserted");
    s.sink.stop();

    let (coeffs_tx, coeffs_rx) = mpsc::channel::<crate::dsp::ChainCoeffs>();
    let chain = crate::dsp::MasteringChain::new(pcm.sample_rate, pcm.channels as usize, settings);
    let mastering_source = MasteringSource::new(pcm.samples, pcm.channels, pcm.sample_rate, chain, coeffs_rx);

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
}

impl MasteringSource {
    fn new(
        samples: Vec<f32>,
        channels: u16,
        sample_rate: u32,
        chain: crate::dsp::MasteringChain,
        coeffs_rx: mpsc::Receiver<crate::dsp::ChainCoeffs>,
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

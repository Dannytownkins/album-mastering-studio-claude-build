use crate::types::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

// ============================================================================
// Diagnostic counters — temporary instrumentation for the realtime-stutter
// remediation work (Fix A / B / C, 2026-05-20). Process-wide statics rather
// than per-player fields to keep the wiring touch-free; there's only ever
// one AudioPlayer instance. Frontend reads via `get_diag_counters`.
//
// Remove after the metrics have been validated on live material and the
// project is back to "no observable realtime issues" — track this with the
// next handoff cadence.
// ============================================================================

/// Snapshot returned by `get_diag_counters`. Plain u64 values (no atomics)
/// so the Tauri serializer is happy. All counts are cumulative since
/// process start.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiagCountersSnapshot {
    /// Times `UpdateChain` was dispatched on the audio command thread
    /// (post-coalescing — coalesced-away intermediate updates do NOT
    /// count). Compare against the TS-side updateChain attempts to see
    /// how aggressively the rAF gate is collapsing.
    pub update_chain_dispatched: u64,
    /// Times a `lufs-preview-landing` worker thread was spawned. Bounded
    /// by the single-in-flight gate plus the latest-pending follow-up
    /// drain — should track the number of distinct settings the user
    /// landed on with Preview LUFS enabled, NOT the number of cache
    /// misses.
    pub lufs_workers_spawned: u64,
    /// Times a worker returned and its `PreviewLandingReady` was applied
    /// (epoch + generation matched the current live state).
    pub lufs_workers_applied: u64,
    /// Times a worker returned and was cached but NOT applied to live
    /// output (epoch matched, generation had moved on). These are the
    /// "wasted but recovered" measurements — the cache catches them so
    /// a wiggle-back to the prior knob position is still a cache hit.
    pub lufs_workers_cached_only: u64,
    /// Times a worker's result was rejected wholesale because the
    /// track had changed since spawn. These would have poisoned the
    /// new track's landing-gain cache pre-Fix-C.
    pub lufs_workers_rejected_stale_epoch: u64,
    /// Times an `UpdateChain` arrived with Preview LUFS on + cache miss
    /// while a worker was already in flight — the latest-pending slot
    /// captured it for the drain on completion.
    pub lufs_workers_queued: u64,
    /// `MasteringSource` mid-fade coefficient promotions: a coefficient
    /// check fired, a new update arrived, AND a prior crossfade was
    /// already in progress. Pre-Fix-A this counter would stay at 0 (the
    /// crossfade simply got re-armed indefinitely); post-Fix-A it
    /// rises during sweeps as each in-progress fade closes out before
    /// the next opens.
    pub mid_fade_promotions: u64,
}

pub(crate) static UPDATE_CHAIN_DISPATCHED: AtomicU64 = AtomicU64::new(0);
pub(crate) static LUFS_WORKERS_SPAWNED: AtomicU64 = AtomicU64::new(0);
pub(crate) static LUFS_WORKERS_APPLIED: AtomicU64 = AtomicU64::new(0);
pub(crate) static LUFS_WORKERS_CACHED_ONLY: AtomicU64 = AtomicU64::new(0);
pub(crate) static LUFS_WORKERS_REJECTED_STALE_EPOCH: AtomicU64 = AtomicU64::new(0);
pub(crate) static LUFS_WORKERS_QUEUED: AtomicU64 = AtomicU64::new(0);
pub(crate) static MID_FADE_PROMOTIONS: AtomicU64 = AtomicU64::new(0);

fn snapshot_diag_counters() -> DiagCountersSnapshot {
    DiagCountersSnapshot {
        update_chain_dispatched: UPDATE_CHAIN_DISPATCHED.load(Ordering::Relaxed),
        lufs_workers_spawned: LUFS_WORKERS_SPAWNED.load(Ordering::Relaxed),
        lufs_workers_applied: LUFS_WORKERS_APPLIED.load(Ordering::Relaxed),
        lufs_workers_cached_only: LUFS_WORKERS_CACHED_ONLY.load(Ordering::Relaxed),
        lufs_workers_rejected_stale_epoch: LUFS_WORKERS_REJECTED_STALE_EPOCH
            .load(Ordering::Relaxed),
        lufs_workers_queued: LUFS_WORKERS_QUEUED.load(Ordering::Relaxed),
        mid_fade_promotions: MID_FADE_PROMOTIONS.load(Ordering::Relaxed),
    }
}

#[tauri::command]
pub async fn get_diag_counters() -> CommandResult<DiagCountersSnapshot> {
    Ok(snapshot_diag_counters())
}

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

use crate::decode::{decode_full, decode_to_peaks, DecodedPcm};
use crate::sources::{LiveCoeffUpdate, MasteringSource, MeteredPcmSource};
use crate::spectrum::{SpectrumAnalyzer, SpectrumRing};

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

/// Prewarm the decode cache for `track_path` in the background.
/// Intended to be called from the frontend the moment the user
/// selects a track in the UI — by the time they click Play / Mastered,
/// the PCM is already in the shared prewarm cache and
/// `handle_play_master` skips the synchronous `decode_full` call.
/// Eliminates the ~1-2 s freeze on first Mastered click for long
/// WAVs (Codex audit May 15, item 7).
///
/// Idempotent: re-prewarming the same `(canonical_path, mtime)`
/// returns immediately without decoding. The decode runs on
/// `tokio::task::spawn_blocking` so the Tauri async worker isn't
/// held for the decode duration and the audio thread is never
/// touched — playback / knob tweaks remain responsive during a
/// prewarm in flight.
#[tauri::command]
pub async fn prewarm_decode(
    track_path: String,
    player: tauri::State<'_, Arc<AudioPlayer>>,
) -> CommandResult<()> {
    if track_path.is_empty() {
        return Err(CommandError::InvalidPath("empty path".to_string()));
    }
    let path = std::path::PathBuf::from(&track_path);
    if crate::files::has_parent_dir_component(&path) {
        return Err(CommandError::InvalidPath(format!(
            "path traversal not allowed: {track_path}"
        )));
    }

    let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
    let mtime = std::fs::metadata(&canonical)
        .ok()
        .and_then(|m| m.modified().ok());

    // Declare this canonical path as the currently-requested prewarm
    // target. After the decode below finishes, the same call will
    // check the target still matches before writing — so a slow
    // prewarm that resolves AFTER a newer selection silently drops
    // its result instead of evicting the newer cache entry.
    player.set_prewarm_target(canonical.clone());

    // Idempotency guard: if the prewarm cache already holds this
    // entry, skip the decode entirely. The frontend can safely call
    // prewarm_decode on every track-select without thrashing.
    if player.prewarm_cache_hit(&canonical, mtime) {
        return Ok(());
    }

    // Decode on the blocking pool so the async Tauri worker thread
    // isn't held for ~1-2 s. The PCM lands in the shared cache; the
    // audio thread reads from there on the next play_master.
    // `tauri::async_runtime::spawn_blocking` delegates to the same
    // tokio runtime Tauri runs its commands on, no extra crate
    // dependency.
    let decode_path = path.clone();
    let join_result = tauri::async_runtime::spawn_blocking(move || decode_full(&decode_path))
        .await
        .map_err(|e| CommandError::Other(format!("prewarm decode task: {e}")))?;
    let decoded = join_result?;
    if decoded.samples.is_empty() {
        return Err(CommandError::Decode(format!(
            "no samples decoded from {track_path}"
        )));
    }

    // Stale-prewarm guard: if a newer selection set a different
    // target while this decode was running, drop the result rather
    // than evict the newer cache entry. The race that motivates
    // this: user selects A (slow), 200 ms later selects B (fast);
    // B's prewarm finishes and writes B; A's prewarm finishes and
    // would overwrite B with A. Without the guard, the user clicks
    // Mastered on B and pays a cold decode again.
    if !player.prewarm_target_matches(&canonical) {
        return Ok(());
    }

    player.set_prewarm_cache(DecodedCacheEntry {
        canonical_path: canonical,
        mtime,
        pcm: decoded,
    });
    Ok(())
}

#[tauri::command]
pub async fn pause_playback(player: tauri::State<'_, Arc<AudioPlayer>>) -> CommandResult<()> {
    player.pause();
    Ok(())
}

#[tauri::command]
pub async fn resume_playback(player: tauri::State<'_, Arc<AudioPlayer>>) -> CommandResult<()> {
    player.resume();
    Ok(())
}

#[tauri::command]
pub async fn stop_playback(player: tauri::State<'_, Arc<AudioPlayer>>) -> CommandResult<()> {
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
            return Err(CommandError::Other(
                "loop region must be finite".to_string(),
            ));
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
    PreviewLandingReady {
        /// Captured at worker spawn time. Rejected by the audio thread if
        /// the current track epoch has moved on — guards the landing-gain
        /// cache against poisoning from a worker that started against a
        /// prior track's PCM.
        track_epoch: u64,
        generation: u64,
        settings: MasteringSettings,
        gain: f32,
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

/// Single-slot opportunistic decode cache shared between the audio
/// thread and Tauri prewarm commands. Distinct from
/// `AudioThreadState.decoded_cache` (which represents the
/// currently-playing PCM and is consulted by `UpdateChain` for live-
/// preview measurements). Splitting the two:
///
///   * UpdateChain never reads this shared cache — it always uses the
///     currently-playing PCM. Prewarm of a different track can't
///     poison live-preview measurements.
///   * `handle_play_master` consults both: local first (zero-cost
///     same-track replay), then this shared cache (prewarm hit), then
///     fresh decode (cold miss).
///   * `prewarm_decode` writes here from off-thread, so a 1–2 s
///     decode can run while the user is still browsing tracks. By the
///     time they click Mastered, the PCM is already in the slot.
type SharedDecodedCache = Arc<Mutex<Option<DecodedCacheEntry>>>;

pub struct AudioPlayer {
    tx: Mutex<Option<Sender<AudioCommand>>>,
    snapshot: Arc<RwLock<PlaybackSnapshot>>,
    /// Shared opportunistic decode cache. Populated by `prewarm_decode`
    /// and read (with fallback to fresh decode) by `handle_play_master`
    /// / `handle_play`. Wrapped in a Mutex rather than RwLock because
    /// reads are paired with writes (cache miss → decode → write), so
    /// the lock contention is negligible.
    prewarm_cache: SharedDecodedCache,
    /// Most-recently-requested prewarm target (canonical path).
    /// Every `prewarm_decode` call sets this at the start; after its
    /// decode finishes, the same call checks the target still matches
    /// before writing to `prewarm_cache`. Without this guard, a SLOW
    /// prewarm could finish AFTER a NEWER selection's prewarm and
    /// evict the newer entry from the single-slot cache (Codex review).
    /// Stale prewarms drop their result silently.
    prewarm_target: Arc<Mutex<Option<PathBuf>>>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let snapshot = Arc::new(RwLock::new(PlaybackSnapshot::default()));
        let prewarm_cache: SharedDecodedCache = Arc::new(Mutex::new(None));
        let prewarm_target: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
        let (tx, rx) = mpsc::channel::<AudioCommand>();
        let snap_for_thread = snapshot.clone();
        let prewarm_for_thread = prewarm_cache.clone();
        let tx_for_thread = tx.clone();
        std::thread::Builder::new()
            .name("audio-player".to_string())
            .spawn(move || audio_thread(rx, tx_for_thread, snap_for_thread, prewarm_for_thread))
            .expect("spawn audio thread");
        Self {
            tx: Mutex::new(Some(tx)),
            snapshot,
            prewarm_cache,
            prewarm_target,
        }
    }

    /// Declare the canonical path this player should consider the
    /// "currently-requested" prewarm target. Called at the start of
    /// `prewarm_decode` so a slow decode can check whether its target
    /// has been superseded before it commits a stale entry to the
    /// shared cache.
    pub(crate) fn set_prewarm_target(&self, canonical: PathBuf) {
        let mut guard = self
            .prewarm_target
            .lock()
            .expect("prewarm target mutex poisoned");
        *guard = Some(canonical);
    }

    /// Returns true when `canonical` matches the most-recently-set
    /// prewarm target. False when no target has been set or the
    /// target has moved on to a different path. Called by
    /// `prewarm_decode` after its decode finishes to decide whether
    /// to write to the cache or drop the stale result.
    pub(crate) fn prewarm_target_matches(&self, canonical: &Path) -> bool {
        let guard = self
            .prewarm_target
            .lock()
            .expect("prewarm target mutex poisoned");
        match guard.as_ref() {
            Some(target) => target == canonical,
            None => false,
        }
    }

    /// Returns true when the prewarm cache already holds an entry
    /// matching `(canonical_path, mtime)`. Used by `prewarm_decode`
    /// to skip redundant decodes when the user re-selects the same
    /// track in the UI.
    pub(crate) fn prewarm_cache_hit(
        &self,
        canonical_path: &Path,
        mtime: Option<std::time::SystemTime>,
    ) -> bool {
        let guard = self
            .prewarm_cache
            .lock()
            .expect("prewarm cache mutex poisoned");
        match guard.as_ref() {
            Some(entry) => entry.canonical_path == canonical_path && entry.mtime == mtime,
            None => false,
        }
    }

    /// Replace the prewarm cache entry. Single-slot LRU: any prior
    /// entry is dropped. Called by `prewarm_decode` after a successful
    /// off-thread decode.
    pub(crate) fn set_prewarm_cache(&self, entry: DecodedCacheEntry) {
        let mut guard = self
            .prewarm_cache
            .lock()
            .expect("prewarm cache mutex poisoned");
        *guard = Some(entry);
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
            Err(_) => Err(CommandError::Other("audio seek reply timeout".to_string())),
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
        let guard = self
            .tx
            .lock()
            .map_err(|_| "audio tx mutex poisoned".to_string())?;
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
    live_coeffs_tx: Option<Sender<LiveCoeffUpdate>>,
    live_coeff_generation: u64,
    live_landing_gain_lin: f32,
    live_sample_rate: u32,
    /// Bumped on every `play` / `play_master`. Captured by LUFS preview
    /// workers at spawn time and re-checked on `PreviewLandingReady`. A
    /// worker that started against a prior track's PCM lands its result
    /// after the cache is cleared at the track boundary; without the
    /// epoch check it would re-insert stale-track gain under the
    /// settings hash and poison the new track's preview.
    track_epoch: u64,
    /// True while a `lufs-preview-landing` worker is alive. Caps the
    /// in-flight measurement count to one and prevents an OS-thread
    /// spawn flood under fast knob sweeps with Preview LUFS on.
    lufs_worker_in_flight: bool,
    /// Most-recent (settings, generation) that wanted a measurement
    /// while a worker was already in flight. The audio thread drains
    /// this when the active worker reports back and spawns the
    /// follow-up measurement — latest-pending semantics, so an
    /// arbitrary stream of cache-miss updates costs at most one
    /// active worker plus one queued worker at any time.
    lufs_worker_pending: Option<(MasteringSettings, u64)>,
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
pub(crate) struct DecodedCacheEntry {
    pub(crate) canonical_path: PathBuf,
    pub(crate) mtime: Option<std::time::SystemTime>,
    pub(crate) pcm: DecodedPcm,
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
    #[cfg(test)]
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

    fn get(&self, settings: &MasteringSettings) -> Option<f32> {
        let hash = settings_landing_hash(settings);
        self.by_hash.get(&hash).copied()
    }

    fn insert(&mut self, settings: &MasteringSettings, gain: f32) {
        let hash = settings_landing_hash(settings);
        self.by_hash.insert(hash, gain);
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
    let window_frames = ((PREVIEW_WINDOW_SECS * sample_rate as f32) as usize).min(total_frames);
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
        let tp = ebu.true_peak(ch).map_err(|e| format!("ebur128 tp: {e}"))?;
        if tp > peak_lin {
            peak_lin = tp;
        }
    }
    let measured_true_peak_dbtp = if peak_lin > 0.0 {
        (20.0 * peak_lin.log10()) as f32
    } else {
        -60.0
    };

    // Route through the shared ceiling-bounded math (engine.rs) so the
    // live-preview path applies exactly the same delta as the offline
    // render paths — preview-to-export parity is the load-bearing
    // property here. The helper returns the applied delta in dB; the
    // preview path converts that to a linear gain scalar (rather than
    // mutating samples) because it ships through `ChainCoeffs` for the
    // live audio thread to apply per frame.
    let ceiling_dbtp = render_settings.effective_ceiling_dbtp();
    let applied_delta_db = crate::engine::ceiling_bounded_landing_delta_db(
        measured,
        measured_true_peak_dbtp,
        target_lufs,
        ceiling_dbtp,
    );
    if applied_delta_db != 0.0 {
        Ok(10.0_f32.powf(applied_delta_db / 20.0))
    } else {
        Ok(1.0)
    }
}

fn preview_landing_window(samples: &[f32], sample_rate: u32, channels: u16) -> Vec<f32> {
    const PREVIEW_WINDOW_SECS: f32 = 8.0;
    let channels_usize = channels.max(1) as usize;
    let total_frames = samples.len() / channels_usize;
    let window_frames = ((PREVIEW_WINDOW_SECS * sample_rate as f32) as usize).min(total_frames);
    let start_frame = total_frames.saturating_sub(window_frames) / 2;
    let start = start_frame * channels_usize;
    let end = ((start_frame + window_frames) * channels_usize).min(samples.len());
    samples[start..end].to_vec()
}

/// Spawn a `lufs-preview-landing` worker thread that measures the export
/// landing gain for `settings` against the cached decoded PCM and sends
/// the result back through `command_tx` as `PreviewLandingReady`. Returns
/// `true` if the worker was spawned (caller should then mark its
/// in-flight gate); `false` if there's no decoded PCM yet or the OS
/// rejected the thread spawn — in either case no worker is alive.
///
/// `track_epoch` is captured here at spawn time and echoed back in the
/// result so the audio thread can drop results from a worker that
/// outlived a track change.
fn try_spawn_lufs_preview_worker(
    decoded_cache: Option<&DecodedCacheEntry>,
    sample_rate: u32,
    settings: MasteringSettings,
    generation: u64,
    track_epoch: u64,
    command_tx: &Sender<AudioCommand>,
) -> bool {
    let Some(cache_entry) = decoded_cache else {
        return false;
    };
    let channels = cache_entry.pcm.channels;
    let samples = preview_landing_window(
        cache_entry.pcm.samples.as_slice(),
        sample_rate,
        channels,
    );
    let command_tx = command_tx.clone();
    let spawn_result = std::thread::Builder::new()
        .name("lufs-preview-landing".to_string())
        .spawn(move || {
            let gain = export_landing_gain_lin_for_preview(
                samples.as_slice(),
                sample_rate,
                channels,
                &settings,
            )
            .unwrap_or(1.0);
            let _ = command_tx.send(AudioCommand::PreviewLandingReady {
                track_epoch,
                generation,
                settings,
                gain,
            });
        });
    if spawn_result.is_ok() {
        LUFS_WORKERS_SPAWNED.fetch_add(1, Ordering::Relaxed);
        true
    } else {
        false
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

/// Resolve PCM for `path` via the three-tier cache hierarchy used by
/// `handle_play` / `handle_play_master`:
///
///   1. Local AudioThreadState `decoded_cache` (currently-playing PCM).
///      Matches the Original/Mastered toggle case where the user is
///      replaying the same track — zero cost.
///   2. Shared `prewarm_cache` populated off the audio thread by
///      `prewarm_decode` (Tauri command, runs on tokio's blocking
///      pool). Hits when the user selected the track in the UI and
///      prewarm finished before they clicked Play / Mastered.
///   3. Fresh `decode_full` — synchronous on the audio thread, the
///      cold case prewarm exists to avoid. 1-2 s for long WAVs.
///
/// Returns the resolved PCM or an error string suitable for forwarding
/// through the caller's `Result<(), String>` reply path.
fn resolve_pcm_with_caches(
    state: Option<&AudioThreadState>,
    prewarm_cache: &SharedDecodedCache,
    path: &Path,
    canonical: &Path,
    mtime: Option<std::time::SystemTime>,
) -> Result<DecodedPcm, String> {
    // Tier 1: local same-track cache (no lock, fastest path).
    if let Some(p) = decode_cache_lookup(
        state.and_then(|s| s.decoded_cache.as_ref()),
        canonical,
        mtime,
    ) {
        return Ok(p);
    }
    // Tier 2: shared prewarm cache. Briefly locked for the lookup; if
    // the lock is contended the prewarm task is mid-write and we fall
    // through to fresh decode (rare, acceptable — the next play_master
    // would hit the now-populated shared cache).
    {
        let guard = prewarm_cache
            .lock()
            .map_err(|e| format!("prewarm cache lock: {e}"))?;
        if let Some(p) = decode_cache_lookup(guard.as_ref(), canonical, mtime) {
            return Ok(p);
        }
    }
    // Tier 3: cold path. Decode synchronously.
    let decoded = crate::decode::decode_full(path).map_err(|e| format!("{e}"))?;
    if decoded.samples.is_empty() {
        return Err("no samples decoded for playback".to_string());
    }
    Ok(decoded)
}

/// True for commands that change which track is loaded into the audio
/// thread. UpdateChain coalescing MUST NOT cross these — a stale
/// UpdateChain queued before a track switch would otherwise apply old
/// settings to the new master once the coalescer reorders it after the
/// barrier (Codex review, the playback-boundary bug).
fn is_playback_barrier(cmd: &AudioCommand) -> bool {
    matches!(
        cmd,
        AudioCommand::Play { .. } | AudioCommand::PlayMaster { .. } | AudioCommand::Stop
    )
}

/// Coalesce a buffered batch of audio commands into the sequence the
/// audio thread should actually dispatch.
///
/// The audio command loop drains every command currently waiting on the
/// channel before processing, so a knob-spam burst typically arrives
/// as multiple UpdateChains alongside one or two latency-sensitive
/// commands (Seek / Pause / Resume). Inside each "segment" — a stretch
/// of buffered commands bounded by `is_playback_barrier` — the
/// coalescer keeps the LATEST UpdateChain and drops the older
/// intermediates (their payloads are stale; the user has already moved
/// past them). Non-UpdateChain commands retain their submission order
/// within the segment, with the surviving UpdateChain dispatched LAST
/// so live playback ends up reflecting the freshest settings.
///
/// **Playback-barrier semantic.** Play / PlayMaster / Stop split the
/// buffer into segments. UpdateChains never cross a barrier: a stale
/// pre-barrier UpdateChain applies to the OLD track (or harmlessly
/// drops into a soon-to-be-replaced live_coeffs_tx), and a fresh
/// post-barrier UpdateChain applies to the NEW track. Without this
/// split the reorder rule "non-UpdateChain first, latest UpdateChain
/// last" would carry stale pre-switch settings across the boundary and
/// apply them to the wrong track.
///
/// Pause / Resume / Seek / SetLoop are NOT barriers — they don't
/// change the loaded track, so UpdateChains can safely coalesce across
/// them.
///
/// Empty input yields an empty vec.
fn coalesced_command_sequence(buffered: Vec<AudioCommand>) -> Vec<AudioCommand> {
    let mut result: Vec<AudioCommand> = Vec::with_capacity(buffered.len());
    let mut segment_in_order: Vec<AudioCommand> = Vec::new();
    let mut segment_latest_update: Option<AudioCommand> = None;

    for cmd in buffered {
        if is_playback_barrier(&cmd) {
            // Flush current segment, then the barrier itself. The
            // barrier MUST stay between the segments to preserve
            // submission-time ordering relative to other commands.
            result.extend(segment_in_order.drain(..));
            if let Some(c) = segment_latest_update.take() {
                result.push(c);
            }
            result.push(cmd);
        } else if matches!(cmd, AudioCommand::UpdateChain { .. }) {
            segment_latest_update = Some(cmd);
        } else {
            segment_in_order.push(cmd);
        }
    }

    // Flush trailing segment (everything after the last barrier, or the
    // entire buffer if no barriers were present).
    result.extend(segment_in_order);
    if let Some(c) = segment_latest_update {
        result.push(c);
    }

    result
}

/// True when the landing-gain cache must be cleared before processing
/// a new play_master / play call. The cache is scoped to the
/// currently-loaded decoded PCM, so it goes stale on:
///   * canonical-path change (different track),
///   * mtime change (same path but the file was re-saved or replaced),
///   * no prior cache entry (first load — empty cache anyway, but
///     the predicate returns `true` so callers don't have to special-case).
fn should_invalidate_landing_cache(
    prior_entry: Option<&DecodedCacheEntry>,
    new_canonical: &Path,
    new_mtime: Option<std::time::SystemTime>,
) -> bool {
    match prior_entry {
        Some(entry) => entry.canonical_path != new_canonical || entry.mtime != new_mtime,
        None => true,
    }
}

/// Dispatch a single audio command. Returns `true` when Shutdown is
/// received so the caller can break the loop. Extracted from the
/// original inline match so the command loop can buffer + coalesce
/// queued commands before dispatching (see `audio_thread` for the
/// drain pattern).
fn process_audio_command(
    cmd: AudioCommand,
    state: &mut Option<AudioThreadState>,
    prewarm_cache: &SharedDecodedCache,
    command_tx: &Sender<AudioCommand>,
) -> bool {
    match cmd {
        AudioCommand::Play {
            track_id,
            path,
            start_position_sec,
            reply,
        } => {
            let outcome = handle_play(state, track_id, &path, start_position_sec, prewarm_cache);
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
                prewarm_cache,
            );
            let _ = reply.send(outcome);
        }
        AudioCommand::UpdateChain {
            settings,
            preview_lufs_landing,
        } => {
            UPDATE_CHAIN_DISPATCHED.fetch_add(1, Ordering::Relaxed);
            if let Some(s) = state.as_mut() {
                let sample_rate = s.live_sample_rate;
                let generation = s.live_coeff_generation.wrapping_add(1);
                s.live_coeff_generation = generation;

                if let Some(tx) = s.live_coeffs_tx.as_ref() {
                    let mut coeffs = crate::dsp::ChainCoeffs::from_settings(sample_rate, &settings);
                    if preview_lufs_landing {
                        if let Some(cached) = s.landing_gain_cache.get(&settings) {
                            coeffs.export_landing_gain_lin = cached;
                            s.live_landing_gain_lin = cached;
                        } else {
                            // Cache miss — apply coefficients now with the
                            // last-known landing scalar (audio keeps flowing)
                            // and schedule a background measurement. Single-
                            // in-flight: if a worker is already running, just
                            // record the latest (settings, generation) as
                            // pending; the active worker's completion handler
                            // drains it. Caps in-flight measurement work at
                            // one OS thread regardless of UpdateChain rate.
                            coeffs.export_landing_gain_lin = s.live_landing_gain_lin;
                            if s.lufs_worker_in_flight {
                                LUFS_WORKERS_QUEUED.fetch_add(1, Ordering::Relaxed);
                                s.lufs_worker_pending = Some((settings.clone(), generation));
                            } else if try_spawn_lufs_preview_worker(
                                s.decoded_cache.as_ref(),
                                sample_rate,
                                settings.clone(),
                                generation,
                                s.track_epoch,
                                command_tx,
                            ) {
                                s.lufs_worker_in_flight = true;
                            }
                        }
                        // No decoded PCM cached yet → leave landing
                        // gain at 1.0. The next play_master will
                        // populate the decode cache and the next
                        // UpdateChain will compute through the cache.
                    }
                    let _ = tx.send(LiveCoeffUpdate { generation, coeffs });
                }
            }
        }
        AudioCommand::PreviewLandingReady {
            track_epoch,
            generation,
            settings,
            gain,
        } => {
            if let Some(s) = state.as_mut() {
                if track_epoch != s.track_epoch {
                    // Stale-epoch result — worker started against a prior
                    // track's PCM. The cache was cleared and the in-flight
                    // gate was already reset at the track boundary, so
                    // touching either here would either poison the new
                    // track's cache or wrongly free the slot the current-
                    // epoch worker still holds. Drop silently.
                    LUFS_WORKERS_REJECTED_STALE_EPOCH.fetch_add(1, Ordering::Relaxed);
                } else {
                    // Always cache the completed measurement, even if a
                    // newer UpdateChain has already moved generation past
                    // this worker's. A revisit to these settings (the user
                    // wiggling back to a prior knob position) then hits the
                    // cache instead of spawning another measurement.
                    s.landing_gain_cache.insert(&settings, gain);
                    if generation == s.live_coeff_generation {
                        // Still the live setting — promote to the active
                        // landing scalar and emit a corrective LiveCoeffUpdate
                        // so the audio output thread crossfades to the
                        // accurate gain.
                        LUFS_WORKERS_APPLIED.fetch_add(1, Ordering::Relaxed);
                        s.live_landing_gain_lin = gain;
                        if let Some(tx) = s.live_coeffs_tx.as_ref() {
                            let mut coeffs = crate::dsp::ChainCoeffs::from_settings(
                                s.live_sample_rate,
                                &settings,
                            );
                            coeffs.export_landing_gain_lin = gain;
                            let _ = tx.send(LiveCoeffUpdate { generation, coeffs });
                        }
                    } else {
                        LUFS_WORKERS_CACHED_ONLY.fetch_add(1, Ordering::Relaxed);
                    }
                    // Worker is no longer running. Drain any pending
                    // measurement and kick off the follow-up if it isn't
                    // already covered by the cache.
                    s.lufs_worker_in_flight = false;
                    if let Some((pending_settings, pending_generation)) =
                        s.lufs_worker_pending.take()
                    {
                        if s.landing_gain_cache.get(&pending_settings).is_some() {
                            // Already cached (e.g. the user landed back on a
                            // measured setting). Nothing to do.
                        } else if try_spawn_lufs_preview_worker(
                            s.decoded_cache.as_ref(),
                            s.live_sample_rate,
                            pending_settings,
                            pending_generation,
                            s.track_epoch,
                            command_tx,
                        ) {
                            s.lufs_worker_in_flight = true;
                        }
                    }
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
        AudioCommand::Seek {
            position_sec,
            reply,
        } => {
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

fn audio_thread(
    rx: mpsc::Receiver<AudioCommand>,
    command_tx: Sender<AudioCommand>,
    snapshot: Arc<RwLock<PlaybackSnapshot>>,
    prewarm_cache: SharedDecodedCache,
) {
    let mut state: Option<AudioThreadState> = None;
    loop {
        // Wait for at least one command (50 ms tick matches the prior
        // poll cadence so loop-region / snapshot housekeeping below
        // still runs every ~50 ms even when no commands arrive).
        let first_cmd: Option<AudioCommand> = match rx.recv_timeout(Duration::from_millis(50)) {
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

            let sequenced = coalesced_command_sequence(buffered);
            for c in sequenced {
                if process_audio_command(c, &mut state, &prewarm_cache, &command_tx) {
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
                let lufs_integrated = to_lufs(s.integrated_lufs_x100.load(Ordering::Relaxed));
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
    prewarm_cache: &SharedDecodedCache,
) -> Result<(), String> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mtime = std::fs::metadata(&canonical)
        .ok()
        .and_then(|m| m.modified().ok());
    let pcm = resolve_pcm_with_caches(state.as_ref(), prewarm_cache, path, &canonical, mtime)?;

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
            live_coeff_generation: 0,
            live_landing_gain_lin: 1.0,
            live_sample_rate: pcm.sample_rate,
            track_epoch: 0,
            lufs_worker_in_flight: false,
            lufs_worker_pending: None,
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
    s.live_coeff_generation = s.live_coeff_generation.wrapping_add(1);
    s.live_landing_gain_lin = 1.0;
    s.live_sample_rate = sample_rate;
    // Track epoch bump invalidates any in-flight LUFS workers from the
    // previous track. They'll still complete and send PreviewLandingReady,
    // but the handler rejects mismatched epochs before touching the
    // landing-gain cache. Original playback never spawns workers itself,
    // but the bump matters when the user toggles Master -> Original mid-
    // measurement.
    s.track_epoch = s.track_epoch.wrapping_add(1);
    s.lufs_worker_in_flight = false;
    s.lufs_worker_pending = None;
    Ok(())
}

fn handle_play_master(
    state: &mut Option<AudioThreadState>,
    track_id: TrackId,
    path: &Path,
    settings: &MasteringSettings,
    start_position_sec: f64,
    preview_lufs_landing: bool,
    prewarm_cache: &SharedDecodedCache,
) -> Result<(), String> {
    // Three-tier PCM resolution (fastest to slowest):
    //   1. Local "currently-playing" cache on AudioThreadState —
    //      same-track replay (Original/Mastered toggle).
    //   2. Shared prewarm cache — populated off the audio thread by
    //      `prewarm_decode` when the user selects a track in the UI.
    //      The 1-2 s freeze on first Mastered click for long WAVs is
    //      what motivates this tier.
    //   3. Fresh `decode_full` — synchronous on the audio thread, the
    //      cold case prewarm exists to avoid.
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mtime = std::fs::metadata(&canonical)
        .ok()
        .and_then(|m| m.modified().ok());
    let pcm = resolve_pcm_with_caches(state.as_ref(), prewarm_cache, path, &canonical, mtime)?;

    // Cache invalidation: clear the landing-gain cache when canonical
    // path OR mtime differs from the prior decoded cache entry. Same-
    // path replays at the same mtime (Original/Mastered toggle,
    // repeated play_master on the same track) preserve the cache
    // because the PCM is unchanged. New track OR re-saved file →
    // entries were computed against the OLD PCM and would mis-land
    // the new one.
    let cache_stale = should_invalidate_landing_cache(
        state.as_ref().and_then(|s| s.decoded_cache.as_ref()),
        &canonical,
        mtime,
    );
    if cache_stale {
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
            live_coeff_generation: 0,
            live_landing_gain_lin: 1.0,
            live_sample_rate: pcm.sample_rate,
            track_epoch: 0,
            lufs_worker_in_flight: false,
            lufs_worker_pending: None,
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

    let (coeffs_tx, coeffs_rx) = mpsc::channel::<LiveCoeffUpdate>();
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
        s.landing_gain_cache
            .insert(settings, chain.coeffs.export_landing_gain_lin);
    }
    s.live_coeff_generation = s.live_coeff_generation.wrapping_add(1);
    s.live_landing_gain_lin = chain.coeffs.export_landing_gain_lin;
    // Track epoch bump invalidates any LUFS preview workers spawned
    // against the prior track. Also wipe the in-flight gate + pending
    // slot so the next UpdateChain spawns a fresh worker rather than
    // queueing behind a dead one. Stale workers still in flight will
    // send PreviewLandingReady against the OLD epoch; the handler
    // rejects them before they can touch the landing-gain cache or
    // emit a corrective LiveCoeffUpdate against the new track.
    s.track_epoch = s.track_epoch.wrapping_add(1);
    s.lufs_worker_in_flight = false;
    s.lufs_worker_pending = None;
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
// MeteredPcmSource + MasteringSource live in `crate::sources` — both are
// pub(crate) so this module + its tests construct them; the rest of the
// crate doesn't see them.
// ============================================================================

// ============================================================================
// Tests
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
            eq_sub_db: 0.0,
            eq_low_db: 0.0,
            eq_low_mid_db: 0.0,
            eq_mid_db: 0.0,
            eq_high_mid_db: 0.0,
            eq_high_db: 0.0,
            eq_sparkle_db: 0.0,
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
            let v =
                0.3 * (n as f32 / sample_rate as f32 * 2.0 * std::f32::consts::PI * 1000.0).sin();
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
    // Prewarm decode cache — mechanical gates for the off-thread decode
    // cache that eliminates the 1-2 s freeze on first Mastered click
    // for long WAVs. The cache stores a single entry keyed by
    // (canonical_path, mtime) and is consulted by handle_play_master as
    // tier 2 of the three-tier resolve_pcm_with_caches hierarchy.
    // ========================================================================

    /// Fresh AudioPlayer reports no cache hit — the prewarm cache
    /// starts empty.
    #[test]
    fn prewarm_cache_empty_after_construction() {
        let player = AudioPlayer::new();
        let path = std::path::PathBuf::from("/tmp/a.wav");
        assert!(!player.prewarm_cache_hit(&path, None));
    }

    /// After set, the same (path, mtime) reports a hit. The whole
    /// point of the cache.
    #[test]
    fn prewarm_cache_hit_after_set() {
        let player = AudioPlayer::new();
        let path = std::path::PathBuf::from("/tmp/a.wav");
        let mtime = Some(std::time::SystemTime::UNIX_EPOCH);
        player.set_prewarm_cache(DecodedCacheEntry {
            canonical_path: path.clone(),
            mtime,
            pcm: DecodedPcm {
                samples: vec![0.0, 0.0],
                sample_rate: 48_000,
                channels: 2,
            },
        });
        assert!(player.prewarm_cache_hit(&path, mtime));
    }

    /// Different path → miss. Cache distinguishes by canonical path.
    #[test]
    fn prewarm_cache_miss_on_different_path() {
        let player = AudioPlayer::new();
        let path_a = std::path::PathBuf::from("/tmp/a.wav");
        let path_b = std::path::PathBuf::from("/tmp/b.wav");
        let mtime = Some(std::time::SystemTime::UNIX_EPOCH);
        player.set_prewarm_cache(DecodedCacheEntry {
            canonical_path: path_a,
            mtime,
            pcm: DecodedPcm {
                samples: vec![],
                sample_rate: 48_000,
                channels: 2,
            },
        });
        assert!(!player.prewarm_cache_hit(&path_b, mtime));
    }

    /// Same path, different mtime → miss. The file was re-saved /
    /// replaced; cached PCM is stale. This is the same predicate
    /// shape that protects the landing_gain_cache.
    #[test]
    fn prewarm_cache_miss_on_mtime_change() {
        let player = AudioPlayer::new();
        let path = std::path::PathBuf::from("/tmp/a.wav");
        let old_mtime = Some(std::time::SystemTime::UNIX_EPOCH);
        let new_mtime =
            Some(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(60));
        player.set_prewarm_cache(DecodedCacheEntry {
            canonical_path: path.clone(),
            mtime: old_mtime,
            pcm: DecodedPcm {
                samples: vec![],
                sample_rate: 48_000,
                channels: 2,
            },
        });
        assert!(!player.prewarm_cache_hit(&path, new_mtime));
    }

    /// set_prewarm_cache replaces the prior entry — single-slot LRU.
    /// After setting B, A's path no longer reports a hit.
    #[test]
    fn prewarm_cache_set_replaces_prior_entry() {
        let player = AudioPlayer::new();
        let path_a = std::path::PathBuf::from("/tmp/a.wav");
        let path_b = std::path::PathBuf::from("/tmp/b.wav");
        let mtime = Some(std::time::SystemTime::UNIX_EPOCH);
        player.set_prewarm_cache(DecodedCacheEntry {
            canonical_path: path_a.clone(),
            mtime,
            pcm: DecodedPcm {
                samples: vec![],
                sample_rate: 48_000,
                channels: 2,
            },
        });
        assert!(player.prewarm_cache_hit(&path_a, mtime));
        player.set_prewarm_cache(DecodedCacheEntry {
            canonical_path: path_b.clone(),
            mtime,
            pcm: DecodedPcm {
                samples: vec![],
                sample_rate: 48_000,
                channels: 2,
            },
        });
        assert!(!player.prewarm_cache_hit(&path_a, mtime));
        assert!(player.prewarm_cache_hit(&path_b, mtime));
    }

    // ----- Prewarm target guard (stale-prewarm-evicts-newer fix) ----

    /// Fresh AudioPlayer reports no target match (target is None).
    /// Future-Claude pitfall: don't change this to "matches whatever
    /// you ask" — the prewarm command relies on the None case
    /// returning false so the very first prewarm always has to
    /// declare its target via set_prewarm_target first.
    #[test]
    fn prewarm_target_unset_matches_nothing() {
        let player = AudioPlayer::new();
        let path = std::path::PathBuf::from("/tmp/a.wav");
        assert!(!player.prewarm_target_matches(&path));
    }

    /// set + check on the same path → match. Trivial pass-through
    /// but gates the basic happy path: a single prewarm declares
    /// its target and recognizes its own result.
    #[test]
    fn prewarm_target_matches_after_set_with_same_path() {
        let player = AudioPlayer::new();
        let path = std::path::PathBuf::from("/tmp/a.wav");
        player.set_prewarm_target(path.clone());
        assert!(player.prewarm_target_matches(&path));
    }

    /// set A → check B reports false. The discriminator: a slow
    /// prewarm of A that wakes up after B was selected sees its
    /// target is no longer A and should drop its result.
    #[test]
    fn prewarm_target_mismatch_on_different_path() {
        let player = AudioPlayer::new();
        let path_a = std::path::PathBuf::from("/tmp/a.wav");
        let path_b = std::path::PathBuf::from("/tmp/b.wav");
        player.set_prewarm_target(path_a);
        assert!(!player.prewarm_target_matches(&path_b));
    }

    /// set A → set B → check A reports false (LIFO replacement).
    /// This is the actual race that the guard fixes: user selects
    /// A, then B; A's slow decode finishes; A's check against the
    /// now-current target (B) returns false; A's result is dropped
    /// so B's cache entry stays.
    #[test]
    fn prewarm_target_set_replaces_prior_target() {
        let player = AudioPlayer::new();
        let path_a = std::path::PathBuf::from("/tmp/a.wav");
        let path_b = std::path::PathBuf::from("/tmp/b.wav");
        player.set_prewarm_target(path_a.clone());
        player.set_prewarm_target(path_b.clone());
        assert!(
            !player.prewarm_target_matches(&path_a),
            "slow prewarm A must NOT match after newer set B — \
             this is the stale-prewarm-evicts-newer guard"
        );
        assert!(
            player.prewarm_target_matches(&path_b),
            "current target B should still match itself"
        );
    }

    /// resolve_pcm_with_caches: tier-2 hit returns the prewarmed PCM
    /// without falling through to decode_full. Verifies the central
    /// promise of the prewarm feature: when the shared cache holds
    /// the right entry, the audio thread skips the synchronous decode.
    #[test]
    fn resolve_pcm_with_caches_tier_2_hit_short_circuits() {
        let prewarm: SharedDecodedCache = Arc::new(Mutex::new(None));
        let path = std::path::PathBuf::from("/tmp/prewarmed.wav");
        let canonical = path.clone();
        let mtime = Some(std::time::SystemTime::UNIX_EPOCH);
        // Pre-populate the shared cache with a distinctive sample
        // payload so we can verify the function returned THE PREWARMED
        // PCM (not a fresh decode).
        let distinctive_samples = vec![0.42_f32, 0.42_f32, 0.42_f32, 0.42_f32];
        {
            let mut guard = prewarm.lock().expect("lock");
            *guard = Some(DecodedCacheEntry {
                canonical_path: canonical.clone(),
                mtime,
                pcm: DecodedPcm {
                    samples: distinctive_samples.clone(),
                    sample_rate: 48_000,
                    channels: 2,
                },
            });
        }
        let result = resolve_pcm_with_caches(None, &prewarm, &path, &canonical, mtime)
            .expect("tier-2 hit must resolve without decode_full");
        assert_eq!(
            result.samples, distinctive_samples,
            "resolve must return the PREWARMED PCM, not a fresh decode"
        );
    }

    /// resolve_pcm_with_caches: tier-2 miss (different path) falls
    /// through to decode_full. With a bogus path the decode_full call
    /// errors — verifies the fall-through path is wired (decode_full
    /// is actually invoked when both cache tiers miss).
    #[test]
    fn resolve_pcm_with_caches_falls_through_on_double_miss() {
        let prewarm: SharedDecodedCache = Arc::new(Mutex::new(None));
        let path = std::path::PathBuf::from("/tmp/does-not-exist-rust-test.wav");
        let canonical = path.clone();
        // No state, no shared cache entry → both tiers miss → tier 3
        // (decode_full) runs, fails on the bogus path. The Err result
        // is the proof that tier 3 was reached (any unhandled fall-
        // through would short-circuit before this).
        let result = resolve_pcm_with_caches(None, &prewarm, &path, &canonical, None);
        assert!(
            result.is_err(),
            "double cache miss must fall through to decode_full, which errors on bogus path"
        );
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
        assert_eq!(
            compute_calls, 1,
            "clear() must force the next call to re-compute"
        );
    }

    // ========================================================================
    // Coalescing — mechanical gates for the knob-spam-protection layer.
    // coalesced_command_sequence() is the entire flow. If it stops
    // dropping intermediate UpdateChains, Seeks stall behind expensive
    // preview-LUFS measurements again. If it lets UpdateChains CROSS
    // playback barriers (Play / PlayMaster / Stop), stale settings
    // apply to the newly-loaded master. These tests gate both.
    // ========================================================================

    // ----- Test helpers ----------------------------------------------------

    fn dummy_play_master(track_id: &str) -> AudioCommand {
        let (reply, _rx) = mpsc::channel();
        AudioCommand::PlayMaster {
            track_id: TrackId(track_id.to_string()),
            path: std::path::PathBuf::from(format!("/tmp/{track_id}.wav")),
            settings: settings_with_intensity(0.5),
            start_position_sec: 0.0,
            preview_lufs_landing: true,
            reply,
        }
    }

    fn dummy_play(track_id: &str) -> AudioCommand {
        let (reply, _rx) = mpsc::channel();
        AudioCommand::Play {
            track_id: TrackId(track_id.to_string()),
            path: std::path::PathBuf::from(format!("/tmp/{track_id}.wav")),
            start_position_sec: 0.0,
            reply,
        }
    }

    fn update_chain_with_intensity(intensity: f32, preview: bool) -> AudioCommand {
        AudioCommand::UpdateChain {
            settings: settings_with_intensity(intensity),
            preview_lufs_landing: preview,
        }
    }

    fn update_chain_intensity(cmd: &AudioCommand) -> Option<f32> {
        match cmd {
            AudioCommand::UpdateChain { settings, .. } => Some(settings.intensity),
            _ => None,
        }
    }

    // ----- Basic coalescing properties -------------------------------------

    /// Empty input never panics and yields an empty sequence.
    #[test]
    fn coalesce_handles_empty_buffer() {
        let result = coalesced_command_sequence(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn lufs_preview_worker_returns_false_without_decoded_cache() {
        // No decoded PCM → nothing to measure. Helper must report
        // false so the caller doesn't mark in_flight.
        let (tx, rx) = mpsc::channel::<AudioCommand>();
        let spawned = try_spawn_lufs_preview_worker(
            None,
            44_100,
            settings_with_intensity(0.5),
            42,
            7,
            &tx,
        );
        assert!(!spawned, "spawn must report false when no PCM is cached");
        // No PreviewLandingReady should ever arrive on the channel.
        assert!(
            rx.recv_timeout(std::time::Duration::from_millis(50)).is_err(),
            "no worker started ⇒ no PreviewLandingReady should be sent"
        );
    }

    #[test]
    fn lufs_preview_worker_echoes_epoch_and_generation() {
        // With a real decoded cache, the worker must spawn, measure,
        // and report back with the SAME (track_epoch, generation) it
        // was handed. The handler relies on those round-trip values to
        // gate cache pollution from prior tracks and to know whether
        // the live setting has moved on.
        let sample_rate = 44_100;
        let channels: u16 = 2;
        let samples = sine_signal(sample_rate as usize, sample_rate, channels);
        let entry = DecodedCacheEntry {
            canonical_path: PathBuf::from("/fake/canonical/a.wav"),
            mtime: Some(std::time::SystemTime::UNIX_EPOCH),
            pcm: DecodedPcm {
                samples,
                sample_rate,
                channels,
            },
        };
        let (tx, rx) = mpsc::channel::<AudioCommand>();
        let spawned_gen: u64 = 9001;
        let spawned_epoch: u64 = 17;
        let spawned = try_spawn_lufs_preview_worker(
            Some(&entry),
            sample_rate,
            settings_with_intensity(0.5),
            spawned_gen,
            spawned_epoch,
            &tx,
        );
        assert!(spawned, "spawn must report true with decoded PCM available");
        let msg = rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("worker should report PreviewLandingReady within 2s");
        match msg {
            AudioCommand::PreviewLandingReady {
                track_epoch,
                generation,
                gain,
                ..
            } => {
                assert_eq!(track_epoch, spawned_epoch, "epoch must round-trip");
                assert_eq!(generation, spawned_gen, "generation must round-trip");
                assert!(
                    gain.is_finite() && gain > 0.0,
                    "measured gain should be finite and positive, got {gain}"
                );
            }
            _ => panic!("expected PreviewLandingReady, got a different AudioCommand variant"),
        }
    }

    /// A single non-UpdateChain command passes through unmodified.
    #[test]
    fn coalesce_single_non_update_chain_passes_through() {
        let result = coalesced_command_sequence(vec![AudioCommand::Pause]);
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], AudioCommand::Pause));
    }

    /// A single UpdateChain survives at the tail of the sequence.
    #[test]
    fn coalesce_single_update_chain_survives_at_end() {
        let result = coalesced_command_sequence(vec![update_chain_with_intensity(0.42, true)]);
        assert_eq!(result.len(), 1);
        assert_eq!(update_chain_intensity(&result[0]), Some(0.42));
    }

    /// Within a segment (no barrier), intermediate UpdateChains are
    /// dropped, the latest survives, and non-UpdateChain commands keep
    /// their submission order with the UpdateChain dispatched LAST.
    /// This is Dan's original knob-spam repro shape.
    #[test]
    fn coalesce_drops_intermediates_and_keeps_seeks_before_latest_update() {
        let (seek_reply, _rx) = mpsc::channel();
        let buffered = vec![
            update_chain_with_intensity(0.1, false),
            AudioCommand::Pause,
            update_chain_with_intensity(0.5, false),
            AudioCommand::Seek {
                position_sec: 42.0,
                reply: seek_reply,
            },
            update_chain_with_intensity(0.9, true),
        ];
        let result = coalesced_command_sequence(buffered);

        // Pause, Seek (in submission order), then the latest UpdateChain.
        assert_eq!(result.len(), 3);
        assert!(matches!(result[0], AudioCommand::Pause));
        match &result[1] {
            AudioCommand::Seek { position_sec, .. } => {
                assert!((*position_sec - 42.0).abs() < 1e-9);
            }
            _ => panic!("expected Seek at position 1"),
        }
        match &result[2] {
            AudioCommand::UpdateChain {
                settings,
                preview_lufs_landing,
            } => {
                assert!((settings.intensity - 0.9).abs() < 1e-6);
                assert!(*preview_lufs_landing);
            }
            _ => panic!("expected latest UpdateChain at position 2"),
        }
    }

    /// Long run of UpdateChains collapses to the last one.
    #[test]
    fn coalesce_collapses_long_run_of_update_chains_to_last() {
        let buffered: Vec<AudioCommand> = (0..20)
            .map(|i| update_chain_with_intensity(i as f32 / 20.0, i % 2 == 0))
            .collect();
        let result = coalesced_command_sequence(buffered);
        assert_eq!(result.len(), 1);
        assert_eq!(update_chain_intensity(&result[0]), Some(0.95)); // i=19
    }

    // ----- Playback-barrier semantics (Codex review fix) -------------------

    /// PlayMaster is a barrier — UpdateChains do NOT cross it. A stale
    /// UpdateChain queued before PlayMaster must apply BEFORE the track
    /// switch (so it lands on the soon-to-be-replaced live_coeffs_tx),
    /// and a fresh UpdateChain queued after PlayMaster must apply to
    /// the newly-loaded master. This is the killer test: pre-fix, the
    /// "latest UpdateChain wins" rule was global across the entire
    /// drained batch, so a pre-PlayMaster UpdateChain could be reordered
    /// to dispatch AFTER PlayMaster and clobber the new track's chain
    /// with stale settings from the OLD track.
    #[test]
    fn coalesce_does_not_cross_play_master_barrier() {
        let buffered = vec![
            update_chain_with_intensity(0.1, false), // OLD track
            dummy_play_master("new-track"),
            update_chain_with_intensity(0.9, true), // NEW track
        ];
        let result = coalesced_command_sequence(buffered);
        assert_eq!(result.len(), 3);
        assert_eq!(
            update_chain_intensity(&result[0]),
            Some(0.1),
            "pre-barrier UpdateChain must stay BEFORE PlayMaster"
        );
        assert!(
            matches!(result[1], AudioCommand::PlayMaster { .. }),
            "PlayMaster must stay between the two segments"
        );
        assert_eq!(
            update_chain_intensity(&result[2]),
            Some(0.9),
            "post-barrier UpdateChain must stay AFTER PlayMaster"
        );
    }

    /// Play is a barrier too (Source playback can be started on a new
    /// track; UpdateChains shouldn't carry across).
    #[test]
    fn coalesce_does_not_cross_play_barrier() {
        let buffered = vec![
            update_chain_with_intensity(0.1, false),
            dummy_play("new-track"),
            update_chain_with_intensity(0.9, true),
        ];
        let result = coalesced_command_sequence(buffered);
        assert_eq!(result.len(), 3);
        assert_eq!(update_chain_intensity(&result[0]), Some(0.1));
        assert!(matches!(result[1], AudioCommand::Play { .. }));
        assert_eq!(update_chain_intensity(&result[2]), Some(0.9));
    }

    /// Stop is a barrier — clears current_track. UpdateChains after
    /// Stop (e.g., the user prepping new settings before pressing Play)
    /// MUST stay after the Stop, not be reordered before it.
    #[test]
    fn coalesce_does_not_cross_stop_barrier() {
        let buffered = vec![
            update_chain_with_intensity(0.1, false),
            AudioCommand::Stop,
            update_chain_with_intensity(0.9, true),
        ];
        let result = coalesced_command_sequence(buffered);
        assert_eq!(result.len(), 3);
        assert_eq!(update_chain_intensity(&result[0]), Some(0.1));
        assert!(matches!(result[1], AudioCommand::Stop));
        assert_eq!(update_chain_intensity(&result[2]), Some(0.9));
    }

    /// Each segment coalesces independently. A long run of UpdateChains
    /// before PlayMaster collapses to its own latest; same for after.
    /// Verifies the per-segment coalescing rule fires on both sides of
    /// a barrier instead of treating the whole buffer as one segment.
    #[test]
    fn coalesce_collapses_each_segment_independently_around_barrier() {
        let buffered = vec![
            update_chain_with_intensity(0.1, false),
            update_chain_with_intensity(0.2, false),
            update_chain_with_intensity(0.3, false),
            dummy_play_master("track-b"),
            update_chain_with_intensity(0.7, false),
            update_chain_with_intensity(0.8, false),
            update_chain_with_intensity(0.9, true),
        ];
        let result = coalesced_command_sequence(buffered);
        // Pre-barrier segment collapses to the last of (0.1, 0.2, 0.3) = 0.3
        // Then PlayMaster
        // Post-barrier segment collapses to the last of (0.7, 0.8, 0.9) = 0.9
        assert_eq!(result.len(), 3);
        assert_eq!(update_chain_intensity(&result[0]), Some(0.3));
        assert!(matches!(result[1], AudioCommand::PlayMaster { .. }));
        assert_eq!(update_chain_intensity(&result[2]), Some(0.9));
    }

    /// Pause / Resume / Seek / SetLoop are NOT barriers — they don't
    /// change the loaded track. UpdateChains can coalesce across them
    /// (subject to the "non-UpdateChain commands keep submission order,
    /// latest UpdateChain dispatched LAST" rule within the segment).
    #[test]
    fn coalesce_does_treat_pause_resume_seek_as_non_barriers() {
        let (seek_reply, _rx) = mpsc::channel();
        let buffered = vec![
            update_chain_with_intensity(0.1, false),
            AudioCommand::Pause,
            update_chain_with_intensity(0.5, false),
            AudioCommand::Resume,
            update_chain_with_intensity(0.7, false),
            AudioCommand::Seek {
                position_sec: 12.0,
                reply: seek_reply,
            },
            update_chain_with_intensity(0.9, true),
            AudioCommand::SetLoop(None),
        ];
        let result = coalesced_command_sequence(buffered);
        // All non-UpdateChain commands keep their order (Pause, Resume,
        // Seek, SetLoop), and the latest UpdateChain (0.9) dispatches
        // last after all of them.
        assert_eq!(result.len(), 5);
        assert!(matches!(result[0], AudioCommand::Pause));
        assert!(matches!(result[1], AudioCommand::Resume));
        assert!(matches!(result[2], AudioCommand::Seek { .. }));
        assert!(matches!(result[3], AudioCommand::SetLoop(None)));
        assert_eq!(update_chain_intensity(&result[4]), Some(0.9));
    }

    // ----- Landing-cache invalidation (Codex review fix) -------------------

    /// No prior entry → always invalidate. First load needs to fall
    /// through the predicate's positive branch.
    #[test]
    fn invalidate_landing_cache_when_no_prior_entry() {
        let path = std::path::PathBuf::from("/tmp/a.wav");
        assert!(should_invalidate_landing_cache(None, &path, None));
    }

    /// Same canonical path AND same mtime → DO NOT invalidate. This
    /// is the Original/Mastered toggle case where the cache should be
    /// preserved.
    #[test]
    fn keep_landing_cache_when_path_and_mtime_match() {
        let path = std::path::PathBuf::from("/tmp/a.wav");
        let mtime = Some(std::time::SystemTime::UNIX_EPOCH);
        let prior = DecodedCacheEntry {
            canonical_path: path.clone(),
            mtime,
            pcm: DecodedPcm {
                samples: vec![],
                sample_rate: 48_000,
                channels: 2,
            },
        };
        assert!(!should_invalidate_landing_cache(Some(&prior), &path, mtime));
    }

    /// Different canonical path → invalidate. User switched tracks.
    #[test]
    fn invalidate_landing_cache_on_path_change() {
        let prior_path = std::path::PathBuf::from("/tmp/a.wav");
        let new_path = std::path::PathBuf::from("/tmp/b.wav");
        let mtime = Some(std::time::SystemTime::UNIX_EPOCH);
        let prior = DecodedCacheEntry {
            canonical_path: prior_path,
            mtime,
            pcm: DecodedPcm {
                samples: vec![],
                sample_rate: 48_000,
                channels: 2,
            },
        };
        assert!(should_invalidate_landing_cache(
            Some(&prior),
            &new_path,
            mtime
        ));
    }

    /// Same path BUT different mtime → invalidate. The file was
    /// re-saved or replaced; cached landing gains were computed
    /// against the OLD PCM and would mis-land the new one. Pre-fix,
    /// only path-change invalidation existed, so a same-path /
    /// different-mtime case would silently mis-land.
    #[test]
    fn invalidate_landing_cache_on_mtime_change_same_path() {
        let path = std::path::PathBuf::from("/tmp/a.wav");
        let old_mtime = Some(std::time::SystemTime::UNIX_EPOCH);
        let new_mtime =
            Some(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(60));
        let prior = DecodedCacheEntry {
            canonical_path: path.clone(),
            mtime: old_mtime,
            pcm: DecodedPcm {
                samples: vec![],
                sample_rate: 48_000,
                channels: 2,
            },
        };
        assert!(
            should_invalidate_landing_cache(Some(&prior), &path, new_mtime),
            "same-path / different-mtime must invalidate — pre-fix this slipped through"
        );
    }

    /// Same path with prior mtime Some and new mtime None (file no
    /// longer reports mtime — e.g. filesystem doesn't support it
    /// after a path swap) → invalidate. The Option mismatch IS a
    /// mtime mismatch under PartialEq.
    #[test]
    fn invalidate_landing_cache_on_mtime_disappearing() {
        let path = std::path::PathBuf::from("/tmp/a.wav");
        let prior = DecodedCacheEntry {
            canonical_path: path.clone(),
            mtime: Some(std::time::SystemTime::UNIX_EPOCH),
            pcm: DecodedPcm {
                samples: vec![],
                sample_rate: 48_000,
                channels: 2,
            },
        };
        assert!(should_invalidate_landing_cache(Some(&prior), &path, None));
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
        let measured = crate::engine::measure_integrated_lufs(&rendered, sample_rate, channels)
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
        let peak = rendered.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        let peak_dbfs = if peak > 0.0 {
            20.0 * peak.log10()
        } else {
            -120.0
        };
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
        let initial_chain = MasteringChain::new(sample_rate, channels as usize, &initial_settings);
        let (coeffs_tx, coeffs_rx) = mpsc::channel::<LiveCoeffUpdate>();
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
        coeffs_tx
            .send(LiveCoeffUpdate {
                generation: 1,
                coeffs: new_coeffs,
            })
            .expect("send new coeffs");

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
    fn mastering_source_ignores_stale_live_coeff_generations() {
        let sample_rate = 44_100;
        let channels: u16 = 2;
        let total_frames = 24_576;
        let samples = sine_signal(total_frames, sample_rate, channels);

        let initial_settings = settings_with_intensity(0.0);
        let initial_chain = MasteringChain::new(sample_rate, channels as usize, &initial_settings);
        let (coeffs_tx, coeffs_rx) = mpsc::channel::<LiveCoeffUpdate>();
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

        let third = total_frames * channels as usize / 3;
        let mut output_initial: Vec<f32> = Vec::with_capacity(third);
        for _ in 0..third {
            if let Some(s) = source.next() {
                output_initial.push(s);
            }
        }

        let mut newer_settings = settings_with_intensity(0.0);
        newer_settings.output_gain_db = 6.0;
        let newer_coeffs = ChainCoeffs::from_settings(sample_rate, &newer_settings);

        let mut stale_settings = settings_with_intensity(0.0);
        stale_settings.output_gain_db = -24.0;
        let stale_coeffs = ChainCoeffs::from_settings(sample_rate, &stale_settings);

        coeffs_tx
            .send(LiveCoeffUpdate {
                generation: 2,
                coeffs: newer_coeffs,
            })
            .expect("send newer coeffs");
        coeffs_tx
            .send(LiveCoeffUpdate {
                generation: 1,
                coeffs: stale_coeffs.clone(),
            })
            .expect("send stale coeffs");

        let mut output_newer: Vec<f32> = Vec::with_capacity(third);
        for _ in 0..third {
            if let Some(s) = source.next() {
                output_newer.push(s);
            }
        }

        coeffs_tx
            .send(LiveCoeffUpdate {
                generation: 1,
                coeffs: stale_coeffs,
            })
            .expect("send late stale coeffs");

        let mut output_after_late_stale: Vec<f32> = Vec::with_capacity(third);
        for _ in 0..third {
            if let Some(s) = source.next() {
                output_after_late_stale.push(s);
            }
        }

        let warmup_skip = 4096;
        let rms_initial = rms(&output_initial[warmup_skip..]);
        let rms_newer = rms(&output_newer[warmup_skip..]);
        let rms_after_late_stale = rms(&output_after_late_stale[warmup_skip..]);

        assert!(
            rms_newer > rms_initial * 1.6,
            "newest generation should apply the louder coeffs \
             (initial={rms_initial:.4}, newer={rms_newer:.4})"
        );
        assert!(
            rms_after_late_stale > rms_initial * 1.6,
            "late stale generation must not overwrite the active newer coeffs \
             (initial={rms_initial:.4}, after_late_stale={rms_after_late_stale:.4})"
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
    fn mastering_source_tracks_latest_under_sustained_updates() {
        // Reproduces the realtime stutter / laggy-knob symptom: pre-Fix-A,
        // sustained coefficient updates (knob sweep) re-armed the crossfade
        // every check interval, leaving self.chain frozen at the pre-sweep
        // settings. Output stayed dominated by the stale chain and the
        // source ran 2x DSP for the whole sweep.
        //
        // Post-Fix-A, each mid-fade update promotes the prior pending chain
        // to main before installing the new one, so the audible chain
        // converges to the latest settings within at most one
        // COEFFS_CROSSFADE_FRAMES window.
        let sample_rate = 44_100;
        let channels: u16 = 2;
        let total_frames = 16_384;
        let samples = sine_signal(total_frames, sample_rate, channels);

        let initial_settings = settings_with_intensity(0.0);
        let initial_chain =
            MasteringChain::new(sample_rate, channels as usize, &initial_settings);
        let (coeffs_tx, coeffs_rx) = mpsc::channel::<LiveCoeffUpdate>();
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

        let loud_settings = settings_with_intensity(1.0);
        let loud_coeffs = ChainCoeffs::from_settings(sample_rate, &loud_settings);

        // Pre-update baseline window, then a sustained stream of "loud"
        // updates at twice the check interval rate so every coeff check
        // finds something pending.
        let baseline_samples = 1024_usize;
        let send_every = 64_usize;
        let total_samples = total_frames * channels as usize;
        let mut output: Vec<f32> = Vec::with_capacity(total_samples);
        let mut gen_counter: u64 = 0;
        for i in 0..total_samples {
            if i >= baseline_samples && i % send_every == 0 {
                gen_counter += 1;
                coeffs_tx
                    .send(LiveCoeffUpdate {
                        generation: gen_counter,
                        coeffs: loud_coeffs.clone(),
                    })
                    .expect("send loud coeffs");
            }
            match source.next() {
                Some(s) => output.push(s),
                None => break,
            }
        }
        assert!(gen_counter > 0, "test should have sent updates");

        // Compare the late steady-state region against the pre-update
        // baseline. Intensity 0.0 -> 1.0 raises Universal gain push from
        // ~0.6 dB to ~2.4 dB (~1.23x RMS). Pre-Fix-A this jump never
        // materialized under sustained updates because self.chain was
        // never promoted; post-Fix-A the late RMS clearly tracks loud.
        let warmup_skip = 512_usize;
        let pre_steady = &output[warmup_skip..baseline_samples];
        let late_start = output.len().saturating_sub(4096);
        let late_steady = &output[late_start..];
        let rms_pre = rms(pre_steady);
        let rms_late = rms(late_steady);
        let ratio = rms_late / rms_pre;
        assert!(
            ratio > 1.15,
            "Fix A regression: source did not converge to loud settings under \
             sustained updates. rms_pre={rms_pre:.4}, rms_late={rms_late:.4}, \
             ratio={ratio:.3}. This indicates the crossfade is being \
             permanently reset and self.chain is frozen at the initial chain."
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
        let later = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(60);
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
        let ref_chain = MasteringChain::new(sample_rate, channels as usize, &initial_settings);
        let (_ref_tx, ref_rx) = mpsc::channel::<LiveCoeffUpdate>();
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
        let live_chain = MasteringChain::new(sample_rate, channels as usize, &initial_settings);
        let (live_tx, live_rx) = mpsc::channel::<LiveCoeffUpdate>();
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
        live_tx
            .send(LiveCoeffUpdate {
                generation: 1,
                coeffs: new_coeffs,
            })
            .expect("send new coeffs");
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
        let (_tx, rx) = mpsc::channel::<LiveCoeffUpdate>();
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
        let (_tx, rx) = mpsc::channel::<LiveCoeffUpdate>();
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
        let (_tx, rx) = mpsc::channel::<LiveCoeffUpdate>();
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

        assert!(
            first > 0.0,
            "first window saw signal, expected >0, got {first}"
        );
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

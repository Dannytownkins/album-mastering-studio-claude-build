use crate::types::*;
use crate::analysis::{
    analyze_one, compute_dynamic_range_p95_p10, compute_energy_density_score,
    compute_spectral_balance_6band, compute_transient_flux, nudge_role_by_position, sanitize_lufs,
};
use crate::wav_writer::{wav_spec, write_samples_into_writer, write_wav};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ebur128::{EbuR128, Mode};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub id: TrackId,
    pub path: String,
}

/// Phase 12.1 export progress — emitted on the "render:progress" Tauri event
/// channel during `render_track_master` / `render_track_preview` so the
/// frontend can render a real progress bar.
#[derive(Debug, Serialize, Clone)]
pub struct RenderProgress {
    pub track_id: TrackId,
    pub kind: RenderKind,
    pub fraction: f32,
}

#[derive(Debug, Deserialize)]
pub struct AlbumTrackInput {
    pub id: TrackId,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct AlbumRenderRequest {
    pub tracks: Vec<AlbumTrackInput>,
    pub album_intent: MasteringSettings,
    pub per_track_overrides: Option<std::collections::HashMap<String, MasteringSettings>>,
}

#[tauri::command]
pub async fn analyze_tracks(tracks: Vec<AnalyzeRequest>) -> CommandResult<Vec<AnalysisResult>> {
    let total = tracks.len();
    let mut out = Vec::with_capacity(total);
    let mut failures: Vec<(TrackId, String)> = Vec::new();
    for (index, req) in tracks.into_iter().enumerate() {
        match analyze_one(req.id.clone(), Path::new(&req.path)) {
            Ok(mut result) => {
                nudge_role_by_position(&mut result, index, total);
                out.push(result);
            }
            Err(e) => {
                failures.push((req.id, e.to_string()));
            }
        }
    }
    // Partial-success policy: if every track failed, surface the first error
    // (otherwise the frontend has no signal at all). If at least one succeeded,
    // return the successes and log the failures — session restore and bulk
    // imports can keep working when one source file has moved.
    if out.is_empty() && !failures.is_empty() {
        let (_, msg) = &failures[0];
        return Err(CommandError::Other(format!(
            "analyze failed for all tracks: {msg}"
        )));
    }
    for (id, msg) in failures {
        eprintln!("analyze_tracks: skipping {} — {}", id.as_str(), msg);
    }
    Ok(out)
}

// ============================================================================
// Ceiling-bounded LUFS landing — shared helpers used by every render path.
//
// Pre-extraction, this math was duplicated across three render paths
// (mastering_render_with_progress, album_render_with_progress,
// render_album_plan_impl) plus a fourth shape variant in
// audio.rs::export_landing_gain_lin_for_preview. The B6 ceiling-bounded
// behavior shipped as three near-identical blocks, and the album-plan
// copy was missed for almost a full session — exactly the drift the
// extraction is meant to prevent.
//
// Two-tier API:
//   * `ceiling_bounded_landing_delta_db`: pure math. Computes the
//     applied delta in dB given pre-measured LUFS+TP and the target/
//     ceiling. Returned value is 0.0 when the landing is a no-op
//     (silent signal, near-zero delta, or no headroom for upward push).
//   * `apply_ceiling_bounded_landing_with_measurements`: math + in-place
//     gain multiply. Returns the applied delta in dB so callers that
//     track post-landing measurements (e.g. the track-export receipt)
//     can shift their tracked LUFS+TP by the same amount.
//   * `measure_and_apply_ceiling_bounded_landing`: full ebur128 pass +
//     apply. For callers that don't already have LUFS+TP measurements
//     in hand (album-simple, album-plan).
//
// The audio.rs live-preview helper uses the pure-math tier directly
// because it returns a gain scalar rather than mutating samples, and
// because its ebur128 setup is measured on a 8 s window (perf
// optimization that the offline render paths intentionally don't share).
// ============================================================================

/// Compute the LUFS-landing delta in dB given pre-measured loudness +
/// true peak. Downward delta applies in full (the limiter already
/// capped peaks at ceiling, so attenuating only moves them further
/// away). Upward delta is bounded by the residual true-peak headroom
/// below the user's ceiling. Returns 0.0 when:
///
///   * the target or measurement is non-finite, or the signal is
///     effectively silent (measured_lufs <= -70 LUFS),
///   * the applied delta would be within ±1e-4 dB of zero (numerical
///     no-op — skip the gain multiply entirely).
///
/// The earlier refuse-upward policy (citing the Sonible / Ozone /
/// Mastering The Mix industry survey) was retired during B6 in favor
/// of letting the user push toward their stated target. The live
/// Export LUFS preview shows the resulting level in real time, so
/// what the user hears is what export writes — no hidden cap.
pub(crate) fn ceiling_bounded_landing_delta_db(
    measured_lufs: f32,
    measured_true_peak_dbtp: f32,
    target_lufs: f32,
    ceiling_dbtp: f32,
) -> f32 {
    if !target_lufs.is_finite() || !measured_lufs.is_finite() || measured_lufs <= -70.0 {
        return 0.0;
    }
    let delta_db = target_lufs - measured_lufs;
    let headroom_db = (ceiling_dbtp - measured_true_peak_dbtp).max(0.0);
    let applied_delta_db = if delta_db < 0.0 {
        delta_db
    } else {
        delta_db.min(headroom_db)
    };
    if applied_delta_db.abs() > 1.0e-4 {
        applied_delta_db
    } else {
        0.0
    }
}

/// Apply ceiling-bounded LUFS landing in-place to a sample slice given
/// pre-measured loudness + true peak. Returns the applied delta in
/// dB (0.0 if no gain was applied) so callers that track post-landing
/// measurements can shift them by the same amount via
/// `measured_lufs += applied; measured_true_peak_dbtp += applied;`.
///
/// Under a uniform linear gain `g`, integrated LUFS and true-peak
/// both shift by exactly `20·log10(g)` dB — so callers never need to
/// re-run the ebur128 pass after scaling.
fn apply_ceiling_bounded_landing_with_measurements(
    samples: &mut [f32],
    measured_lufs: f32,
    measured_true_peak_dbtp: f32,
    target_lufs: f32,
    ceiling_dbtp: f32,
) -> f32 {
    let applied_delta_db = ceiling_bounded_landing_delta_db(
        measured_lufs,
        measured_true_peak_dbtp,
        target_lufs,
        ceiling_dbtp,
    );
    if applied_delta_db != 0.0 {
        let gain_lin = 10.0_f32.powf(applied_delta_db / 20.0);
        for s in samples.iter_mut() {
            *s *= gain_lin;
        }
    }
    applied_delta_db
}

/// Full-stack ceiling-bounded LUFS landing: measure integrated LUFS +
/// BS.1770 true peak via ebur128, compute the bounded delta, apply in
/// place. Used by render paths that don't already have measurements
/// in hand (album-simple, album-plan). The track-export path measures
/// separately so it can also feed the receipt's `RenderedMeasurements`,
/// and routes through `apply_ceiling_bounded_landing_with_measurements`
/// directly.
fn measure_and_apply_ceiling_bounded_landing(
    samples: &mut [f32],
    sample_rate: u32,
    channels: u16,
    settings: &MasteringSettings,
) -> CommandResult<()> {
    let Some(target_lufs) = settings.effective_target_lufs() else {
        return Ok(());
    };
    if !target_lufs.is_finite() {
        return Ok(());
    }
    let channels_u32 = u32::from(channels.max(1));
    let mut ebu = EbuR128::new(channels_u32, sample_rate, Mode::I | Mode::TRUE_PEAK)
        .map_err(|e| CommandError::Render(format!("ebur128 init: {e}")))?;
    ebu.add_frames_f32(samples)
        .map_err(|e| CommandError::Render(format!("ebur128 feed: {e}")))?;
    let measured_lufs = sanitize_lufs(
        ebu.loudness_global()
            .map_err(|e| CommandError::Render(format!("ebur128 global: {e}")))? as f32,
    );
    let mut peak_lin: f64 = 0.0;
    for ch in 0..channels_u32 {
        let tp = ebu
            .true_peak(ch)
            .map_err(|e| CommandError::Render(format!("ebur128 tp: {e}")))?;
        if tp > peak_lin {
            peak_lin = tp;
        }
    }
    let measured_true_peak_dbtp = if peak_lin > 0.0 {
        (20.0 * peak_lin.log10()) as f32
    } else {
        -60.0
    };
    let ceiling_dbtp = settings.effective_ceiling_dbtp();
    apply_ceiling_bounded_landing_with_measurements(
        samples,
        measured_lufs,
        measured_true_peak_dbtp,
        target_lufs,
        ceiling_dbtp,
    );
    Ok(())
}

/// Measure post-render integrated loudness (BS.1770) of an interleaved f32
/// buffer. Returns the raw ebur128 reading — callers should treat values
/// below -70 LUFS as "effectively silent" and skip downstream gain math, the
/// same way `analyze_tracks` does. Used by the LUFS-landing stage in
/// `mastering_render_with_progress` and by contract tests that verify the
/// landing actually lands.
pub fn measure_integrated_lufs(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> CommandResult<f32> {
    let channels_u32 = u32::from(channels.max(1));
    let mut ebu = EbuR128::new(channels_u32, sample_rate, Mode::I)
        .map_err(|e| CommandError::Render(format!("ebur128 init: {e}")))?;
    ebu.add_frames_f32(samples)
        .map_err(|e| CommandError::Render(format!("ebur128 feed: {e}")))?;
    Ok(ebu
        .loudness_global()
        .map_err(|e| CommandError::Render(format!("ebur128 global: {e}")))?
        as f32)
}

/// File-path variant: decodes the WAV (or any supported format) via the same
/// pipeline `analyze_tracks` uses, then measures integrated LUFS. Convenience
/// for contract tests that want to read back the rendered output's loudness.
pub fn measure_integrated_lufs_at_path(path: &Path) -> CommandResult<f32> {
    let pcm = crate::decode::decode_full(path)?;
    measure_integrated_lufs(&pcm.samples, pcm.sample_rate, pcm.channels)
}

#[tauri::command]
pub async fn render_track_preview(
    track_id: TrackId,
    track_path: String,
    settings: MasteringSettings,
    app: tauri::AppHandle,
) -> CommandResult<RenderJob> {
    let out_dir = render_output_dir(&app, RenderKind::Preview)?;
    let track_id_for_progress = track_id.clone();
    let app_for_progress = app.clone();
    let on_progress = move |fraction: f32| {
        let _ = app_for_progress.emit(
            "render:progress",
            RenderProgress {
                track_id: track_id_for_progress.clone(),
                kind: RenderKind::Preview,
                fraction,
            },
        );
    };
    mastering_render_with_progress(
        track_id,
        Path::new(&track_path),
        &settings,
        &out_dir,
        RenderKind::Preview,
        Some(&on_progress),
        None,
    )
}

#[tauri::command]
pub async fn render_track_master(
    track_id: TrackId,
    track_path: String,
    settings: MasteringSettings,
    output_path: Option<String>,
    app: tauri::AppHandle,
) -> CommandResult<RenderJob> {
    let out_dir = render_output_dir(&app, RenderKind::Master)?;
    let explicit_output_path = output_path.as_deref().map(Path::new);
    let track_id_for_progress = track_id.clone();
    let app_for_progress = app.clone();
    let on_progress = move |fraction: f32| {
        let _ = app_for_progress.emit(
            "render:progress",
            RenderProgress {
                track_id: track_id_for_progress.clone(),
                kind: RenderKind::Master,
                fraction,
            },
        );
    };
    mastering_render_with_progress(
        track_id,
        Path::new(&track_path),
        &settings,
        &out_dir,
        RenderKind::Master,
        Some(&on_progress),
        explicit_output_path,
    )
}

#[tauri::command]
pub async fn render_album_master(
    request: AlbumRenderRequest,
    output_dir: Option<String>,
    app: tauri::AppHandle,
) -> CommandResult<RenderJob> {
    if request.tracks.is_empty() {
        return Err(CommandError::Other("album has no tracks".to_string()));
    }
    let out_dir = match output_dir {
        Some(path) => explicit_output_dir(Path::new(&path))?,
        None => render_output_dir(&app, RenderKind::Album)?,
    };
    // The first track's id is the "representative" id on the progress events.
    // Frontend treats the album bar as one unit, so it just needs to know
    // "this is the album render" via `kind = Album` — the track_id field is
    // populated for consistency with the per-track progress payload shape.
    let representative_id = request
        .tracks
        .first()
        .map(|t| t.id.clone())
        .unwrap_or_else(|| TrackId("album".to_string()));
    let app_for_progress = app.clone();
    let on_progress = move |fraction: f32| {
        let _ = app_for_progress.emit(
            "render:progress",
            RenderProgress {
                track_id: representative_id.clone(),
                kind: RenderKind::Album,
                fraction,
            },
        );
    };
    album_render_with_progress(&request, &out_dir, Some(&on_progress))
}

pub fn album_render(req: &AlbumRenderRequest, out_dir: &Path) -> CommandResult<RenderJob> {
    album_render_with_progress(req, out_dir, None)
}

/// Same as `album_render` but accepts an optional progress callback. Reports
/// `(track_index + within_track_fraction) / total_tracks`, where the inner
/// fraction comes from processing each track in 4096-frame chunks (same
/// granularity as `mastering_render_with_progress`). Fires `cb(0.0)` once at
/// the start and `cb(1.0)` once at the end, with monotonic-non-decreasing
/// values in between — matches the per-track contract so the frontend's
/// existing progress-bar wiring needs no changes.
pub fn album_render_with_progress(
    req: &AlbumRenderRequest,
    out_dir: &Path,
    on_progress: Option<&dyn Fn(f32)>,
) -> CommandResult<RenderJob> {
    let bit_depth = req.album_intent.effective_bit_depth();
    let album_path = unique_album_path(out_dir)?;

    let mut album_writer: Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>> = None;
    let mut common_sr: u32 = 0;
    let mut common_channels: u16 = 0;
    let mut individual_paths: Vec<String> = Vec::with_capacity(req.tracks.len());
    let mut track_ids: Vec<TrackId> = Vec::with_capacity(req.tracks.len());

    let total_tracks = req.tracks.len();
    if let Some(cb) = on_progress {
        cb(0.0);
    }

    for (i, input) in req.tracks.iter().enumerate() {
        let path = Path::new(&input.path);
        if crate::files::has_parent_dir_component(path) {
            return Err(CommandError::InvalidPath(format!(
                "path traversal not allowed: {}",
                input.path
            )));
        }
        if !path.exists() {
            return Err(CommandError::Io(format!(
                "source not found: {}",
                input.path
            )));
        }
        let pcm = crate::decode::decode_full(path)?;
        if pcm.samples.is_empty() {
            return Err(CommandError::Decode(format!(
                "no samples decoded from {}",
                input.path
            )));
        }

        if i == 0 {
            common_sr = pcm.sample_rate;
            common_channels = pcm.channels.max(1);
            let spec = wav_spec(common_channels, common_sr, bit_depth)?;
            album_writer = Some(
                hound::WavWriter::create(&album_path, spec)
                    .map_err(|e| CommandError::Io(e.to_string()))?,
            );
        } else if pcm.sample_rate != common_sr {
            return Err(CommandError::Other(format!(
                "album sample-rate mismatch on {}: {} Hz vs album {} Hz (Phase 11 will add resampling)",
                input.path, pcm.sample_rate, common_sr
            )));
        } else if pcm.channels != common_channels {
            return Err(CommandError::Other(format!(
                "album channel mismatch on {}: {} ch vs album {} ch",
                input.path, pcm.channels, common_channels
            )));
        }

        let settings = req
            .per_track_overrides
            .as_ref()
            .and_then(|m| m.get(input.id.as_str()))
            .unwrap_or(&req.album_intent);
        let mut render_settings = settings.clone();
        render_settings.volume_match = false;
        let mut samples = pcm.samples;
        let channels_usize = pcm.channels.max(1) as usize;
        let mut chain =
            crate::dsp::MasteringChain::new(pcm.sample_rate, channels_usize, &render_settings);

        // Chunk the per-track render so we can fire sub-track progress events
        // — matches `mastering_render_with_progress`'s 4096-frame granularity.
        // Chain state (limiter lookahead, biquad memory) flows correctly across
        // chunk boundaries because we reuse the same `chain` instance.
        const CHUNK_FRAMES: usize = 4096;
        let chunk_samples = CHUNK_FRAMES * channels_usize;
        let track_total = samples.len();
        let mut processed = 0;
        while processed < track_total {
            let end = (processed + chunk_samples).min(track_total);
            chain.process_interleaved(&mut samples[processed..end], channels_usize);
            processed = end;
            if let Some(cb) = on_progress {
                let within_track = processed as f32 / track_total.max(1) as f32;
                let overall = (i as f32 + within_track) / total_tracks.max(1) as f32;
                cb(overall.min(1.0));
            }
        }

        // B5: per-track ceiling-bounded LUFS landing. Pre-B5 this path
        // skipped LUFS landing entirely — album-simple exports rendered
        // at whatever the chain happened to produce, ignoring the user's
        // delivery-profile / advanced.lufs_offset_db target. Now routes
        // through the shared helper alongside the other two render paths.
        measure_and_apply_ceiling_bounded_landing(
            &mut samples,
            pcm.sample_rate,
            pcm.channels,
            &render_settings,
        )?;

        let individual = unique_output_path(out_dir, path, &input.id, RenderKind::Master)?;
        write_wav(&individual, &samples, pcm.sample_rate, pcm.channels, bit_depth)?;
        individual_paths.push(individual.to_string_lossy().to_string());
        track_ids.push(input.id.clone());

        let writer = album_writer.as_mut().expect("album writer initialized");
        write_samples_into_writer(writer, &samples, bit_depth)?;
    }

    album_writer
        .ok_or_else(|| CommandError::Other("no album writer created".to_string()))?
        .finalize()
        .map_err(|e| CommandError::Io(e.to_string()))?;

    let mut output_paths = Vec::with_capacity(individual_paths.len() + 1);
    output_paths.push(album_path.to_string_lossy().to_string());
    output_paths.extend(individual_paths);

    if let Some(cb) = on_progress {
        cb(1.0);
    }

    Ok(RenderJob {
        id: uuid::Uuid::new_v4().to_string(),
        kind: RenderKind::Album,
        target_tracks: track_ids,
        status: JobStatus::Done,
        progress: 1.0,
        started_at_iso: now_iso(),
        output_paths,
        // Album-master measurements are not yet plumbed; the album writer
        // streams per-track segments through the chain and concatenates them,
        // so a single post-render measurement requires either reading the
        // composed file back or adding an EbuR128 collector that spans every
        // segment. Tracked separately from the Codex audit P0 fix.
        measurements: None,
    })
}

// ============================================================================
// Phase B Step 3: AlbumPlan-driven render path.
//
// Consumes an AlbumPlan + per-track settings + per-track source paths and
// produces:
//   1. NN per-track WAVs named NN-<sanitized_title>.wav
//   2. one continuous album.wav with TransitionSpec silence between tracks
//   3. manifest.json documenting the plan + per-track output paths +
//      post-render measured integrated LUFS for each track
//
// The per-track render reuses the existing chunked-chain pipeline from
// `album_render_with_progress`, but each track's `MasteringSettings` is
// shadowed by the plan's `arc_lufs_offset_db` (added to the effective
// LUFS target) and `intensity_scale` (multiplied onto `settings.intensity`).
//
// Sample-rate / channel-count mismatches between tracks fail with a
// clear error — resampling is deferred to a future phase.
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AlbumTrackRenderInput {
    pub track_id: TrackId,
    pub source_path: String,
    pub settings: MasteringSettings,
}

#[derive(Debug, Deserialize)]
pub struct AlbumPlanRenderRequest {
    pub plan: AlbumPlan,
    pub tracks: Vec<AlbumTrackRenderInput>,
}

#[derive(Debug, Serialize, Clone)]
pub struct AlbumTrackRenderRecord {
    pub track_id: TrackId,
    pub position: u32,
    pub output_path: String,
    pub measured_lufs: f32,
}

#[derive(Debug, Serialize, Clone)]
pub struct AlbumRenderReport {
    pub album_wav_path: String,
    pub manifest_path: String,
    pub tracks: Vec<AlbumTrackRenderRecord>,
}

#[derive(Debug, Serialize)]
struct AlbumManifest<'a> {
    plan: &'a AlbumPlan,
    rendered_at_iso: String,
    sample_rate: u32,
    channels: u16,
    bit_depth: u16,
    album_wav_path: &'a str,
    tracks: &'a [AlbumTrackRenderRecord],
}

/// Sanitize a string into a safe file-name component. Replaces any
/// character outside `[A-Za-z0-9._-]` with `_`. Empty input becomes
/// `"untitled"`.
fn sanitize_for_filename(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed: String = cleaned.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed
    }
}

/// Shadow a per-track `MasteringSettings` with the album plan's offsets:
///   * advanced.lufs_offset_db is REPLACED with
///     `effective_target_lufs() + arc_lufs_offset_db` so the per-track
///     render lands at the arc-modulated target.
///   * intensity is multiplied by intensity_scale (clamped to [0, 1.5]).
fn apply_album_shadow(
    settings: &MasteringSettings,
    entry: &AlbumTrackEntry,
    album_intensity: f32,
    curve_value: f32,
    energy_density: f32,
) -> MasteringSettings {
    let mut shadowed = settings.clone();
    let base_target = shadowed
        .effective_target_lufs()
        .unwrap_or(-14.0);
    shadowed.advanced.lufs_offset_db =
        Some(base_target + entry.arc_lufs_offset_db);
    shadowed.intensity =
        (shadowed.intensity * entry.intensity_scale).clamp(0.0, 1.5);

    // Phase B+ Step 7: apply the per-character mastering bias on top of
    // the user's per-track settings. EQ band offsets add to the existing
    // user EQ; width / warmth coerce None to a neutral baseline (1.0 /
    // 0.0) before the offset lands; intensity gets a final bias add then
    // re-clamp.
    let bias = crate::album::mastering_bias_for(
        entry.album_character,
        energy_density,
        curve_value,
        album_intensity,
    );
    shadowed.eq_low_db += bias.low_end_db;
    shadowed.eq_low_mid_db += bias.low_mid_db;
    shadowed.eq_mid_db += bias.presence_db;
    shadowed.eq_high_db += bias.air_db;
    if bias.width_offset.abs() > 1.0e-4 {
        let base_width = shadowed.advanced.width.unwrap_or(1.0);
        shadowed.advanced.width = Some((base_width + bias.width_offset).clamp(0.0, 2.0));
    }
    if bias.warmth_offset.abs() > 1.0e-4 {
        let base_warmth = shadowed.advanced.warmth.unwrap_or(0.0);
        shadowed.advanced.warmth = Some((base_warmth + bias.warmth_offset).clamp(0.0, 1.0));
    }
    shadowed.intensity =
        (shadowed.intensity + bias.intensity_offset).clamp(0.0, 1.5);

    shadowed
}

pub fn render_album_plan_impl(
    request: &AlbumPlanRenderRequest,
    out_dir: &Path,
    on_progress: Option<&dyn Fn(f32)>,
) -> CommandResult<AlbumRenderReport> {
    if request.plan.tracks.is_empty() {
        return Err(CommandError::Other(
            "AlbumPlan has no tracks".to_string(),
        ));
    }
    // Lookup: TrackId → (source_path, settings).
    let settings_by_id: std::collections::HashMap<&str, &AlbumTrackRenderInput> =
        request
            .tracks
            .iter()
            .map(|t| (t.track_id.as_str(), t))
            .collect();

    let bit_depth = request
        .plan
        .tracks
        .first()
        .and_then(|t| settings_by_id.get(t.track_id.as_str()))
        .map(|input| input.settings.effective_bit_depth())
        .unwrap_or(24);

    std::fs::create_dir_all(out_dir)
        .map_err(|e| CommandError::Io(e.to_string()))?;

    let total_tracks = request.plan.tracks.len();
    if let Some(cb) = on_progress {
        cb(0.0);
    }

    // Two passes:
    //   Pass 1 — decode + render each track into samples in memory, write
    //   the per-track WAV with NN-<title>.wav name, measure post-render
    //   LUFS, and remember the rendered samples + transition spec for the
    //   continuous writer in pass 2. Memory cost is the full album in f32;
    //   for a typical 60-min album at 48k stereo that's ~1.3 GB which is
    //   acceptable on modern desktop. Future optimization can stream
    //   directly without staging.
    //
    //   Pass 2 — open the album writer, stream each track's samples in,
    //   inject Gap silence frames per TransitionSpec, finalize.
    let mut rendered_samples: Vec<Vec<f32>> = Vec::with_capacity(total_tracks);
    let mut track_records: Vec<AlbumTrackRenderRecord> = Vec::with_capacity(total_tracks);
    let mut common_sr: u32 = 0;
    let mut common_channels: u16 = 0;

    for (i, entry) in request.plan.tracks.iter().enumerate() {
        let input = settings_by_id
            .get(entry.track_id.as_str())
            .copied()
            .ok_or_else(|| {
                CommandError::Other(format!(
                    "AlbumPlan references track_id {} but no settings/path was provided",
                    entry.track_id.as_str()
                ))
            })?;
        let path = Path::new(&input.source_path);
        if !path.exists() {
            return Err(CommandError::Io(format!(
                "source not found: {}",
                input.source_path
            )));
        }
        let pcm = crate::decode::decode_full(path)?;
        if pcm.samples.is_empty() {
            return Err(CommandError::Decode(format!(
                "no samples decoded from {}",
                input.source_path
            )));
        }
        if i == 0 {
            common_sr = pcm.sample_rate;
            common_channels = pcm.channels.max(1);
        } else if pcm.sample_rate != common_sr {
            return Err(CommandError::Other(format!(
                "album sample-rate mismatch on {}: {} Hz vs album {} Hz (resampling not yet supported)",
                input.source_path, pcm.sample_rate, common_sr
            )));
        } else if pcm.channels != common_channels {
            return Err(CommandError::Other(format!(
                "album channel mismatch on {}: {} ch vs album {} ch",
                input.source_path, pcm.channels, common_channels
            )));
        }

        // Per-track curve value for the per-character mastering bias.
        // For Preset arcs we resample the 6-point curve to actual track
        // count; for Custom arcs we use a neutral 0.5 (no curve-driven
        // air-band swing in the bias).
        let curve_value = match &request.plan.arc {
            AlbumArc::Preset { preset } => {
                let curve =
                    crate::album::resample_arc_curve(preset.curve(), total_tracks);
                curve.get(i).copied().unwrap_or(0.5)
            }
            AlbumArc::Custom { .. } => 0.5,
        };
        // B1: compute per-track energy density from the decoded PCM so the
        // album-arc character-bias presence-band energy-gate uses the same
        // signal as the analysis path. Pre-B1 this was hardcoded to 0.5,
        // dead-coding the gate in the album EXPORT path while
        // `analyze_tracks` computed real values.
        //
        // Four measurements: integrated LUFS, 6-band spectral balance,
        // dynamic range (p95-p10), transient flux. Falls back to 0.5
        // (the prior literal, treated as "neutral") if any input is
        // unavailable — matches `compute_energy_density_score`'s contract.
        let energy_density_score = {
            let lufs = measure_integrated_lufs(
                &pcm.samples,
                pcm.sample_rate,
                pcm.channels,
            )
            .unwrap_or(-30.0);
            let spec6 = compute_spectral_balance_6band(
                &pcm.samples,
                pcm.sample_rate,
                pcm.channels as usize,
            );
            let dr = compute_dynamic_range_p95_p10(
                &pcm.samples,
                pcm.sample_rate,
                pcm.channels as usize,
            );
            let tflux = compute_transient_flux(
                &pcm.samples,
                pcm.sample_rate,
                pcm.channels as usize,
            );
            compute_energy_density_score(lufs, spec6.as_ref(), dr, tflux)
        };
        let energy_density = energy_density_score.unwrap_or(0.5);
        let shadowed = apply_album_shadow(
            &input.settings,
            entry,
            request.plan.intensity,
            curve_value,
            energy_density,
        );
        let mut shadowed = shadowed;
        shadowed.volume_match = false;
        let mut samples = pcm.samples;
        let channels_usize = pcm.channels.max(1) as usize;
        let mut chain = crate::dsp::MasteringChain::new(
            pcm.sample_rate,
            channels_usize,
            &shadowed,
        );
        const CHUNK_FRAMES: usize = 4096;
        let chunk_samples = CHUNK_FRAMES * channels_usize;
        let track_total = samples.len();
        let mut processed = 0;
        while processed < track_total {
            let end = (processed + chunk_samples).min(track_total);
            chain.process_interleaved(
                &mut samples[processed..end],
                channels_usize,
            );
            processed = end;
            if let Some(cb) = on_progress {
                let within_track = processed as f32 / track_total.max(1) as f32;
                let overall = (i as f32 + within_track) / total_tracks.max(1) as f32;
                cb(overall.min(1.0));
            }
        }

        // Per-track ceiling-bounded LUFS landing on the album-plan
        // path. `shadowed.effective_target_lufs()` is the arc-modulated
        // target (per-track LUFS offset baked into the shadow), so each
        // track lands at its arc-curve-determined target rather than
        // the raw album-intent target — preserving the album-arc story.
        // The B6 ceiling-bounded math is shared with the track-export
        // and album-simple paths via the helper.
        measure_and_apply_ceiling_bounded_landing(
            &mut samples,
            pcm.sample_rate,
            pcm.channels,
            &shadowed,
        )?;

        // Per-track WAV named NN-<sanitized_title>.wav.
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("track");
        let safe = sanitize_for_filename(stem);
        let per_track_name = format!("{:02}-{}.wav", entry.position, safe);
        let per_track_path = out_dir.join(&per_track_name);
        write_wav(
            &per_track_path,
            &samples,
            pcm.sample_rate,
            pcm.channels,
            bit_depth,
        )?;

        let measured_lufs =
            measure_integrated_lufs(&samples, pcm.sample_rate, pcm.channels)?;
        track_records.push(AlbumTrackRenderRecord {
            track_id: entry.track_id.clone(),
            position: entry.position,
            output_path: per_track_path.to_string_lossy().to_string(),
            measured_lufs,
        });
        rendered_samples.push(samples);
    }

    // Pass 2 — assemble the continuous album.wav, inserting silence
    // frames per TransitionSpec.
    let album_path = unique_album_path(out_dir)?;
    let spec = wav_spec(common_channels, common_sr, bit_depth)?;
    let mut album_writer =
        hound::WavWriter::create(&album_path, spec).map_err(|e| CommandError::Io(e.to_string()))?;
    for (i, samples) in rendered_samples.iter().enumerate() {
        write_samples_into_writer(&mut album_writer, samples, bit_depth)?;
        if i + 1 < rendered_samples.len() {
            // Transition slot between track i and track i+1.
            if let Some(t) = request.plan.transitions.get(i) {
                if matches!(t.kind, TransitionKind::Gap) {
                    let gap_seconds = t.duration_seconds.clamp(0.0, 5.0);
                    let gap_frames = (gap_seconds * common_sr as f32) as usize;
                    let gap_samples = gap_frames * common_channels as usize;
                    let zeros = vec![0.0_f32; gap_samples];
                    write_samples_into_writer(&mut album_writer, &zeros, bit_depth)?;
                }
            }
        }
    }
    album_writer
        .finalize()
        .map_err(|e| CommandError::Io(e.to_string()))?;

    // Manifest.
    let manifest_path = out_dir.join("manifest.json");
    let manifest = AlbumManifest {
        plan: &request.plan,
        rendered_at_iso: now_iso(),
        sample_rate: common_sr,
        channels: common_channels,
        bit_depth,
        album_wav_path: &album_path.to_string_lossy(),
        tracks: &track_records,
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| CommandError::Other(format!("manifest serde: {e}")))?;
    std::fs::write(&manifest_path, manifest_json)
        .map_err(|e| CommandError::Io(e.to_string()))?;

    if let Some(cb) = on_progress {
        cb(1.0);
    }

    Ok(AlbumRenderReport {
        album_wav_path: album_path.to_string_lossy().to_string(),
        manifest_path: manifest_path.to_string_lossy().to_string(),
        tracks: track_records,
    })
}

#[derive(Debug, Deserialize)]
pub struct PlanAlbumRequest {
    pub title: String,
    pub analyses: Vec<AnalysisResult>,
    pub durations: Vec<f64>,
    pub arc: AlbumArc,
    pub intensity: f32,
}

/// Phase B Step 4: thin Tauri wrapper around `album::build_album_plan`.
/// Lets the frontend pick (arc, intensity) and immediately receive the
/// per-track plan without duplicating the math in TypeScript.
#[tauri::command]
pub async fn plan_album(request: PlanAlbumRequest) -> CommandResult<AlbumPlan> {
    let refs: Vec<&AnalysisResult> = request.analyses.iter().collect();
    Ok(crate::album::build_album_plan(
        request.title,
        &refs,
        &request.durations,
        request.arc,
        request.intensity,
    ))
}

#[tauri::command]
pub async fn render_album_plan(
    request: AlbumPlanRenderRequest,
    output_dir: Option<String>,
    app: tauri::AppHandle,
) -> CommandResult<AlbumRenderReport> {
    let out_dir = match output_dir {
        Some(path) => explicit_output_dir(Path::new(&path))?,
        None => render_output_dir(&app, RenderKind::Album)?,
    };
    let app_for_progress = app.clone();
    let on_progress = move |fraction: f32| {
        let _ = app_for_progress.emit(
            "render:progress",
            RenderProgress {
                track_id: TrackId(String::new()),
                kind: RenderKind::Album,
                fraction,
            },
        );
    };
    render_album_plan_impl(&request, &out_dir, Some(&on_progress))
}

fn unique_album_path(out_dir: &Path) -> CommandResult<PathBuf> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let candidate = out_dir.join(format!("album_continuous_{ts}.wav"));
    if !candidate.exists() {
        return Ok(candidate);
    }
    for n in 1..1000 {
        let alt = out_dir.join(format!("album_continuous_{ts}_{n}.wav"));
        if !alt.exists() {
            return Ok(alt);
        }
    }
    Err(CommandError::Io(
        "could not generate unique album path".to_string(),
    ))
}

pub fn render_output_dir(app: &tauri::AppHandle, kind: RenderKind) -> CommandResult<PathBuf> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| CommandError::Other(format!("app_data_dir: {e}")))?;
    let leaf = match kind {
        RenderKind::Preview => "previews",
        RenderKind::Master => "masters",
        RenderKind::Album => "albums",
    };
    let dir = app_data.join("renders").join(leaf);
    std::fs::create_dir_all(&dir).map_err(|e| CommandError::Io(e.to_string()))?;
    Ok(dir)
}

pub fn mastering_render(
    track_id: TrackId,
    source_path: &Path,
    settings: &MasteringSettings,
    out_dir: &Path,
    kind: RenderKind,
) -> CommandResult<RenderJob> {
    mastering_render_with_progress(track_id, source_path, settings, out_dir, kind, None, None)
}

pub fn mastering_render_to_path(
    track_id: TrackId,
    source_path: &Path,
    settings: &MasteringSettings,
    out_dir: &Path,
    kind: RenderKind,
    output_path: &Path,
) -> CommandResult<RenderJob> {
    mastering_render_with_progress(
        track_id,
        source_path,
        settings,
        out_dir,
        kind,
        None,
        Some(output_path),
    )
}

/// Same as `mastering_render` but accepts an optional progress callback that
/// fires after each ~4096-frame chunk with the current 0.0–1.0 fraction.
/// Phase 12.1 perf — `render_track_master` / `render_track_preview` /
/// `render_album_master` thread an AppHandle-emitting closure through here so
/// the frontend can render a real progress bar instead of an indeterminate
/// "Rendering…" message. Contract tests pass `None` and ignore progress.
pub fn mastering_render_with_progress(
    track_id: TrackId,
    source_path: &Path,
    settings: &MasteringSettings,
    out_dir: &Path,
    kind: RenderKind,
    on_progress: Option<&dyn Fn(f32)>,
    output_path: Option<&Path>,
) -> CommandResult<RenderJob> {
    let source_path_str = source_path.to_string_lossy().to_string();
    if source_path_str.is_empty() {
        return Err(CommandError::InvalidPath("empty path".to_string()));
    }
    if crate::files::has_parent_dir_component(source_path) {
        return Err(CommandError::InvalidPath(format!(
            "path traversal not allowed: {source_path_str}"
        )));
    }
    if !source_path.exists() {
        return Err(CommandError::Io(format!(
            "source file not found: {source_path_str}"
        )));
    }

    let pcm = crate::decode::decode_full(source_path)?;
    if pcm.samples.is_empty() {
        return Err(CommandError::Decode(
            "no samples decoded from source".to_string(),
        ));
    }
    let channels = pcm.channels as usize;
    let channels_max = channels.max(1);
    let mut samples = pcm.samples;
    let mut render_settings = settings.clone();
    render_settings.volume_match = false;
    let mut chain =
        crate::dsp::MasteringChain::new(pcm.sample_rate, channels_max, &render_settings);

    // Process in 4096-frame chunks (~93 ms at 44.1 kHz) so progress callbacks
    // fire ~10 times per second. The chain's per-frame state (limiter
    // lookahead, biquad memory) flows through chunk boundaries because we
    // call into the same `chain` instance for each chunk.
    const CHUNK_FRAMES: usize = 4096;
    let chunk_samples = CHUNK_FRAMES * channels_max;
    let total_samples = samples.len();
    let mut processed = 0;
    if let Some(cb) = on_progress {
        cb(0.0);
    }
    while processed < total_samples {
        let end = (processed + chunk_samples).min(total_samples);
        chain.process_interleaved(&mut samples[processed..end], channels_max);
        processed = end;
        if let Some(cb) = on_progress {
            let fraction = processed as f32 / total_samples.max(1) as f32;
            cb(fraction.min(1.0));
        }
    }

    // Single full BS.1770 pass over the post-chain samples — used both to
    // decide LUFS landing and to populate the rendered-output measurements
    // for the export receipt (Codex audit 2026-05-13 P0: the receipt must
    // describe the rendered output, not the source analysis).
    //
    // We measure once and shift the result mathematically if landing applies.
    // Under a uniform linear gain `g`, integrated LUFS and true-peak both
    // shift by exactly `20·log10(g)` dB, and LRA (a range between gated
    // loudness percentiles) is preserved. So we never need to re-run the
    // ~25 MB-per-track ebur128 pass after scaling.
    let channels_u32 = u32::from(pcm.channels.max(1));
    let mut ebu = EbuR128::new(
        channels_u32,
        pcm.sample_rate,
        Mode::I | Mode::LRA | Mode::TRUE_PEAK,
    )
    .map_err(|e| CommandError::Render(format!("ebur128 init: {e}")))?;
    ebu.add_frames_f32(&samples)
        .map_err(|e| CommandError::Render(format!("ebur128 feed: {e}")))?;
    let mut measured_lufs = sanitize_lufs(
        ebu.loudness_global()
            .map_err(|e| CommandError::Render(format!("ebur128 global: {e}")))?
            as f32,
    );
    let lra = ebu
        .loudness_range()
        .map_err(|e| CommandError::Render(format!("ebur128 lra: {e}")))?
        as f32;
    let mut peak_lin: f64 = 0.0;
    for ch in 0..channels_u32 {
        let tp = ebu
            .true_peak(ch)
            .map_err(|e| CommandError::Render(format!("ebur128 tp: {e}")))?;
        if tp > peak_lin {
            peak_lin = tp;
        }
    }
    let mut measured_true_peak_dbtp = if peak_lin > 0.0 {
        (20.0 * peak_lin.log10()) as f32
    } else {
        -60.0
    };

    // Ceiling-bounded LUFS landing. Routes through the shared helper
    // with the LUFS+TP we already measured for the receipt. The
    // helper returns the applied delta in dB so we can shift the
    // tracked measurements (which feed `RenderedMeasurements`) in
    // lockstep — under a uniform linear gain, integrated LUFS and
    // true-peak both shift by exactly the same dB amount, so no
    // second ebur128 pass is needed.
    if let Some(target_lufs) = render_settings.effective_target_lufs() {
        let ceiling_dbtp = render_settings.effective_ceiling_dbtp();
        let applied_delta_db = apply_ceiling_bounded_landing_with_measurements(
            &mut samples,
            measured_lufs,
            measured_true_peak_dbtp,
            target_lufs,
            ceiling_dbtp,
        );
        if applied_delta_db != 0.0 {
            measured_lufs += applied_delta_db;
            measured_true_peak_dbtp += applied_delta_db;
        }
    }

    let bit_depth = render_settings.effective_bit_depth();
    let measurements = RenderedMeasurements {
        lufs_integrated: measured_lufs,
        true_peak_dbtp: measured_true_peak_dbtp,
        dynamic_range_lu: if lra.is_finite() { lra } else { 0.0 },
        sample_rate: pcm.sample_rate,
        bit_depth,
    };
    let out_path = match output_path {
        Some(path) => explicit_output_path(path)?,
        None => unique_output_path(out_dir, source_path, &track_id, kind)?,
    };
    write_wav(&out_path, &samples, pcm.sample_rate, pcm.channels, bit_depth)?;
    if let Some(cb) = on_progress {
        cb(1.0);
    }

    Ok(RenderJob {
        id: uuid::Uuid::new_v4().to_string(),
        kind,
        target_tracks: vec![track_id],
        status: JobStatus::Done,
        progress: 1.0,
        started_at_iso: now_iso(),
        output_paths: vec![out_path.to_string_lossy().to_string()],
        measurements: Some(measurements),
    })
}

fn explicit_output_path(path: &Path) -> CommandResult<PathBuf> {
    if path.as_os_str().is_empty() {
        return Err(CommandError::InvalidPath("empty output path".to_string()));
    }
    if path.file_name().is_none() {
        return Err(CommandError::InvalidPath(format!(
            "output path must include a file name: {}",
            path.to_string_lossy()
        )));
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| CommandError::Io(e.to_string()))?;
        }
    }
    Ok(path.to_path_buf())
}

fn explicit_output_dir(path: &Path) -> CommandResult<PathBuf> {
    if path.as_os_str().is_empty() {
        return Err(CommandError::InvalidPath("empty output directory".to_string()));
    }
    std::fs::create_dir_all(path).map_err(|e| CommandError::Io(e.to_string()))?;
    Ok(path.to_path_buf())
}

fn unique_output_path(
    out_dir: &Path,
    source: &Path,
    track_id: &TrackId,
    kind: RenderKind,
) -> CommandResult<PathBuf> {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("track");
    let kind_tag = match kind {
        RenderKind::Preview => "preview",
        RenderKind::Master => "master",
        RenderKind::Album => "album",
    };
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let id_short = track_id.as_str().chars().take(8).collect::<String>();
    let filename = format!("{stem}__{kind_tag}__{id_short}__{ts}.wav");
    let candidate = out_dir.join(&filename);
    if !candidate.exists() {
        return Ok(candidate);
    }
    for n in 1..1000 {
        let alt = out_dir.join(format!(
            "{stem}__{kind_tag}__{id_short}__{ts}__{n}.wav"
        ));
        if !alt.exists() {
            return Ok(alt);
        }
    }
    Err(CommandError::Io(
        "could not generate unique output path".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_output_dir_creates_selected_album_folder() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let chosen = tmp.path().join("Album Masters").join("Round 1");

        let out_dir = explicit_output_dir(&chosen).expect("explicit output dir");

        assert_eq!(out_dir, chosen);
        assert!(out_dir.is_dir(), "selected album folder should be created");
    }

    #[test]
    fn explicit_output_dir_rejects_empty_path() {
        let err = explicit_output_dir(Path::new("")).expect_err("empty dir should fail");

        assert!(
            matches!(err, CommandError::InvalidPath(ref message) if message == "empty output directory"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn explicit_output_path_creates_parent_for_native_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let chosen = tmp.path().join("Masters").join("track master.wav");

        let out_path = explicit_output_path(&chosen).expect("explicit output path");

        assert_eq!(out_path, chosen);
        assert!(
            chosen.parent().expect("parent").is_dir(),
            "selected output parent should be created"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn explicit_output_path_creates_parent_for_windows_backslash_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let chosen = format!(
            "{}\\Masters\\track master.wav",
            tmp.path().to_string_lossy()
        );
        let chosen = PathBuf::from(chosen);

        let out_path = explicit_output_path(&chosen).expect("explicit output path");

        assert_eq!(out_path, chosen);
        assert!(
            out_path.parent().expect("parent").is_dir(),
            "Windows backslash output parent should be created"
        );
    }

    // ========================================================================
    // ceiling_bounded_landing_delta_db — mechanical gates for the shared
    // landing math now used by all four render/preview paths. Tests
    // exercise the pure math via input/output pairs so a future change
    // to the formula can't silently shift behavior on any single caller.
    // ========================================================================

    /// Downward delta applies in full (the limiter has already capped
    /// peaks at ceiling, so attenuating only moves them further away).
    #[test]
    fn ceiling_bounded_landing_downward_applies_full_delta() {
        // measured -10 LUFS, peak -1 dBTP, target -14 LUFS, ceiling -1.
        // delta = target - measured = -4. Should apply in full.
        let applied =
            ceiling_bounded_landing_delta_db(-10.0, -1.0, -14.0, -1.0);
        assert!(
            (applied - -4.0).abs() < 1.0e-6,
            "downward delta should apply in full; got {applied}"
        );
    }

    /// Upward delta applies in full when there's headroom below ceiling.
    /// Verifies the post-B6 "let the slider push upward when safe"
    /// behavior is preserved through the extraction.
    #[test]
    fn ceiling_bounded_landing_upward_uses_full_headroom_when_available() {
        // measured -23 LUFS, peak -15 dBTP, target -14, ceiling -1.
        // delta = +9; headroom = 14. Push the full +9.
        let applied =
            ceiling_bounded_landing_delta_db(-23.0, -15.0, -14.0, -1.0);
        assert!(
            (applied - 9.0).abs() < 1.0e-6,
            "upward delta should apply in full when headroom > delta; got {applied}"
        );
    }

    /// Upward delta is clamped by ceiling headroom — verifies the cap
    /// fires when the chain already pushed peaks near the ceiling.
    #[test]
    fn ceiling_bounded_landing_upward_clamped_by_ceiling_headroom() {
        // measured -10 LUFS, peak -3 dBTP, target -6, ceiling -1.
        // delta = +4; headroom = 2. Push only +2.
        let applied =
            ceiling_bounded_landing_delta_db(-10.0, -3.0, -6.0, -1.0);
        assert!(
            (applied - 2.0).abs() < 1.0e-6,
            "upward delta should clamp to ceiling headroom; got {applied}"
        );
    }

    /// Upward delta with zero headroom (post-chain peak already at
    /// ceiling) clamps to zero — no push, no change. This is the
    /// "slider feels inert on already-limiter-slammed material" case,
    /// which is the spec-correct behavior.
    #[test]
    fn ceiling_bounded_landing_upward_zero_when_no_headroom() {
        // measured -10 LUFS, peak -1 dBTP (at ceiling), target -6.
        // delta = +4; headroom = 0. Push zero.
        let applied = ceiling_bounded_landing_delta_db(-10.0, -1.0, -6.0, -1.0);
        assert_eq!(
            applied, 0.0,
            "no headroom should produce zero applied delta; got {applied}"
        );
    }

    /// Silent signal (-70 LUFS gate) bypasses landing entirely.
    /// Pre-extraction, every duplicate copy of the math had the
    /// `measured_lufs > -70.0` guard. Verifies the extracted helper
    /// inherits it.
    #[test]
    fn ceiling_bounded_landing_skips_silent_signal() {
        let applied = ceiling_bounded_landing_delta_db(-80.0, -60.0, -14.0, -1.0);
        assert_eq!(
            applied, 0.0,
            "silent signal (-70 LUFS gate) should produce zero delta; got {applied}"
        );
    }

    /// Non-finite target or measurement bypasses landing — silent
    /// guard against NaN propagation into the gain stage.
    #[test]
    fn ceiling_bounded_landing_skips_non_finite_inputs() {
        assert_eq!(
            ceiling_bounded_landing_delta_db(f32::NAN, -1.0, -14.0, -1.0),
            0.0,
            "NaN measured_lufs should produce zero delta"
        );
        assert_eq!(
            ceiling_bounded_landing_delta_db(-10.0, -1.0, f32::NAN, -1.0),
            0.0,
            "NaN target should produce zero delta"
        );
        assert_eq!(
            ceiling_bounded_landing_delta_db(-10.0, -1.0, f32::INFINITY, -1.0),
            0.0,
            "infinite target should produce zero delta"
        );
    }

    /// Near-zero delta (chain already lands at target within 1e-4 dB)
    /// produces zero so the gain multiply is skipped entirely.
    /// Prevents tiny floating-point noise from triggering a
    /// near-identity gain pass over every sample.
    #[test]
    fn ceiling_bounded_landing_skips_negligible_delta() {
        // measured -14.00005, target -14. Delta = -5e-5, abs < 1e-4.
        let applied =
            ceiling_bounded_landing_delta_db(-14.00005, -1.0, -14.0, -1.0);
        assert_eq!(
            applied, 0.0,
            "delta below the ±1e-4 dB noise threshold should produce zero; got {applied}"
        );
    }

    /// Apply-in-place returns the same delta the math core would
    /// compute and ALSO mutates the sample buffer by the corresponding
    /// linear gain. Wraps the math core's contract plus the in-place
    /// step the render paths depend on.
    #[test]
    fn apply_with_measurements_mutates_samples_and_returns_delta() {
        // Construct a sample buffer at uniform amplitude 0.5. Apply
        // a -6 dB landing (measured -10 LUFS, target -16, plenty of
        // headroom — but delta is downward so headroom doesn't bind).
        let mut samples = vec![0.5_f32; 1024];
        let applied = apply_ceiling_bounded_landing_with_measurements(
            &mut samples, -10.0, -1.0, -16.0, -1.0,
        );
        assert!(
            (applied - -6.0).abs() < 1.0e-6,
            "expected -6 dB applied delta; got {applied}"
        );
        // -6 dB linear ≈ 0.501. Each sample = 0.5 * 0.501 ≈ 0.2506.
        let expected_lin = 10.0_f32.powf(-6.0 / 20.0);
        let expected_sample = 0.5_f32 * expected_lin;
        for s in &samples {
            assert!(
                (s - expected_sample).abs() < 1.0e-5,
                "sample mutation should match the linear-gain of applied delta; \
                 got {s}, expected {expected_sample}"
            );
        }
    }

    /// Apply-in-place returns 0.0 and leaves samples untouched when
    /// the math core would no-op. Verifies the contract: callers can
    /// use `if applied != 0.0` to decide whether to mutate downstream
    /// state (e.g. the track-export receipt's tracked LUFS).
    #[test]
    fn apply_with_measurements_is_a_noop_when_delta_is_zero() {
        let mut samples = vec![0.5_f32; 32];
        let original = samples.clone();
        // Silent signal → math returns 0.
        let applied = apply_ceiling_bounded_landing_with_measurements(
            &mut samples, -80.0, -60.0, -14.0, -1.0,
        );
        assert_eq!(applied, 0.0);
        assert_eq!(samples, original, "samples must not be mutated on no-op");
    }

    /// B4: every production *_iso field now reads from `now_iso()` instead
    /// of the frozen `ISO_PLACEHOLDER`. Verifies the helper returns a
    /// real RFC 3339 timestamp near the current time, and explicitly
    /// confirms it does NOT return the placeholder. Test fixtures still
    /// use `ISO_PLACEHOLDER` for deterministic AnalysisResult construction.
    #[test]
    fn now_iso_returns_current_rfc3339_timestamp_not_placeholder() {
        let ts = now_iso();
        let parsed = chrono::DateTime::parse_from_rfc3339(&ts)
            .expect("now_iso must return a valid RFC 3339 timestamp");
        let now = chrono::Utc::now();
        let diff_seconds = (now - parsed.with_timezone(&chrono::Utc))
            .num_seconds()
            .abs();
        assert!(
            diff_seconds < 5,
            "now_iso timestamp ({ts}) should be near now (within 5 s), got {diff_seconds}s drift"
        );
        assert_ne!(
            ts, ISO_PLACEHOLDER,
            "now_iso must return a real current timestamp, not the frozen test placeholder"
        );
    }
}

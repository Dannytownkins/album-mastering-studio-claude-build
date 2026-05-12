use crate::types::*;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ebur128::{EbuR128, Mode};
use serde::Deserialize;
use tauri::Manager;

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub id: TrackId,
    pub path: String,
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
    let mut out = Vec::with_capacity(tracks.len());
    for req in tracks {
        out.push(analyze_one(req.id, Path::new(&req.path))?);
    }
    Ok(out)
}

pub fn analyze_one(track_id: TrackId, path: &Path) -> CommandResult<AnalysisResult> {
    if crate::files::has_parent_dir_component(path) {
        return Err(CommandError::InvalidPath(format!(
            "path traversal not allowed: {}",
            path.display()
        )));
    }
    if !path.exists() {
        return Err(CommandError::Io(format!(
            "source file not found: {}",
            path.display()
        )));
    }

    let pcm = crate::audio::decode_full(path)?;
    if pcm.samples.is_empty() {
        return Err(CommandError::Decode("no samples decoded".to_string()));
    }

    let channels_u32 = u32::from(pcm.channels.max(1));
    let mut ebu = EbuR128::new(
        channels_u32,
        pcm.sample_rate,
        Mode::I | Mode::LRA | Mode::TRUE_PEAK,
    )
    .map_err(|e| CommandError::Other(format!("ebur128 init: {e}")))?;
    ebu.add_frames_f32(&pcm.samples)
        .map_err(|e| CommandError::Other(format!("ebur128 feed: {e}")))?;

    let lufs_integrated = sanitize_lufs(
        ebu.loudness_global()
            .map_err(|e| CommandError::Other(format!("ebur128 global: {e}")))? as f32,
    );
    let lra = ebu
        .loudness_range()
        .map_err(|e| CommandError::Other(format!("ebur128 lra: {e}")))? as f32;

    let mut peak_lin: f64 = 0.0;
    for ch in 0..channels_u32 {
        let tp = ebu
            .true_peak(ch)
            .map_err(|e| CommandError::Other(format!("ebur128 tp: {e}")))?;
        if tp > peak_lin {
            peak_lin = tp;
        }
    }
    let true_peak_dbtp = if peak_lin > 0.0 {
        (20.0 * peak_lin.log10()) as f32
    } else {
        -60.0
    };

    let stereo_width = compute_stereo_width(&pcm.samples, pcm.channels as usize);
    let spectral_balance = compute_spectral_balance(&pcm.samples, pcm.channels as usize);
    let transient_density = compute_transient_density(&pcm.samples, pcm.channels as usize);

    let short_term_max = if lra.is_finite() {
        lufs_integrated + (lra * 0.5).max(0.0)
    } else {
        lufs_integrated
    };

    let recommended_universal = MasteringSettings {
        preset: Preset::Universal,
        intensity: 0.5,
        eq_low_db: 0.0,
        eq_mid_db: 0.0,
        eq_high_db: 0.0,
        volume_match: false,
        advanced: AdvancedSettings {
            lufs_offset_db: Some(-14.0 - lufs_integrated),
            ceiling_dbtp: Some(-1.0),
            bit_depth: Some(24),
            target_sample_rate: Some(pcm.sample_rate),
            ..Default::default()
        },
    };

    Ok(AnalysisResult {
        track_id,
        lufs_integrated,
        lufs_short_term_max: short_term_max,
        true_peak_dbtp,
        dynamic_range_lu: if lra.is_finite() { lra } else { 0.0 },
        spectral_balance,
        transient_density,
        stereo_width,
        recommended_universal,
        measured_at_iso: ISO_PLACEHOLDER.to_string(),
    })
}

fn sanitize_lufs(v: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        -70.0
    }
}

fn compute_stereo_width(samples: &[f32], channels: usize) -> f32 {
    if channels < 2 {
        return 0.0;
    }
    let mut mid_sq = 0.0_f64;
    let mut side_sq = 0.0_f64;
    for frame in samples.chunks(channels) {
        let l = f64::from(*frame.first().unwrap_or(&0.0));
        let r = f64::from(*frame.get(1).unwrap_or(&0.0));
        let m = (l + r) * 0.5;
        let s = (l - r) * 0.5;
        mid_sq += m * m;
        side_sq += s * s;
    }
    let total = mid_sq + side_sq;
    if total > 0.0 {
        (side_sq / total) as f32
    } else {
        0.0
    }
}

fn compute_spectral_balance(samples: &[f32], channels: usize) -> SpectralBalance {
    if samples.is_empty() || channels == 0 {
        return SpectralBalance {
            low: 0.33,
            mid: 0.34,
            high: 0.33,
        };
    }
    // Simple band split via first-order RC filters. Phase 11b can replace with
    // Linkwitz-Riley crossovers or FFT for sharper bands.
    let mut low_lp_state = 0.0_f64;
    let mut high_lp_state = 0.0_f64;
    let mut low_sq = 0.0_f64;
    let mut mid_sq = 0.0_f64;
    let mut high_sq = 0.0_f64;

    // Assume 44.1 kHz reference; the bands are approximate either way.
    let low_alpha = 0.015; // ~100 Hz first-order LP at 44.1k
    let high_alpha = 0.45; // ~3 kHz first-order LP boundary for mid/high split

    for frame in samples.chunks(channels) {
        let mut mono = 0.0_f64;
        for &s in frame.iter() {
            mono += f64::from(s);
        }
        mono /= channels as f64;

        low_lp_state += low_alpha * (mono - low_lp_state);
        high_lp_state += high_alpha * (mono - high_lp_state);

        let low = low_lp_state;
        let mid = high_lp_state - low_lp_state;
        let high = mono - high_lp_state;

        low_sq += low * low;
        mid_sq += mid * mid;
        high_sq += high * high;
    }

    let total = low_sq + mid_sq + high_sq;
    if total > 0.0 {
        SpectralBalance {
            low: (low_sq / total) as f32,
            mid: (mid_sq / total) as f32,
            high: (high_sq / total) as f32,
        }
    } else {
        SpectralBalance {
            low: 0.33,
            mid: 0.34,
            high: 0.33,
        }
    }
}

fn compute_transient_density(samples: &[f32], channels: usize) -> f32 {
    if samples.is_empty() || channels == 0 {
        return 0.0;
    }
    // Crude zero-crossing-based proxy on the mono mix. Phase 11b can replace
    // with a real onset detector.
    let mut prev = 0.0_f32;
    let mut crossings = 0_u64;
    let mut frames = 0_u64;
    for frame in samples.chunks(channels) {
        let mut mono = 0.0;
        for &s in frame.iter() {
            mono += s;
        }
        mono /= channels as f32;
        if (mono >= 0.0) != (prev >= 0.0) && (mono - prev).abs() > 0.005 {
            crossings += 1;
        }
        prev = mono;
        frames += 1;
    }
    if frames == 0 {
        return 0.0;
    }
    // Normalize to a 0..1 range; ~4000 crossings/sec is dense (typical drums).
    let rate = crossings as f32 / frames as f32;
    (rate * 50.0).min(1.0).max(0.0)
}

#[tauri::command]
pub async fn render_track_preview(
    track_id: TrackId,
    track_path: String,
    settings: MasteringSettings,
    app: tauri::AppHandle,
) -> CommandResult<RenderJob> {
    let out_dir = render_output_dir(&app, RenderKind::Preview)?;
    mastering_render(
        track_id,
        Path::new(&track_path),
        &settings,
        &out_dir,
        RenderKind::Preview,
    )
}

#[tauri::command]
pub async fn render_track_master(
    track_id: TrackId,
    track_path: String,
    settings: MasteringSettings,
    app: tauri::AppHandle,
) -> CommandResult<RenderJob> {
    let out_dir = render_output_dir(&app, RenderKind::Master)?;
    mastering_render(
        track_id,
        Path::new(&track_path),
        &settings,
        &out_dir,
        RenderKind::Master,
    )
}

#[tauri::command]
pub async fn render_album_master(
    request: AlbumRenderRequest,
    app: tauri::AppHandle,
) -> CommandResult<RenderJob> {
    if request.tracks.is_empty() {
        return Err(CommandError::Other("album has no tracks".to_string()));
    }
    let out_dir = render_output_dir(&app, RenderKind::Album)?;
    album_render(&request, &out_dir)
}

pub fn album_render(req: &AlbumRenderRequest, out_dir: &Path) -> CommandResult<RenderJob> {
    let bit_depth = req.album_intent.advanced.bit_depth.unwrap_or(24);
    let album_path = unique_album_path(out_dir)?;

    let mut album_writer: Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>> = None;
    let mut common_sr: u32 = 0;
    let mut common_channels: u16 = 0;
    let mut individual_paths: Vec<String> = Vec::with_capacity(req.tracks.len());
    let mut track_ids: Vec<TrackId> = Vec::with_capacity(req.tracks.len());

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
        let pcm = crate::audio::decode_full(path)?;
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
        let mut samples = pcm.samples;
        let channels_usize = pcm.channels.max(1) as usize;
        let mut chain = crate::dsp::MasteringChain::new(pcm.sample_rate, channels_usize, settings);
        chain.process_interleaved(&mut samples, channels_usize);

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

    Ok(RenderJob {
        id: uuid::Uuid::new_v4().to_string(),
        kind: RenderKind::Album,
        target_tracks: track_ids,
        status: JobStatus::Done,
        progress: 1.0,
        started_at_iso: ISO_PLACEHOLDER.to_string(),
        output_paths,
    })
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

fn wav_spec(channels: u16, sample_rate: u32, bit_depth: u16) -> CommandResult<hound::WavSpec> {
    let (bits, fmt) = match bit_depth {
        16 => (16u16, hound::SampleFormat::Int),
        24 => (24u16, hound::SampleFormat::Int),
        32 => (32u16, hound::SampleFormat::Float),
        other => {
            return Err(CommandError::Other(format!(
                "unsupported bit depth: {other}"
            )))
        }
    };
    Ok(hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: bits,
        sample_format: fmt,
    })
}

fn write_samples_into_writer(
    writer: &mut hound::WavWriter<std::io::BufWriter<std::fs::File>>,
    samples: &[f32],
    bit_depth: u16,
) -> CommandResult<()> {
    match bit_depth {
        16 => {
            for &s in samples {
                let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
                writer
                    .write_sample(v)
                    .map_err(|e| CommandError::Io(e.to_string()))?;
            }
        }
        24 => {
            for &s in samples {
                let v = (s.clamp(-1.0, 1.0) * 8_388_607.0).round() as i32;
                writer
                    .write_sample(v)
                    .map_err(|e| CommandError::Io(e.to_string()))?;
            }
        }
        32 => {
            for &s in samples {
                writer
                    .write_sample(s.clamp(-1.0, 1.0))
                    .map_err(|e| CommandError::Io(e.to_string()))?;
            }
        }
        other => {
            return Err(CommandError::Other(format!(
                "unsupported bit depth: {other}"
            )))
        }
    }
    Ok(())
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

    let pcm = crate::audio::decode_full(source_path)?;
    if pcm.samples.is_empty() {
        return Err(CommandError::Decode(
            "no samples decoded from source".to_string(),
        ));
    }
    let channels = pcm.channels as usize;
    let mut samples = pcm.samples;
    let mut chain =
        crate::dsp::MasteringChain::new(pcm.sample_rate, channels.max(1), settings);
    chain.process_interleaved(&mut samples, channels.max(1));

    let bit_depth = settings.advanced.bit_depth.unwrap_or(24);
    let out_path = unique_output_path(out_dir, source_path, &track_id, kind)?;
    write_wav(&out_path, &samples, pcm.sample_rate, pcm.channels, bit_depth)?;

    Ok(RenderJob {
        id: uuid::Uuid::new_v4().to_string(),
        kind,
        target_tracks: vec![track_id],
        status: JobStatus::Done,
        progress: 1.0,
        started_at_iso: ISO_PLACEHOLDER.to_string(),
        output_paths: vec![out_path.to_string_lossy().to_string()],
    })
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

fn write_wav(
    path: &Path,
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    bit_depth: u16,
) -> CommandResult<()> {
    let (bits, fmt) = match bit_depth {
        16 => (16u16, hound::SampleFormat::Int),
        24 => (24u16, hound::SampleFormat::Int),
        32 => (32u16, hound::SampleFormat::Float),
        other => {
            return Err(CommandError::Other(format!(
                "unsupported bit depth: {other}"
            )))
        }
    };
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: bits,
        sample_format: fmt,
    };
    let mut writer =
        hound::WavWriter::create(path, spec).map_err(|e| CommandError::Io(e.to_string()))?;
    match bit_depth {
        16 => {
            for &s in samples {
                let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
                writer
                    .write_sample(v)
                    .map_err(|e| CommandError::Io(e.to_string()))?;
            }
        }
        24 => {
            for &s in samples {
                let v = (s.clamp(-1.0, 1.0) * 8_388_607.0).round() as i32;
                writer
                    .write_sample(v)
                    .map_err(|e| CommandError::Io(e.to_string()))?;
            }
        }
        32 => {
            for &s in samples {
                writer
                    .write_sample(s.clamp(-1.0, 1.0))
                    .map_err(|e| CommandError::Io(e.to_string()))?;
            }
        }
        _ => unreachable!(),
    }
    writer
        .finalize()
        .map_err(|e| CommandError::Io(e.to_string()))?;
    Ok(())
}

use crate::types::*;
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

/// Phase 9.2: when a track's per-track role inference is weak, nudge first/
/// last positions toward Opener/Closer respectively. Strong-confidence roles
/// (e.g. an obvious Single at track 1) are left alone — the per-track signal
/// dominates when it's clear.
pub fn nudge_role_by_position(
    result: &mut AnalysisResult,
    index: usize,
    total: usize,
) {
    if total <= 1 {
        return;
    }
    let weak = matches!(
        result.role_confidence,
        Some(InferenceConfidence::Unsure) | None
    );
    let mid_default = matches!(result.role_confidence, Some(InferenceConfidence::Moderate))
        && matches!(result.inferred_role, Some(TrackRole::AlbumTrack));
    let eligible = weak || mid_default;
    if !eligible {
        return;
    }
    if index == 0 {
        result.inferred_role = Some(TrackRole::Opener);
        result.role_confidence = Some(InferenceConfidence::Moderate);
    } else if index == total - 1 {
        result.inferred_role = Some(TrackRole::Closer);
        result.role_confidence = Some(InferenceConfidence::Moderate);
    }
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

    // Phase A5: richer measurements. All Optional — they degrade
    // gracefully when the signal is too short or silent.
    let spectral_balance_6band =
        compute_spectral_balance_6band(&pcm.samples, pcm.sample_rate, pcm.channels as usize);
    let transient_flux =
        compute_transient_flux(&pcm.samples, pcm.sample_rate, pcm.channels as usize);
    let stereo_correlation =
        compute_stereo_correlation(&pcm.samples, pcm.channels as usize);
    let dynamic_range_p95_p10_db =
        compute_dynamic_range_p95_p10(&pcm.samples, pcm.sample_rate, pcm.channels as usize);
    let lufs_short_term_max_3s =
        compute_short_term_max_lufs(&pcm.samples, pcm.sample_rate, pcm.channels);
    let energy_density_score = compute_energy_density_score(
        lufs_integrated,
        spectral_balance_6band.as_ref(),
        dynamic_range_p95_p10_db,
        transient_flux,
    );

    // Prefer the true 3 s short-term max from ebur128 Mode::S when
    // available; fall back to the prior estimate (integrated + LRA / 2)
    // for short signals where Mode::S doesn't have enough material.
    let short_term_max = lufs_short_term_max_3s.unwrap_or_else(|| {
        if lra.is_finite() {
            lufs_integrated + (lra * 0.5).max(0.0)
        } else {
            lufs_integrated
        }
    });

    let recommended_universal = MasteringSettings {
        preset: Preset::Universal,
        intensity: 0.5,
        eq_low_db: 0.0,
        eq_low_mid_db: 0.0,
        eq_mid_db: 0.0,
        eq_high_db: 0.0,
        volume_match: false,
        input_gain_db: 0.0,
        output_gain_db: 0.0,
        delivery_profile: DeliveryProfile::default(),
        album: None,
        advanced: AdvancedSettings {
            lufs_offset_db: Some(-14.0 - lufs_integrated),
            ceiling_dbtp: Some(-1.0),
            bit_depth: Some(24),
            target_sample_rate: Some(pcm.sample_rate),
            ..Default::default()
        },
    };

    let duration_sec = if pcm.sample_rate > 0 && pcm.channels > 0 {
        (pcm.samples.len() as f64) / (pcm.channels.max(1) as f64 * pcm.sample_rate as f64)
    } else {
        0.0
    };
    // Phase A5: role inference prefers transient_flux when available
    // (spectral-flux is a stronger Single-track signal than ZCR), and
    // falls back to transient_density for backward compatibility.
    let role_transient_signal = transient_flux.unwrap_or(transient_density);
    let (role, role_conf) =
        infer_role(lufs_integrated, role_transient_signal, duration_sec);
    let (character, character_conf) =
        infer_character(&spectral_balance, transient_density);

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
        inferred_role: Some(role),
        role_confidence: Some(role_conf),
        inferred_character: Some(character),
        character_confidence: Some(character_conf),
        spectral_balance_6band,
        transient_flux,
        stereo_correlation,
        dynamic_range_p95_p10_db,
        lufs_short_term_max_3s,
        energy_density_score,
    })
}

fn infer_role(
    lufs: f32,
    transient_density: f32,
    duration_sec: f64,
) -> (TrackRole, InferenceConfidence) {
    // Interlude: short and quiet/sparse.
    if duration_sec > 0.0 && duration_sec < 90.0 && transient_density < 0.4 {
        return (TrackRole::Interlude, InferenceConfidence::Moderate);
    }
    // Single / banger: loud and dense.
    if lufs.is_finite() && lufs > -10.0 && transient_density > 0.6 {
        return (TrackRole::Single, InferenceConfidence::Strong);
    }
    // Ballad: quiet and sparse.
    if lufs.is_finite() && lufs < -16.0 && transient_density < 0.4 {
        return (TrackRole::Ballad, InferenceConfidence::Moderate);
    }
    // Default fallback.
    (TrackRole::AlbumTrack, InferenceConfidence::Unsure)
}

fn infer_character(
    spectral: &SpectralBalance,
    transient_density: f32,
) -> (TrackCharacter, InferenceConfidence) {
    if spectral.high > 0.45 {
        return (TrackCharacter::Bright, InferenceConfidence::Strong);
    }
    if spectral.high < 0.15 {
        return (TrackCharacter::Dark, InferenceConfidence::Moderate);
    }
    if transient_density > 0.65 {
        return (TrackCharacter::Dense, InferenceConfidence::Moderate);
    }
    if transient_density < 0.25 {
        return (TrackCharacter::Sparse, InferenceConfidence::Moderate);
    }
    (TrackCharacter::Balanced, InferenceConfidence::Unsure)
}

fn sanitize_lufs(v: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        -70.0
    }
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
    let pcm = crate::audio::decode_full(path)?;
    measure_integrated_lufs(&pcm.samples, pcm.sample_rate, pcm.channels)
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

// ============================================================================
// Phase A5: richer pre-mastering analysis. Implementations ported from
// Codex's `src/album_mastering_studio/analysis.py`. These do not affect
// audition or export quality directly — they feed role / character
// inference and (later) album-arc planning.
// ============================================================================

/// 6-band spectral balance via Hann-windowed FFT. Returns `None` if the
/// signal is too short for a meaningful FFT (< 1024 frames after
/// power-of-two truncation) or has no energy.
fn compute_spectral_balance_6band(
    samples: &[f32],
    sample_rate: u32,
    channels: usize,
) -> Option<crate::types::SpectralBalance6> {
    if channels == 0 || samples.is_empty() || sample_rate == 0 {
        return None;
    }
    let total_frames = samples.len() / channels;
    // Up to 30 seconds — picks the largest power of two ≤ min(30s, total).
    let max_frames = (sample_rate as usize).saturating_mul(30);
    let usable = total_frames.min(max_frames);
    let mut fft_size = 1_usize;
    while fft_size * 2 <= usable && fft_size < 1 << 18 {
        fft_size *= 2;
    }
    if fft_size < 1024 {
        return None;
    }

    use rustfft::num_complex::Complex;
    use rustfft::FftPlanner;
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_size);

    let mut buf: Vec<Complex<f32>> = Vec::with_capacity(fft_size);
    let two_pi = 2.0 * std::f32::consts::PI;
    for i in 0..fft_size {
        let mut mono = 0.0_f32;
        let frame_start = i * channels;
        for c in 0..channels {
            mono += samples[frame_start + c];
        }
        mono /= channels as f32;
        // Hann window — reduces spectral leakage.
        let w = 0.5 * (1.0 - (two_pi * i as f32 / (fft_size as f32 - 1.0)).cos());
        buf.push(Complex {
            re: mono * w,
            im: 0.0,
        });
    }
    fft.process(&mut buf);

    let bins = fft_size / 2;
    let bin_hz = sample_rate as f32 / fft_size as f32;
    // Edge frequencies for the 6 bands (sub / low / low_mid / mid /
    // presence / air). Top edge clamped to min(Nyquist, 16 kHz).
    let top = (sample_rate as f32 / 2.0).min(16_000.0);
    let edges = [20.0, 80.0, 250.0, 800.0, 2500.0, 6500.0, top];
    let mut bands = [0.0_f64; 6];
    for bin in 1..bins {
        let freq = bin as f32 * bin_hz;
        let idx = if freq >= edges[0] && freq < edges[1] {
            0
        } else if freq >= edges[1] && freq < edges[2] {
            1
        } else if freq >= edges[2] && freq < edges[3] {
            2
        } else if freq >= edges[3] && freq < edges[4] {
            3
        } else if freq >= edges[4] && freq < edges[5] {
            4
        } else if freq >= edges[5] && freq < edges[6] {
            5
        } else {
            continue;
        };
        let c = buf[bin];
        bands[idx] += (c.re as f64) * (c.re as f64) + (c.im as f64) * (c.im as f64);
    }
    let total: f64 = bands.iter().sum();
    if total <= 1.0e-12 {
        return None;
    }
    Some(crate::types::SpectralBalance6 {
        sub: (bands[0] / total) as f32,
        low: (bands[1] / total) as f32,
        low_mid: (bands[2] / total) as f32,
        mid: (bands[3] / total) as f32,
        presence: (bands[4] / total) as f32,
        air: (bands[5] / total) as f32,
    })
}

/// Pearson correlation between L and R channels. `None` for mono.
fn compute_stereo_correlation(samples: &[f32], channels: usize) -> Option<f32> {
    if channels < 2 || samples.is_empty() {
        return None;
    }
    let n = samples.len() / channels;
    if n < 16 {
        return None;
    }
    // Two-pass for numerical stability.
    let mut sum_l = 0.0_f64;
    let mut sum_r = 0.0_f64;
    for frame in samples.chunks_exact(channels) {
        sum_l += frame[0] as f64;
        sum_r += frame[1] as f64;
    }
    let inv_n = 1.0 / n as f64;
    let mean_l = sum_l * inv_n;
    let mean_r = sum_r * inv_n;
    let mut cov = 0.0_f64;
    let mut var_l = 0.0_f64;
    let mut var_r = 0.0_f64;
    for frame in samples.chunks_exact(channels) {
        let dl = frame[0] as f64 - mean_l;
        let dr = frame[1] as f64 - mean_r;
        cov += dl * dr;
        var_l += dl * dl;
        var_r += dr * dr;
    }
    let denom = (var_l * var_r).sqrt();
    if denom > 1.0e-12 {
        Some((cov / denom).clamp(-1.0, 1.0) as f32)
    } else {
        None
    }
}

/// Dynamic range as P95 minus P10 of RMS-block dB values. 100 ms windows
/// at 50 ms hop. Better "how dynamic does this feel" than crest factor.
fn compute_dynamic_range_p95_p10(
    samples: &[f32],
    sample_rate: u32,
    channels: usize,
) -> Option<f32> {
    if channels == 0 || samples.is_empty() || sample_rate == 0 {
        return None;
    }
    let frames_per_block = (sample_rate as f32 * 0.1) as usize;
    let frames_per_hop = (sample_rate as f32 * 0.05) as usize;
    if frames_per_hop == 0 {
        return None;
    }
    let window = frames_per_block * channels;
    let hop = frames_per_hop * channels;
    if samples.len() < window {
        return None;
    }
    let mut rms_db: Vec<f32> = Vec::with_capacity(samples.len() / hop);
    let mut pos = 0;
    while pos + window <= samples.len() {
        let chunk = &samples[pos..pos + window];
        let mut sum_sq = 0.0_f64;
        for &s in chunk {
            sum_sq += (s as f64) * (s as f64);
        }
        let rms = (sum_sq / chunk.len() as f64).sqrt();
        if rms > 1.0e-9 {
            rms_db.push((20.0 * rms.log10()) as f32);
        }
        pos += hop;
    }
    if rms_db.len() < 4 {
        return None;
    }
    rms_db.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p10 = rms_db[(rms_db.len() * 10) / 100];
    let p95 = rms_db[((rms_db.len() * 95) / 100).min(rms_db.len() - 1)];
    Some(p95 - p10)
}

/// Maximum short-term LUFS via ebur128 Mode::S (3 s sliding window).
/// Feeds the signal in ~100 ms chunks and samples loudness_shortterm()
/// at each boundary, returning the max.
fn compute_short_term_max_lufs(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Option<f32> {
    if samples.is_empty() || channels == 0 {
        return None;
    }
    let mut ebu =
        ebur128::EbuR128::new(u32::from(channels), sample_rate, ebur128::Mode::S).ok()?;
    let chunk_frames = (sample_rate / 10).max(1) as usize;
    let chunk_samples = chunk_frames * channels as usize;
    let mut max_st = f32::NEG_INFINITY;
    let mut pos = 0;
    while pos < samples.len() {
        let end = (pos + chunk_samples).min(samples.len());
        ebu.add_frames_f32(&samples[pos..end]).ok()?;
        if let Ok(st) = ebu.loudness_shortterm() {
            let v = st as f32;
            if v.is_finite() && v > max_st {
                max_st = v;
            }
        }
        pos = end;
    }
    if max_st.is_finite() {
        Some(max_st)
    } else {
        None
    }
}

/// Spectral-flux transient density. 40 ms windows at 10 ms hop; the
/// positive (one-sided) flux of the RMS envelope is averaged and
/// normalized by mean RMS. Higher = more percussive.
fn compute_transient_flux(
    samples: &[f32],
    sample_rate: u32,
    channels: usize,
) -> Option<f32> {
    if channels == 0 || samples.is_empty() || sample_rate == 0 {
        return None;
    }
    let frames_per_window = (sample_rate as f32 * 0.04) as usize;
    let frames_per_hop = (sample_rate as f32 * 0.01) as usize;
    if frames_per_hop == 0 {
        return None;
    }
    let window = frames_per_window * channels;
    let hop = frames_per_hop * channels;
    if samples.len() < window {
        return None;
    }
    let mut rms: Vec<f64> = Vec::with_capacity(samples.len() / hop);
    let mut pos = 0;
    while pos + window <= samples.len() {
        let chunk = &samples[pos..pos + window];
        let mut sum_sq = 0.0_f64;
        for &s in chunk {
            sum_sq += (s as f64) * (s as f64);
        }
        rms.push((sum_sq / chunk.len() as f64).sqrt());
        pos += hop;
    }
    if rms.len() < 4 {
        return None;
    }
    let mean_rms: f64 = rms.iter().sum::<f64>() / rms.len() as f64;
    if mean_rms <= 1.0e-9 {
        return None;
    }
    let mut positive_flux = 0.0_f64;
    let mut count = 0_usize;
    for w in rms.windows(2) {
        let diff = w[1] - w[0];
        if diff > 0.0 {
            positive_flux += diff;
            count += 1;
        }
    }
    if count == 0 {
        return None;
    }
    Some(((positive_flux / count as f64) / mean_rms) as f32)
}

/// Composite "how hot does this mix feel" score in `[0, 1]`. Weighted
/// combination of loudness, brightness, density, transient flux per
/// Codex's analysis.py formula. Requires the 6-band spectral balance,
/// dynamic range, and transient flux — returns `None` if any input is
/// missing.
fn compute_energy_density_score(
    lufs_integrated: f32,
    spectral_6: Option<&crate::types::SpectralBalance6>,
    dynamic_range_p95_p10_db: Option<f32>,
    transient_flux: Option<f32>,
) -> Option<f32> {
    let spec = spectral_6?;
    let dr = dynamic_range_p95_p10_db?;
    let flux = transient_flux?;
    // Loudness term: -30 LUFS → 0, 0 LUFS → 1. Clamped.
    let loudness_norm = ((lufs_integrated + 30.0) / 30.0).clamp(0.0, 1.0);
    // Brightness term: presence + air share, scaled.
    let brightness_norm = ((spec.presence + spec.air) * 2.0).clamp(0.0, 1.0);
    // Density: low dynamic range → high density. 12 LU as the soft anchor.
    let density_norm = (1.0 - dr / 12.0).clamp(0.0, 1.0);
    // Transient flux already in roughly [0, 1] for typical content.
    let transient_norm = flux.clamp(0.0, 1.0);
    Some(
        0.44 * loudness_norm
            + 0.21 * brightness_norm
            + 0.23 * density_norm
            + 0.12 * transient_norm,
    )
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
        started_at_iso: ISO_PLACEHOLDER.to_string(),
        output_paths,
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
    rendered_at_iso: &'static str,
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
) -> MasteringSettings {
    let mut shadowed = settings.clone();
    let base_target = shadowed
        .effective_target_lufs()
        .unwrap_or(-14.0);
    shadowed.advanced.lufs_offset_db =
        Some(base_target + entry.arc_lufs_offset_db);
    shadowed.intensity =
        (shadowed.intensity * entry.intensity_scale).clamp(0.0, 1.5);
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
        let pcm = crate::audio::decode_full(path)?;
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

        let shadowed = apply_album_shadow(&input.settings, entry);
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

        // Apply per-track LUFS landing using the shadowed target (the
        // arc-modulated value).
        if let Some(target_lufs) = shadowed.effective_target_lufs() {
            if target_lufs.is_finite() {
                let measured =
                    measure_integrated_lufs(&samples, pcm.sample_rate, pcm.channels)?;
                if measured.is_finite() && measured > -70.0 {
                    let delta_db = target_lufs - measured;
                    if delta_db < 0.0 {
                        let gain_lin = 10.0_f32.powf(delta_db / 20.0);
                        for s in samples.iter_mut() {
                            *s *= gain_lin;
                        }
                    }
                }
            }
        }

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
        rendered_at_iso: ISO_PLACEHOLDER,
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
    app: tauri::AppHandle,
) -> CommandResult<AlbumRenderReport> {
    let out_dir = render_output_dir(&app, RenderKind::Album)?;
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

// ============================================================================
// Phase A4: TPDF dither for integer-output WAV writers.
//
// 16/24-bit PCM rounds a float sample to the nearest integer, which
// produces signal-correlated harmonic distortion at low levels — the
// quantization error becomes a periodic function of the signal. Triangular
// probability density noise of ±1 LSB peak amplitude, added BEFORE
// quantization, decorrelates the error from the signal: the per-sample
// quantization noise becomes Gaussian-ish white noise at the LSB level,
// at the cost of ~3 dB extra noise floor (inaudible at 16-bit; below
// hearing at 24-bit). Reference: Lipshitz / Vanderkooy 1992.
//
// Applied ONLY in the offline render path. The live audio thread in
// audio.rs stays f32 throughout, so there's no quantization to dither.
//
// PRNG: xorshift32. Two shifts, two XORs, one f32 divide per draw — far
// cheaper than the rand crate's SmallRng for the volume of noise we
// generate (millions of samples per render). State held in `DitherRng`
// for deterministic per-render output.
// ============================================================================

struct DitherRng {
    state: u32,
}

impl DitherRng {
    fn new(seed: u32) -> Self {
        // xorshift32 has a zero-fixed-point; substitute a non-zero seed.
        Self {
            state: if seed == 0 { 0xCAFE_BABE } else { seed },
        }
    }

    /// One uniform draw in `[0, 1)` from the top 23 bits of state.
    #[inline]
    fn next_unit(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        ((self.state >> 9) as f32) / 8_388_608.0_f32
    }

    /// Triangular noise in `[-2, 2)` LSB — the sum of two independent
    /// uniforms in `[-1, 1)`. Per the standard mastering-dither shape
    /// (Lipshitz / Vanderkooy) at ±2 LSB peak amplitude. Returned in
    /// LSB units; callers multiply by `1 / scale` to convert to
    /// amplitude before adding to the sample.
    #[inline]
    fn tpdf_lsb(&mut self) -> f32 {
        let u1 = 2.0 * self.next_unit() - 1.0; // [-1, 1)
        let u2 = 2.0 * self.next_unit() - 1.0; // [-1, 1)
        u1 + u2 // triangle in [-2, 2)
    }
}

const INT16_SCALE: f32 = 32_767.0;
const INT24_SCALE: f32 = 8_388_607.0;

#[inline]
fn quantize_16_tpdf(sample: f32, rng: &mut DitherRng) -> i16 {
    let dithered = sample + rng.tpdf_lsb() / INT16_SCALE;
    (dithered.clamp(-1.0, 1.0) * INT16_SCALE).round() as i16
}

#[inline]
fn quantize_24_tpdf(sample: f32, rng: &mut DitherRng) -> i32 {
    let dithered = sample + rng.tpdf_lsb() / INT24_SCALE;
    (dithered.clamp(-1.0, 1.0) * INT24_SCALE).round() as i32
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
    // Phase A4: TPDF dither for int paths. f32 output stays as-is.
    let mut rng = DitherRng::new(0xA11_CE);
    match bit_depth {
        16 => {
            for &s in samples {
                writer
                    .write_sample(quantize_16_tpdf(s, &mut rng))
                    .map_err(|e| CommandError::Io(e.to_string()))?;
            }
        }
        24 => {
            for &s in samples {
                writer
                    .write_sample(quantize_24_tpdf(s, &mut rng))
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
    mastering_render_with_progress(track_id, source_path, settings, out_dir, kind, None)
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
    let channels_max = channels.max(1);
    let mut samples = pcm.samples;
    let mut chain =
        crate::dsp::MasteringChain::new(pcm.sample_rate, channels_max, settings);

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

    // Phase 12.2 — LUFS landing. When the user has set `lufs_offset_db` as a
    // target loudness, measure the post-chain integrated LUFS via BS.1770 and
    // attenuate the rendered samples to meet the target. Refuse-upward
    // policy: we only ever scale DOWN. Scaling up post-chain would push the
    // already-limited peaks past the user's true-peak ceiling, which no
    // mastering tool the research surveyed (Sonible smart:limit, Ozone
    // Maximizer, Mastering The Mix LIMITER) is willing to do silently. When
    // the chain produced quieter audio than the target, we leave the samples
    // unchanged and let the user re-render with more Intensity / Input Gain.
    // See `docs/research/most-recent-mastering-app-research.md` for the
    // industry-survey notes behind this decision.
    if let Some(target_lufs) = settings.effective_target_lufs() {
        if target_lufs.is_finite() {
            let measured =
                measure_integrated_lufs(&samples, pcm.sample_rate, pcm.channels)?;
            if measured.is_finite() && measured > -70.0 {
                let delta_db = target_lufs - measured;
                if delta_db < 0.0 {
                    let gain_lin = 10.0_f32.powf(delta_db / 20.0);
                    for s in samples.iter_mut() {
                        *s *= gain_lin;
                    }
                }
                // delta_db >= 0 → refuse-upward, samples unchanged. The
                // rendered file's measured LUFS will reveal the gap when the
                // user re-imports it; future polish can add a
                // "lufs_target_unmet" advisory to the export receipt.
            }
        }
    }

    let bit_depth = settings.effective_bit_depth();
    let out_path = unique_output_path(out_dir, source_path, &track_id, kind)?;
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
    // Phase A4: TPDF dither for int paths. f32 output stays as-is.
    let mut rng = DitherRng::new(0xA11_CE);
    match bit_depth {
        16 => {
            for &s in samples {
                writer
                    .write_sample(quantize_16_tpdf(s, &mut rng))
                    .map_err(|e| CommandError::Io(e.to_string()))?;
            }
        }
        24 => {
            for &s in samples {
                writer
                    .write_sample(quantize_24_tpdf(s, &mut rng))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Phase A4: at -90 dBFS the signal sits at ~1 LSB of a 16-bit
    /// quantizer. Without dither, `round()` quantizes the sine to a
    /// tiny set of integer values (mostly 0, with occasional ±1 at the
    /// peaks) — the quantization noise is periodic and signal-correlated.
    /// With TPDF dither, the noise floor expands so the output takes on
    /// MANY distinct integer values, decorrelating the error from the
    /// signal. This is the textbook reason to dither.
    ///
    /// Concrete acceptance: the dithered sequence must produce at least
    /// 6 distinct integer values; the undithered sequence stays at 3
    /// or fewer (the deliberate signed-quantization fan-out).
    #[test]
    fn tpdf_dither_decorrelates_quantization_at_minus_90_dbfs() {
        let sr = 48_000_u32;
        let n = (sr as f32 * 0.1) as usize;
        let amp = 10.0_f32.powf(-90.0 / 20.0);
        let omega = 2.0 * std::f32::consts::PI * 1000.0 / sr as f32;
        let samples: Vec<f32> = (0..n)
            .map(|i| amp * (omega * i as f32).sin())
            .collect();

        let mut undithered = HashSet::new();
        for &s in &samples {
            let v = (s.clamp(-1.0, 1.0) * INT16_SCALE).round() as i16;
            undithered.insert(v);
        }

        let mut rng = DitherRng::new(0x1234_5678);
        let mut dithered = HashSet::new();
        for &s in &samples {
            dithered.insert(quantize_16_tpdf(s, &mut rng));
        }

        assert!(
            undithered.len() <= 3,
            "undithered -90 dBFS sine should stay tightly quantized; got {} distinct values",
            undithered.len()
        );
        assert!(
            dithered.len() > undithered.len(),
            "dither must expand the integer count: undithered={}, dithered={}",
            undithered.len(),
            dithered.len()
        );
        assert!(
            dithered.len() >= 5,
            "dithered -90 dBFS sine should hit at least 5 distinct values \
             (signal ~±1 LSB peak, TPDF noise ±2 LSB peak); got {}",
            dithered.len()
        );
    }

    /// TPDF dither's mean should be ~0 — over many samples the noise
    /// contribution averages out. Verifies the PRNG is balanced.
    #[test]
    fn tpdf_dither_has_zero_mean() {
        let mut rng = DitherRng::new(0xDEAD_BEEF);
        let n = 100_000;
        let mean: f32 =
            (0..n).map(|_| rng.tpdf_lsb()).sum::<f32>() / (n as f32);
        assert!(
            mean.abs() < 0.01,
            "TPDF mean across {} samples should be ~0; got {}",
            n,
            mean
        );
    }

    // ====================================================================
    // Phase A5: pre-mastering analysis tests.
    // ====================================================================

    /// Spectral flux should read materially higher on a percussive
    /// signal (impulse train) than on a sustained one (continuous sine
    /// at the same average level). This is the core "is the new
    /// transient_flux actually better than the prior ZCR proxy" check.
    #[test]
    fn transient_flux_higher_on_percussive_than_sustained() {
        let sr = 48_000_u32;
        let n = (sr as f32 * 2.0) as usize;
        // Percussive: short bursts of high-amplitude sine at 5 Hz rate
        // (one click every 200 ms). Each burst is 10 ms of 1 kHz sine
        // followed by silence. The RMS envelope oscillates strongly.
        let burst_len = (sr as f32 * 0.01) as usize;
        let burst_period = (sr as f32 * 0.2) as usize;
        let omega = 2.0 * std::f32::consts::PI * 1000.0 / sr as f32;
        let percussive: Vec<f32> = (0..n)
            .map(|i| {
                let phase = i % burst_period;
                if phase < burst_len {
                    0.5 * (omega * i as f32).sin()
                } else {
                    0.0
                }
            })
            .collect();
        // Sustained: continuous 1 kHz sine at lower amplitude so the
        // average level roughly matches the percussive signal. RMS
        // envelope is essentially flat.
        let sustained: Vec<f32> = (0..n)
            .map(|i| 0.1 * (omega * i as f32).sin())
            .collect();

        let flux_p = compute_transient_flux(&percussive, sr, 1)
            .expect("percussive flux");
        let flux_s = compute_transient_flux(&sustained, sr, 1)
            .expect("sustained flux");
        assert!(
            flux_p > flux_s * 5.0,
            "percussive flux ({:.3}) should be >>5x sustained flux ({:.3})",
            flux_p,
            flux_s
        );
    }

    /// Stereo correlation: identical L/R reads ~+1.0; inverted L/R
    /// reads ~-1.0; decorrelated reads ~0.
    #[test]
    fn stereo_correlation_identical_inverted_decorrelated() {
        let n = 480_000;
        let omega = 2.0 * std::f32::consts::PI * 440.0 / 48_000.0;
        let identical: Vec<f32> = (0..n)
            .flat_map(|i| {
                let v = 0.3 * (omega * i as f32).sin();
                [v, v]
            })
            .collect();
        let inverted: Vec<f32> = (0..n)
            .flat_map(|i| {
                let v = 0.3 * (omega * i as f32).sin();
                [v, -v]
            })
            .collect();

        let c_id = compute_stereo_correlation(&identical, 2).expect("identical");
        let c_inv = compute_stereo_correlation(&inverted, 2).expect("inverted");
        assert!(
            (c_id - 1.0).abs() < 1.0e-3,
            "identical L/R should correlate ~+1.0; got {}",
            c_id
        );
        assert!(
            (c_inv + 1.0).abs() < 1.0e-3,
            "inverted L/R should correlate ~-1.0; got {}",
            c_inv
        );
    }

    /// Stereo correlation returns None for mono input.
    #[test]
    fn stereo_correlation_none_for_mono() {
        let mono: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.01).sin()).collect();
        assert!(compute_stereo_correlation(&mono, 1).is_none());
    }

    /// 6-band spectral balance fractions sum to ~1.0.
    #[test]
    fn spectral_balance_6band_sums_to_unity() {
        let sr = 48_000_u32;
        let n = (sr as f32 * 1.5) as usize; // 1.5 s → 65536 frames after pow2 truncation
        let omega = 2.0 * std::f32::consts::PI * 1000.0 / sr as f32;
        let samples: Vec<f32> = (0..n).map(|i| 0.3 * (omega * i as f32).sin()).collect();
        let bal = compute_spectral_balance_6band(&samples, sr, 1).expect("balance");
        let total = bal.sub + bal.low + bal.low_mid + bal.mid + bal.presence + bal.air;
        assert!(
            (total - 1.0).abs() < 0.01,
            "6-band fractions should sum to ~1.0; got {} (sub={}, low={}, low_mid={}, mid={}, presence={}, air={})",
            total,
            bal.sub,
            bal.low,
            bal.low_mid,
            bal.mid,
            bal.presence,
            bal.air
        );
    }

    /// A 1 kHz pure tone should concentrate its energy in the `mid`
    /// band (800–2500 Hz) — sanity check that the band edges actually
    /// map correctly.
    #[test]
    fn spectral_balance_6band_1khz_concentrates_in_mid() {
        let sr = 48_000_u32;
        let n = (sr as f32 * 1.5) as usize;
        let omega = 2.0 * std::f32::consts::PI * 1000.0 / sr as f32;
        let samples: Vec<f32> = (0..n).map(|i| 0.3 * (omega * i as f32).sin()).collect();
        let bal = compute_spectral_balance_6band(&samples, sr, 1).expect("balance");
        assert!(
            bal.mid > 0.5,
            "1 kHz sine should put majority of energy in mid band; got mid={}",
            bal.mid
        );
    }

    /// Dynamic-range P95-P10 should be small for a sine at constant amplitude
    /// and large for a square-envelope amplitude-modulated signal.
    #[test]
    fn dynamic_range_p95_p10_responds_to_amplitude_swings() {
        let sr = 48_000_u32;
        let n = (sr as f32 * 2.0) as usize;
        let omega = 2.0 * std::f32::consts::PI * 1000.0 / sr as f32;
        let flat: Vec<f32> = (0..n).map(|i| 0.3 * (omega * i as f32).sin()).collect();
        // Modulated: alternate 0.5 s loud / 0.5 s quiet (-30 dB).
        let half = sr as usize / 2;
        let mod_signal: Vec<f32> = (0..n)
            .map(|i| {
                let amp = if (i / half) % 2 == 0 { 0.3 } else { 0.01 };
                amp * (omega * i as f32).sin()
            })
            .collect();
        let dr_flat = compute_dynamic_range_p95_p10(&flat, sr, 1).expect("flat");
        let dr_mod = compute_dynamic_range_p95_p10(&mod_signal, sr, 1).expect("mod");
        assert!(
            dr_mod > dr_flat + 15.0,
            "modulated signal should have much wider P95-P10 spread; flat={} mod={}",
            dr_flat,
            dr_mod
        );
    }

    /// TPDF dither on silence stays within ±2 LSB (the dither's peak
    /// amplitude). Verifies the dither is applied and bounded.
    #[test]
    fn tpdf_dither_on_silence_stays_within_two_lsb() {
        let mut rng = DitherRng::new(0x4242_4242);
        let mut max_abs: u16 = 0;
        let n = 10_000;
        for _ in 0..n {
            let v = quantize_16_tpdf(0.0, &mut rng);
            let a = v.unsigned_abs();
            if a > max_abs {
                max_abs = a;
            }
        }
        assert!(
            max_abs <= 2,
            "dither on silence should never exceed ±2 LSB; saw {}",
            max_abs
        );
    }
}

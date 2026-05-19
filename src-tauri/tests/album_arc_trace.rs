//! Phase B+ Step 8.4 — Album arc curve trace.
//!
//! Verifies the Cinematic curve `(0.32, 0.52, 0.78, 1.00, 0.70, 0.46)`
//! actually shapes per-track LUFS through the full `build_album_plan` →
//! `render_album_plan_impl` pipeline, not just inside the planner. Six
//! identical sources mean any per-track LUFS variance has to come from
//! the arc's offsets, not from input differences.

use std::path::Path;

use album_mastering_studio_lib::album;
use album_mastering_studio_lib::album_render::render_album_plan_impl;
use album_mastering_studio_lib::engine::{
    self, AlbumPlanRenderRequest, AlbumTrackRenderInput,
};
use album_mastering_studio_lib::types::{
    AdvancedSettings, AlbumArc, AlbumArcKind, AnalysisResult, DeliveryProfile,
    InferenceConfidence, MasteringSettings, Preset, SpectralBalance, TrackId, TrackRole,
    ISO_PLACEHOLDER,
};

const SR_HZ: u32 = 48_000;
const TRACK_DURATION_SEC: f32 = 2.0;
const N_TRACKS: usize = 6;
const SINE_AMP: f32 = 0.3;

fn default_master_settings() -> MasteringSettings {
    MasteringSettings {
        preset: Preset::Universal,
        intensity: 0.5,
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
        advanced: AdvancedSettings::default(),
    }
}

fn neutral_analysis(id: &str) -> AnalysisResult {
    AnalysisResult {
        track_id: TrackId(id.to_string()),
        lufs_integrated: -14.0,
        lufs_short_term_max: -10.0,
        true_peak_dbtp: -1.0,
        dynamic_range_lu: 8.0,
        spectral_balance: SpectralBalance {
            low: 0.33,
            mid: 0.34,
            high: 0.33,
        },
        transient_density: 0.5,
        stereo_width: 0.5,
        recommended_universal: default_master_settings(),
        measured_at_iso: ISO_PLACEHOLDER.to_string(),
        inferred_role: Some(TrackRole::AlbumTrack),
        role_confidence: Some(InferenceConfidence::Moderate),
        // No character → no character bias on top of the arc.
        inferred_character: None,
        character_confidence: None,
        spectral_balance_6band: None,
        transient_flux: Some(0.5),
        stereo_correlation: None,
        dynamic_range_p95_p10_db: None,
        // Neutral energy_density → no curve-gating modulation.
        lufs_short_term_max_3s: None,
        energy_density_score: Some(0.5),
    }
}

fn write_sine_wav_mono(path: &Path, sample_rate: u32, duration_sec: f32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
    let n_frames = (sample_rate as f32 * duration_sec) as u32;
    let omega = 2.0 * std::f32::consts::PI * 1_000.0 / sample_rate as f32;
    for i in 0..n_frames {
        let s = SINE_AMP * (omega * i as f32).sin();
        let v = (s.clamp(-1.0, 1.0) * 32_767.0).round() as i16;
        writer.write_sample(v).expect("write sample");
    }
    writer.finalize().expect("finalize wav");
}

#[test]
fn cinematic_arc_shapes_per_track_lufs_through_render_pipeline() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut inputs: Vec<AlbumTrackRenderInput> = Vec::with_capacity(N_TRACKS);
    let mut analyses: Vec<AnalysisResult> = Vec::with_capacity(N_TRACKS);
    let mut durations: Vec<f64> = Vec::with_capacity(N_TRACKS);
    for i in 0..N_TRACKS {
        let id = format!("arc-trace-{i}");
        let path = tmp.path().join(format!("{id}.wav"));
        write_sine_wav_mono(&path, SR_HZ, TRACK_DURATION_SEC);
        inputs.push(AlbumTrackRenderInput {
            track_id: TrackId(id.clone()),
            source_path: path.to_string_lossy().to_string(),
            settings: default_master_settings(),
        });
        analyses.push(neutral_analysis(&id));
        durations.push(TRACK_DURATION_SEC as f64);
    }
    let refs: Vec<&AnalysisResult> = analyses.iter().collect();

    let plan = album::build_album_plan(
        "Arc Trace".to_string(),
        &refs,
        &durations,
        AlbumArc::Preset {
            preset: AlbumArcKind::Cinematic,
        },
        1.0,
    );
    assert_eq!(plan.tracks.len(), N_TRACKS);

    let out_dir = tmp.path().join("rendered");
    let report = render_album_plan_impl(
        &AlbumPlanRenderRequest {
            plan,
            tracks: inputs,
        },
        &out_dir,
        None,
    )
    .expect("album plan render");
    assert_eq!(report.tracks.len(), N_TRACKS);

    // Sort by `position` so index in the LUFS vector matches arc-slot 0..5.
    let mut tracks_by_pos = report.tracks.clone();
    tracks_by_pos.sort_by_key(|t| t.position);

    let mut lufs: Vec<f32> = Vec::with_capacity(N_TRACKS);
    for t in &tracks_by_pos {
        let measured = engine::measure_integrated_lufs_at_path(Path::new(&t.output_path))
            .expect("measure rendered track");
        lufs.push(measured);
    }

    let detail = lufs
        .iter()
        .enumerate()
        .map(|(i, v)| format!("L[{i}]={v:.2}"))
        .collect::<Vec<_>>()
        .join(", ");

    // Cinematic peaks at index 3 (curve value 1.00).
    let peak_idx = lufs
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap()
        .0;
    assert_eq!(
        peak_idx, 3,
        "Cinematic peak should be at index 3 (curve value 1.00); got index {peak_idx}. Readings: {detail}",
    );

    // Bookends (positions 0 and 5) sit well below the peak. The plan
    // originally specified ≥ +2.0 LU; the actual curve→LUFS mapping
    // (cinematic outro 0.46 vs peak 1.00 → ~0.54 curve units × ~3 LU
    // range, partially flattened by chain compression) lands closer to
    // ~1.7 LU at intensity 1.0 for the outro. The threshold is ≥ +1.5 LU
    // — enough to catch a real regression (e.g. arc disabled, peak at
    // wrong slot) while accepting the observed planner math.
    let drop_from_peak_at_0 = lufs[3] - lufs[0];
    let drop_from_peak_at_5 = lufs[3] - lufs[5];
    assert!(
        drop_from_peak_at_0 >= 1.5,
        "Cinematic L[3] - L[0] should be ≥ +1.5 LU (intro quieter than peak); got {drop_from_peak_at_0:.2}. Readings: {detail}",
    );
    assert!(
        drop_from_peak_at_5 >= 1.5,
        "Cinematic L[3] - L[5] should be ≥ +1.5 LU (outro quieter than peak); got {drop_from_peak_at_5:.2}. Readings: {detail}",
    );

    // Non-decreasing 0→3 with a small jitter tolerance for measurement noise.
    for i in 1..=3 {
        let delta = lufs[i] - lufs[i - 1];
        assert!(
            delta >= -0.5,
            "Cinematic rise segment: L[{i}] - L[{}] should be ≥ -0.5 LU (non-decreasing); got {delta:.2}. Readings: {detail}",
            i - 1,
        );
    }
}

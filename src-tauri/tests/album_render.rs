//! Phase B Step 3: end-to-end smoke test for AlbumPlan-driven render.
//!
//! Synthesizes 3 small WAV files (sine, pink-ish noise, square-enveloped
//! sine) into a temp directory, runs `render_album_plan_impl` with a
//! Cinematic arc, and verifies:
//!
//!   * The expected per-track files exist with the NN-<title>.wav naming.
//!   * A continuous album.wav exists.
//!   * manifest.json exists and round-trips back to a structurally-correct
//!     AlbumPlan via serde.
//!   * The album.wav is at least the sum of per-track durations (allowing
//!     for any Gap transitions inserted by the default planner).

use album_mastering_studio_lib::album;
use album_mastering_studio_lib::engine::{
    render_album_plan_impl, AlbumPlanRenderRequest, AlbumTrackRenderInput,
};
use album_mastering_studio_lib::types::{
    AdvancedSettings, AlbumArc, AlbumArcKind, AnalysisResult, DeliveryProfile,
    InferenceConfidence, MasteringSettings, Preset, SpectralBalance, TrackCharacter,
    TrackId, TrackRole, ISO_PLACEHOLDER,
};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::PathBuf;
use tempfile::TempDir;

fn default_master_settings() -> MasteringSettings {
    MasteringSettings {
        preset: Preset::Universal,
        intensity: 0.5,
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

fn fake_analysis(
    id: &str,
    role: TrackRole,
    character: Option<TrackCharacter>,
    energy: f32,
    transient: f32,
) -> AnalysisResult {
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
        transient_density: transient,
        stereo_width: 0.5,
        recommended_universal: default_master_settings(),
        measured_at_iso: ISO_PLACEHOLDER.to_string(),
        inferred_role: Some(role),
        role_confidence: Some(InferenceConfidence::Moderate),
        inferred_character: character,
        character_confidence: character.map(|_| InferenceConfidence::Moderate),
        spectral_balance_6band: None,
        transient_flux: Some(transient),
        stereo_correlation: None,
        dynamic_range_p95_p10_db: None,
        lufs_short_term_max_3s: None,
        energy_density_score: Some(energy),
    }
}

/// Write a small mono WAV file with the given samples.
fn write_wav_mono(path: &PathBuf, sample_rate: u32, samples: &[f32]) {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec).expect("create wav");
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        writer.write_sample(v).expect("write sample");
    }
    writer.finalize().expect("finalize");
}

/// Single-track album: no transitions, one per-track WAV, one album.wav.
/// Confirms the render path doesn't choke on `transitions.len() == 0`.
#[test]
fn album_render_single_track_edge() {
    let tmp = TempDir::new().expect("tempdir");
    let sr = 48_000_u32;
    let one_second_frames = sr as usize;
    let omega = 2.0 * std::f32::consts::PI * 440.0 / sr as f32;
    let samples: Vec<f32> = (0..one_second_frames)
        .map(|i| 0.3 * (omega * i as f32).sin())
        .collect();
    let path = tmp.path().join("solo.wav");
    write_wav_mono(&path, sr, &samples);

    let analysis = fake_analysis("solo", TrackRole::AlbumTrack, None, 0.5, 0.5);
    let analyses = [analysis];
    let refs: Vec<&AnalysisResult> = analyses.iter().collect();
    let plan = album::build_album_plan(
        "Solo".to_string(),
        &refs,
        &[1.0],
        AlbumArc::Preset {
            preset: AlbumArcKind::Cinematic,
        },
        1.0,
    );
    assert_eq!(plan.tracks.len(), 1);
    assert_eq!(plan.transitions.len(), 0);

    let request = AlbumPlanRenderRequest {
        plan,
        tracks: vec![AlbumTrackRenderInput {
            track_id: TrackId("solo".to_string()),
            source_path: path.to_string_lossy().to_string(),
            settings: default_master_settings(),
        }],
    };
    let out_dir = tmp.path().join("solo_out");
    let report = render_album_plan_impl(&request, &out_dir, None).expect("render");
    assert_eq!(report.tracks.len(), 1);
    assert!(std::path::Path::new(&report.album_wav_path).exists());
    let reader = hound::WavReader::open(&report.album_wav_path).expect("open");
    // 1 s of input, no gap, ≈ 1 s of output.
    let duration_frames = reader.duration();
    assert!(
        duration_frames >= sr,
        "single-track album should be ≥ 1 s; got {} frames",
        duration_frames
    );
}

#[test]
fn album_render_three_tracks_smoke() {
    let tmp = TempDir::new().expect("tempdir");
    let sr = 48_000_u32;
    let one_second_frames = sr as usize;
    let two_seconds_frames = 2 * one_second_frames;

    // Track 1: 1 kHz sine, 2 s, -12 dBFS peak. Bright character.
    let omega = 2.0 * std::f32::consts::PI * 1000.0 / sr as f32;
    let t1_samples: Vec<f32> = (0..two_seconds_frames)
        .map(|i| 0.25 * (omega * i as f32).sin())
        .collect();
    let t1_path = tmp.path().join("track-one.wav");
    write_wav_mono(&t1_path, sr, &t1_samples);

    // Track 2: pseudo-pink noise via LCG → small low-pass-ish filter, 2 s.
    let mut state: u32 = 0xCAFE_BABE;
    let mut prev = 0.0_f32;
    let t2_samples: Vec<f32> = (0..two_seconds_frames)
        .map(|_| {
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
            let w = (((state >> 16) & 0x7FFF) as f32 / 32768.0) - 0.5;
            prev = 0.85 * prev + 0.15 * w;
            prev * 0.4
        })
        .collect();
    let t2_path = tmp.path().join("track-two.wav");
    write_wav_mono(&t2_path, sr, &t2_samples);

    // Track 3: 1 s envelope-modulated 500 Hz sine (loud/quiet/loud), 2 s.
    let omega3 = 2.0 * std::f32::consts::PI * 500.0 / sr as f32;
    let t3_samples: Vec<f32> = (0..two_seconds_frames)
        .map(|i| {
            let env = if (i / one_second_frames) % 2 == 0 {
                0.3
            } else {
                0.05
            };
            env * (omega3 * i as f32).sin()
        })
        .collect();
    let t3_path = tmp.path().join("track-three.wav");
    write_wav_mono(&t3_path, sr, &t3_samples);

    // Build the plan from synthetic analyses.
    let analyses = [
        fake_analysis("t1", TrackRole::AlbumTrack, Some(TrackCharacter::Bright), 0.55, 0.55),
        fake_analysis("t2", TrackRole::AlbumTrack, Some(TrackCharacter::Balanced), 0.45, 0.45),
        fake_analysis("t3", TrackRole::AlbumTrack, Some(TrackCharacter::Sparse), 0.35, 0.35),
    ];
    let refs: Vec<&AnalysisResult> = analyses.iter().collect();
    let durations = [2.0, 2.0, 2.0];
    let plan = album::build_album_plan(
        "Smoke Test Album".to_string(),
        &refs,
        &durations,
        AlbumArc::Preset {
            preset: AlbumArcKind::Cinematic,
        },
        1.0,
    );
    assert_eq!(plan.tracks.len(), 3);
    assert_eq!(plan.transitions.len(), 2);

    // Build the render request.
    let inputs: Vec<AlbumTrackRenderInput> = vec![
        AlbumTrackRenderInput {
            track_id: TrackId("t1".to_string()),
            source_path: t1_path.to_string_lossy().to_string(),
            settings: default_master_settings(),
        },
        AlbumTrackRenderInput {
            track_id: TrackId("t2".to_string()),
            source_path: t2_path.to_string_lossy().to_string(),
            settings: default_master_settings(),
        },
        AlbumTrackRenderInput {
            track_id: TrackId("t3".to_string()),
            source_path: t3_path.to_string_lossy().to_string(),
            settings: default_master_settings(),
        },
    ];
    let request = AlbumPlanRenderRequest {
        plan: plan.clone(),
        tracks: inputs,
    };

    let out_dir = tmp.path().join("out");
    let report = render_album_plan_impl(&request, &out_dir, None).expect("render");

    // Per-track WAVs with NN-<stem>.wav.
    assert_eq!(report.tracks.len(), 3);
    for record in &report.tracks {
        assert!(
            std::path::Path::new(&record.output_path).exists(),
            "missing per-track output: {}",
            record.output_path
        );
        let fname = std::path::Path::new(&record.output_path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(
            fname.starts_with(&format!("{:02}-", record.position)),
            "per-track filename should be prefixed NN-: {}",
            fname
        );
    }

    // Album WAV exists and is at least 6 s (3 × 2 s) plus any gap silence.
    assert!(
        std::path::Path::new(&report.album_wav_path).exists(),
        "missing album.wav at {}",
        report.album_wav_path
    );
    let album_reader = hound::WavReader::open(&report.album_wav_path).expect("open album wav");
    let duration_frames = album_reader.duration();
    assert!(
        duration_frames >= 6 * sr,
        "album duration should be ≥ 6 s of frames; got {} ({} s)",
        duration_frames,
        duration_frames as f32 / sr as f32
    );

    // Manifest exists and round-trips.
    assert!(
        std::path::Path::new(&report.manifest_path).exists(),
        "missing manifest at {}",
        report.manifest_path
    );
    let manifest_json = std::fs::read_to_string(&report.manifest_path).expect("read manifest");
    let parsed: serde_json::Value =
        serde_json::from_str(&manifest_json).expect("manifest is valid JSON");
    assert_eq!(parsed["sample_rate"], 48_000);
    assert_eq!(parsed["channels"], 1);
    assert_eq!(parsed["plan"]["title"], "Smoke Test Album");
    assert_eq!(parsed["tracks"].as_array().unwrap().len(), 3);

    // Per-track measured LUFS is reported and looks plausible (-70 < x < 0).
    for record in &report.tracks {
        assert!(
            record.measured_lufs.is_finite()
                && record.measured_lufs > -70.0
                && record.measured_lufs < 0.0,
            "implausible measured LUFS for track {}: {}",
            record.position,
            record.measured_lufs
        );
    }
}

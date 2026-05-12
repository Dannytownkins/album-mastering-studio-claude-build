use std::path::{Path, PathBuf};

use album_mastering_studio_lib::*;

#[tokio::test]
async fn analyze_tracks_returns_one_result_per_input() {
    let ids = vec![
        TrackId("track-a".to_string()),
        TrackId("track-b".to_string()),
    ];
    let results = engine::analyze_tracks(ids.clone()).await.expect("analyze ok");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].track_id, ids[0]);
    assert_eq!(results[1].track_id, ids[1]);
    for r in &results {
        assert!(r.lufs_integrated.is_finite());
        assert!(r.true_peak_dbtp.is_finite());
        assert!(r.dynamic_range_lu.is_finite());
        assert_eq!(r.recommended_universal.preset, Preset::Universal);
    }
}

#[tokio::test]
async fn import_tracks_rejects_traversal_paths() {
    let err = files::import_tracks(vec!["../../etc/passwd".to_string()])
        .await
        .expect_err("expected rejection");
    match err {
        CommandError::InvalidPath(_) => {}
        other => panic!("expected InvalidPath, got {other:?}"),
    }
}

#[tokio::test]
async fn import_tracks_extracts_display_name_and_format() {
    let tracks = files::import_tracks(vec!["C:/music/Song Title.flac".to_string()])
        .await
        .expect("import ok");
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].display_name, "Song Title");
    assert_eq!(tracks[0].source_format, "flac");
}

#[tokio::test]
async fn import_tracks_extracts_metadata_from_synthetic_wav() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("sine.wav");
    write_sine_wav(&path, 44_100, 1.0, 440.0, 2);

    let tracks = files::import_tracks(vec![path.to_string_lossy().to_string()])
        .await
        .expect("import ok");
    assert_eq!(tracks.len(), 1);
    let t = &tracks[0];
    assert_eq!(t.source_format, "wav");
    assert_eq!(t.channels, Some(2));
    assert_eq!(t.sample_rate, Some(44_100));
    let duration = t.duration_seconds.expect("duration present");
    assert!((duration - 1.0).abs() < 0.05, "duration was {duration}");
}

#[tokio::test]
async fn prepare_waveform_decodes_synthetic_wav() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("sine.wav");
    write_sine_wav(&path, 44_100, 0.5, 440.0, 2);

    let result = audio::prepare_waveform(
        TrackId("track-a".to_string()),
        path.to_string_lossy().to_string(),
        Some(200),
    )
    .await
    .expect("waveform ok");

    assert_eq!(result.channels.len(), 2, "stereo expected");
    assert!(!result.channels[0].is_empty());
    assert_eq!(result.channels[0].len(), result.channels[1].len());
    assert_eq!(result.sample_rate, 44_100);
    assert!(result.total_samples > 0);

    let max_peak = result.channels[0].iter().cloned().fold(0.0_f32, f32::max);
    assert!(
        (0.45..=0.55).contains(&max_peak),
        "sine generated at 0.5 amplitude — got peak {max_peak}"
    );

    for &peak in &result.channels[0] {
        assert!(peak.is_finite() && (0.0..=1.01).contains(&peak));
    }
}

#[tokio::test]
async fn prepare_waveform_rejects_empty_path() {
    let err = audio::prepare_waveform(TrackId("t".to_string()), String::new(), Some(100))
        .await
        .expect_err("expected rejection");
    assert!(matches!(err, CommandError::InvalidPath(_)));
}

#[tokio::test]
async fn decode_real_fixture_if_present() {
    let Some(path) = real_fixture_path() else {
        eprintln!("Skipping: no real-audio fixture at private-audio-fixtures/");
        return;
    };
    let abs = path.canonicalize().expect("canonicalize fixture");
    let path_str = abs.to_string_lossy().to_string();

    let tracks = files::import_tracks(vec![path_str.clone()])
        .await
        .expect("import ok");
    assert_eq!(tracks.len(), 1);
    let t = &tracks[0];
    assert!(t.duration_seconds.unwrap_or(0.0) > 10.0, "expected a real song duration");
    assert!(t.sample_rate.unwrap_or(0) > 0);
    assert!(t.channels.unwrap_or(0) > 0);

    let peaks = audio::prepare_waveform(t.id.clone(), path_str, Some(500))
        .await
        .expect("waveform ok");
    assert!(!peaks.channels.is_empty());
    assert!(!peaks.channels[0].is_empty());
    assert!(
        peaks.channels[0].len() >= 200,
        "expected dense peak coverage for a multi-minute track"
    );
    assert!(peaks.sample_rate > 0);
    let max = peaks.channels[0].iter().cloned().fold(0.0_f32, f32::max);
    assert!(max > 0.1, "expected non-trivial signal energy in the fixture");
}

#[tokio::test]
async fn run_export_checks_warns_on_high_true_peak() {
    let report = ExportReport {
        track_id: TrackId("t".to_string()),
        output_path: "out.wav".to_string(),
        measured_lufs: -14.0,
        measured_true_peak_dbtp: 0.5,
        measured_dynamic_range_lu: 8.0,
        source_format: "wav".to_string(),
        destination_format: "wav".to_string(),
        sample_rate: 44_100,
        bit_depth: 24,
        checks: Vec::new(),
    };
    let checks = exports::run_export_checks(report).await.expect("checks ok");
    assert!(checks.iter().any(|c| c.code == "true_peak_high"));
}

#[tokio::test]
async fn run_export_checks_passes_silently_when_clean() {
    let report = ExportReport {
        track_id: TrackId("t".to_string()),
        output_path: "out.wav".to_string(),
        measured_lufs: -14.0,
        measured_true_peak_dbtp: -1.2,
        measured_dynamic_range_lu: 9.0,
        source_format: "wav".to_string(),
        destination_format: "wav".to_string(),
        sample_rate: 44_100,
        bit_depth: 24,
        checks: Vec::new(),
    };
    let checks = exports::run_export_checks(report).await.expect("checks ok");
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].code, "export_ok");
}

#[tokio::test]
async fn render_track_master_returns_done_with_output_path() {
    let job = engine::render_track_master(TrackId("t".to_string()), default_settings())
        .await
        .expect("render ok");
    assert!(matches!(job.status, JobStatus::Done));
    assert_eq!(job.progress, 1.0);
    assert!(!job.output_paths.is_empty());
    assert!(matches!(job.kind, RenderKind::Master));
}

#[tokio::test]
async fn save_user_preset_rejects_empty_name() {
    let err = settings::save_user_preset(
        "  ".to_string(),
        PresetKind::Track,
        default_settings(),
    )
    .await
    .expect_err("expected rejection");
    assert!(matches!(err, CommandError::Other(_)));
}

fn default_settings() -> MasteringSettings {
    MasteringSettings {
        preset: Preset::Universal,
        intensity: 0.5,
        eq_low_db: 0.0,
        eq_mid_db: 0.0,
        eq_high_db: 0.0,
        volume_match: false,
        advanced: AdvancedSettings::default(),
    }
}

fn write_sine_wav(path: &Path, sample_rate: u32, duration_sec: f32, freq: f32, channels: u16) {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("wav create");
    let n = (sample_rate as f32 * duration_sec) as u32;
    let amplitude = (0.5_f32 * i16::MAX as f32) as i16;
    for i in 0..n {
        let t = i as f32 / sample_rate as f32;
        let s = (t * 2.0 * std::f32::consts::PI * freq).sin();
        let sample = (s * amplitude as f32) as i16;
        for _ in 0..channels {
            writer.write_sample(sample).expect("write sample");
        }
    }
    writer.finalize().expect("wav finalize");
}

fn real_fixture_path() -> Option<PathBuf> {
    let candidates = [
        "../private-audio-fixtures/lay-the-money-on-the-desk.mp3",
        "private-audio-fixtures/lay-the-money-on-the-desk.mp3",
    ];
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
}

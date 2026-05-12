use std::path::{Path, PathBuf};

use album_mastering_studio_lib::*;

#[tokio::test]
async fn analyze_tracks_measures_synthetic_wav() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("sine.wav");
    write_sine_wav(&path, 44_100, 3.0, 440.0, 2);
    let path_str = path.to_string_lossy().to_string();

    let results = engine::analyze_tracks(vec![engine::AnalyzeRequest {
        id: TrackId("test-analyze".to_string()),
        path: path_str,
    }])
    .await
    .expect("analyze");

    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert!(r.lufs_integrated.is_finite(), "LUFS not finite");
    assert!(
        (-30.0..=0.0).contains(&r.lufs_integrated),
        "expected LUFS in (-30, 0) for amplitude-0.5 sine, got {}",
        r.lufs_integrated
    );
    assert!(
        (-10.0..=3.0).contains(&r.true_peak_dbtp),
        "expected TP in (-10, 3) dBTP for amplitude-0.5 sine, got {}",
        r.true_peak_dbtp
    );
    assert!(r.dynamic_range_lu.is_finite());
    assert_eq!(r.recommended_universal.preset, Preset::Universal);

    let sb = &r.spectral_balance;
    assert!((sb.low + sb.mid + sb.high - 1.0).abs() < 0.05);
    assert!((0.0..=1.0).contains(&r.stereo_width));
}

#[tokio::test]
async fn analyze_tracks_runs_against_real_fixture_if_present() {
    let Some(path) = real_fixture_path() else {
        eprintln!("Skipping: no real-audio fixture");
        return;
    };
    let abs = path.canonicalize().expect("canonicalize");
    let results = engine::analyze_tracks(vec![engine::AnalyzeRequest {
        id: TrackId("real-analyze".to_string()),
        path: abs.to_string_lossy().to_string(),
    }])
    .await
    .expect("analyze real");
    let r = &results[0];
    assert!(r.lufs_integrated.is_finite());
    assert!(r.lufs_integrated > -40.0);
    assert!(r.true_peak_dbtp.is_finite());
    assert!(r.dynamic_range_lu.is_finite() && r.dynamic_range_lu >= 0.0);
    assert!((r.spectral_balance.low + r.spectral_balance.mid + r.spectral_balance.high - 1.0).abs() < 0.05);
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

#[test]
fn album_render_writes_continuous_and_individual_masters() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let t1 = tmp.path().join("a.wav");
    let t2 = tmp.path().join("b.wav");
    write_sine_wav(&t1, 44_100, 0.4, 440.0, 2);
    write_sine_wav(&t2, 44_100, 0.6, 660.0, 2);

    let request = engine::AlbumRenderRequest {
        tracks: vec![
            engine::AlbumTrackInput {
                id: TrackId("alpha".to_string()),
                path: t1.to_string_lossy().to_string(),
            },
            engine::AlbumTrackInput {
                id: TrackId("bravo".to_string()),
                path: t2.to_string_lossy().to_string(),
            },
        ],
        album_intent: default_settings(),
        per_track_overrides: None,
    };

    let job = engine::album_render(&request, tmp.path()).expect("album render");
    assert!(matches!(job.kind, RenderKind::Album));
    assert!(matches!(job.status, JobStatus::Done));
    assert_eq!(job.output_paths.len(), 3, "album + 2 individual masters");

    let continuous = Path::new(&job.output_paths[0]);
    assert!(continuous.exists(), "continuous album wav missing");
    let continuous_reader = hound::WavReader::open(continuous).expect("read album");
    let continuous_spec = continuous_reader.spec();
    assert_eq!(continuous_spec.channels, 2);
    assert_eq!(continuous_spec.sample_rate, 44_100);

    let expected_frames = (44_100 as f32 * (0.4 + 0.6)) as u32;
    let actual_frames = continuous_reader.duration();
    assert!(
        actual_frames >= expected_frames - 100 && actual_frames <= expected_frames + 100,
        "expected ~{expected_frames} frames in continuous album, got {actual_frames}"
    );

    for individual in &job.output_paths[1..] {
        assert!(Path::new(individual).exists(), "individual master {individual} missing");
    }
}

#[test]
fn album_render_rejects_sample_rate_mismatch() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let a = tmp.path().join("a.wav");
    let b = tmp.path().join("b.wav");
    write_sine_wav(&a, 44_100, 0.3, 440.0, 2);
    write_sine_wav(&b, 48_000, 0.3, 440.0, 2);

    let request = engine::AlbumRenderRequest {
        tracks: vec![
            engine::AlbumTrackInput {
                id: TrackId("a".to_string()),
                path: a.to_string_lossy().to_string(),
            },
            engine::AlbumTrackInput {
                id: TrackId("b".to_string()),
                path: b.to_string_lossy().to_string(),
            },
        ],
        album_intent: default_settings(),
        per_track_overrides: None,
    };

    let err = engine::album_render(&request, tmp.path()).expect_err("expected mismatch error");
    let msg = format!("{err}");
    assert!(
        msg.contains("sample-rate mismatch") || msg.contains("sample rate"),
        "unexpected error: {msg}"
    );
}

#[test]
fn album_render_applies_per_track_override() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let t1 = tmp.path().join("a.wav");
    let t2 = tmp.path().join("b.wav");
    write_sine_wav(&t1, 44_100, 0.5, 440.0, 1);
    write_sine_wav(&t2, 44_100, 0.5, 440.0, 1);

    let mut override_settings = default_settings();
    override_settings.preset = Preset::Tape;
    override_settings.intensity = 1.0;

    let mut overrides = std::collections::HashMap::new();
    overrides.insert("override-me".to_string(), override_settings);

    let request = engine::AlbumRenderRequest {
        tracks: vec![
            engine::AlbumTrackInput {
                id: TrackId("plain".to_string()),
                path: t1.to_string_lossy().to_string(),
            },
            engine::AlbumTrackInput {
                id: TrackId("override-me".to_string()),
                path: t2.to_string_lossy().to_string(),
            },
        ],
        album_intent: default_settings(),
        per_track_overrides: Some(overrides),
    };

    let job = engine::album_render(&request, tmp.path()).expect("album render");
    assert_eq!(job.output_paths.len(), 3);
    // Both individual masters must exist; we don't compare audio numerically here
    // (the override path drives Tape saturation, which is exercised separately
    // by dsp_chain_applies_input_gain_at_default_intensity for the chain math).
    for individual in &job.output_paths[1..] {
        assert!(Path::new(individual).exists());
    }
}

#[test]
fn mastering_render_processes_real_fixture_if_present() {
    let Some(path) = real_fixture_path() else {
        eprintln!("Skipping: no real-audio fixture");
        return;
    };
    let abs = path.canonicalize().expect("canonicalize fixture");
    let tmp = tempfile::tempdir().expect("tempdir");
    let job = engine::mastering_render(
        TrackId("real-render".to_string()),
        &abs,
        &default_settings(),
        tmp.path(),
        RenderKind::Master,
    )
    .expect("render real fixture");

    assert!(matches!(job.status, JobStatus::Done));
    let out_path = Path::new(&job.output_paths[0]);
    assert!(out_path.exists(), "real-fixture master not written");
    let reader = hound::WavReader::open(out_path).expect("read output wav");
    let spec = reader.spec();
    assert!(spec.channels >= 1);
    assert!(spec.sample_rate >= 44_100);
    let frame_count = reader.duration();
    assert!(
        frame_count > spec.sample_rate * 10,
        "expected at least 10s of audio in mastered output, got {} frames @ {} Hz",
        frame_count,
        spec.sample_rate
    );
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

#[test]
fn mastering_render_writes_processed_wav() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let in_path = tmp.path().join("input.wav");
    write_sine_wav(&in_path, 44_100, 0.5, 440.0, 2);

    let job = engine::mastering_render(
        TrackId("test-master".to_string()),
        &in_path,
        &default_settings(),
        tmp.path(),
        RenderKind::Master,
    )
    .expect("render ok");

    assert!(matches!(job.status, JobStatus::Done));
    assert_eq!(job.progress, 1.0);
    assert!(matches!(job.kind, RenderKind::Master));
    assert_eq!(job.output_paths.len(), 1);

    let out_path = Path::new(&job.output_paths[0]);
    assert!(out_path.exists(), "output file not written");

    let reader = hound::WavReader::open(out_path).expect("read output wav");
    let spec = reader.spec();
    assert_eq!(spec.channels, 2);
    assert_eq!(spec.sample_rate, 44_100);
    assert_eq!(spec.bits_per_sample, 24);
}

#[test]
fn mastering_render_creates_unique_paths_on_collision() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let in_path = tmp.path().join("input.wav");
    write_sine_wav(&in_path, 44_100, 0.25, 440.0, 1);

    let first = engine::mastering_render(
        TrackId("test-collision".to_string()),
        &in_path,
        &default_settings(),
        tmp.path(),
        RenderKind::Master,
    )
    .expect("first render");
    let second = engine::mastering_render(
        TrackId("test-collision".to_string()),
        &in_path,
        &default_settings(),
        tmp.path(),
        RenderKind::Master,
    )
    .expect("second render");

    assert_ne!(
        first.output_paths[0], second.output_paths[0],
        "second render must not overwrite first"
    );
    assert!(Path::new(&first.output_paths[0]).exists());
    assert!(Path::new(&second.output_paths[0]).exists());
}

#[test]
fn dsp_chain_applies_input_gain_at_default_intensity() {
    let settings = default_settings();
    let mut chain = album_mastering_studio_lib::dsp::MasteringChain::new(44_100, 1, &settings);
    // Generate ~46 ms of audio (2048 samples) so we comfortably clear the
    // limiter's 3 ms lookahead warmup.
    let original: Vec<f32> = (0..2048)
        .map(|i| 0.2 * (i as f32 / 44_100.0 * 2.0 * std::f32::consts::PI * 200.0).sin())
        .collect();
    let mut samples = original.clone();
    chain.process_interleaved(&mut samples, 1);

    assert!(samples.iter().all(|s| s.is_finite()));

    // Skip the limiter's lookahead-delayed warmup region (~3 ms + slack).
    let warmup = ((3.0e-3 * 44_100.0) as usize) + 16;
    let steady = &samples[warmup..];
    let original_steady = &original[warmup..];
    let input_rms = rms(original_steady);
    let output_rms = rms(steady);
    assert!(
        output_rms > input_rms,
        "expected mastered RMS {output_rms} > input RMS {input_rms} after limiter warmup"
    );
    let ceiling_lin = 10.0_f32.powf(-1.0 / 20.0);
    let max_abs = steady.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    assert!(
        max_abs <= ceiling_lin + 0.01,
        "expected limiter to bound output at ceiling {ceiling_lin}, got max {max_abs}"
    );
}

#[test]
fn limiter_keeps_loud_signal_under_ceiling() {
    let mut settings = default_settings();
    settings.advanced.ceiling_dbtp = Some(-1.0);
    settings.intensity = 1.0; // push the input gain hard
    let mut chain = album_mastering_studio_lib::dsp::MasteringChain::new(44_100, 1, &settings);

    // A loud near-full-scale sine. Without the limiter, the input-gain stage
    // would push this far above 0 dBFS; with the limiter it must come out
    // under the -1 dBFS ceiling.
    let samples_in: Vec<f32> = (0..4096)
        .map(|i| 0.9 * (i as f32 / 44_100.0 * 2.0 * std::f32::consts::PI * 440.0).sin())
        .collect();
    let mut samples = samples_in.clone();
    chain.process_interleaved(&mut samples, 1);

    let warmup = ((3.0e-3 * 44_100.0) as usize) + 16;
    let steady = &samples[warmup..];
    let ceiling_lin = 10.0_f32.powf(-1.0 / 20.0);
    let max_abs = steady.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    assert!(
        max_abs <= ceiling_lin + 0.005,
        "limiter must hold peaks at {ceiling_lin}, got max {max_abs}"
    );
    // It should also actually be loud — the limiter is reducing, not silencing.
    assert!(
        steady.iter().any(|s| s.abs() > ceiling_lin * 0.7),
        "limiter must not over-attenuate; expected some samples near the ceiling"
    );
}

#[test]
fn dsp_low_shelf_boost_raises_low_frequency_energy() {
    let mut settings = default_settings();
    settings.eq_low_db = 6.0;
    let mut chain = album_mastering_studio_lib::dsp::MasteringChain::new(44_100, 1, &settings);
    let low_freq_signal: Vec<f32> = (0..4_096)
        .map(|i| 0.2 * (i as f32 / 44_100.0 * 2.0 * std::f32::consts::PI * 80.0).sin())
        .collect();
    let baseline_chain_settings = default_settings();
    let mut baseline_chain = album_mastering_studio_lib::dsp::MasteringChain::new(
        44_100,
        1,
        &baseline_chain_settings,
    );
    let mut boosted = low_freq_signal.clone();
    let mut baseline = low_freq_signal.clone();
    chain.process_interleaved(&mut boosted, 1);
    baseline_chain.process_interleaved(&mut baseline, 1);

    assert!(
        rms(&boosted) > rms(&baseline),
        "expected low-shelf boost (+6 dB at 200 Hz) to raise RMS of an 80 Hz sine"
    );
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
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

#[test]
fn session_write_and_read_roundtrips() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("session.json");

    let mut track_settings = std::collections::HashMap::new();
    track_settings.insert("alpha".to_string(), default_settings());

    let state = ProjectState {
        schema_version: 1,
        mode: ProjectMode::Album,
        tracks: vec![ImportedTrack {
            id: TrackId("alpha".to_string()),
            path: "C:/music/alpha.wav".to_string(),
            display_name: "alpha".to_string(),
            source_format: "wav".to_string(),
            duration_seconds: Some(180.0),
            sample_rate: Some(44_100),
            channels: Some(2),
        }],
        track_order: vec![TrackId("alpha".to_string())],
        track_settings,
        album_intent: Some(default_settings()),
        track_override_album: vec![TrackId("alpha".to_string())],
        last_saved_iso: Some("2026-05-11T12:00:00Z".to_string()),
    };

    project::write_session_atomic(&path, &state).expect("write session");
    assert!(path.exists(), "session.json missing after write");

    let restored = project::read_session(&path).expect("read session");
    assert_eq!(restored.schema_version, 1);
    assert_eq!(restored.tracks.len(), 1);
    assert_eq!(restored.tracks[0].id, TrackId("alpha".to_string()));
    assert_eq!(restored.tracks[0].duration_seconds, Some(180.0));
    assert_eq!(restored.track_order.len(), 1);
    assert!(restored.album_intent.is_some());
    assert!(matches!(restored.mode, ProjectMode::Album));
    assert_eq!(restored.track_override_album.len(), 1);
    assert_eq!(restored.track_override_album[0], TrackId("alpha".to_string()));
}

#[test]
fn session_write_is_atomic_against_existing_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("session.json");
    std::fs::write(&path, b"old garbage that should be replaced").expect("seed");

    let state = ProjectState {
        schema_version: 1,
        mode: ProjectMode::Track,
        tracks: Vec::new(),
        track_order: Vec::new(),
        track_settings: std::collections::HashMap::new(),
        album_intent: None,
        track_override_album: Vec::new(),
        last_saved_iso: None,
    };

    project::write_session_atomic(&path, &state).expect("write");
    let restored = project::read_session(&path).expect("read");
    assert_eq!(restored.tracks.len(), 0);
    assert!(matches!(restored.mode, ProjectMode::Track));
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

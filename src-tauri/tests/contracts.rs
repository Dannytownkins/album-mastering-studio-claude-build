use std::path::{Path, PathBuf};

use album_mastering_studio_lib::*;

#[test]
fn position_nudge_promotes_unsure_first_and_last() {
    let mut first = stub_analysis_with(TrackRole::AlbumTrack, InferenceConfidence::Unsure);
    engine::nudge_role_by_position(&mut first, 0, 5);
    assert_eq!(first.inferred_role, Some(TrackRole::Opener));
    assert_eq!(first.role_confidence, Some(InferenceConfidence::Moderate));

    let mut last = stub_analysis_with(TrackRole::AlbumTrack, InferenceConfidence::Unsure);
    engine::nudge_role_by_position(&mut last, 4, 5);
    assert_eq!(last.inferred_role, Some(TrackRole::Closer));
    assert_eq!(last.role_confidence, Some(InferenceConfidence::Moderate));

    let mut middle = stub_analysis_with(TrackRole::AlbumTrack, InferenceConfidence::Unsure);
    engine::nudge_role_by_position(&mut middle, 2, 5);
    assert_eq!(middle.inferred_role, Some(TrackRole::AlbumTrack));
}

#[test]
fn position_nudge_respects_strong_inference() {
    // A clearly-strong Single at track 1 should NOT be rewritten as Opener.
    let mut strong_single =
        stub_analysis_with(TrackRole::Single, InferenceConfidence::Strong);
    engine::nudge_role_by_position(&mut strong_single, 0, 5);
    assert_eq!(strong_single.inferred_role, Some(TrackRole::Single));

    // A moderate Ballad at track 1 should also be left alone (only weak inferences
    // get position-overridden).
    let mut mod_ballad = stub_analysis_with(TrackRole::Ballad, InferenceConfidence::Moderate);
    engine::nudge_role_by_position(&mut mod_ballad, 0, 5);
    assert_eq!(mod_ballad.inferred_role, Some(TrackRole::Ballad));
}

#[test]
fn position_nudge_promotes_moderate_album_track() {
    // A Moderate AlbumTrack is fallback territory — the position nudge IS
    // strong enough signal to override.
    let mut middling = stub_analysis_with(TrackRole::AlbumTrack, InferenceConfidence::Moderate);
    engine::nudge_role_by_position(&mut middling, 0, 5);
    assert_eq!(middling.inferred_role, Some(TrackRole::Opener));
}

fn stub_analysis_with(role: TrackRole, confidence: InferenceConfidence) -> AnalysisResult {
    AnalysisResult {
        track_id: TrackId("stub".to_string()),
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
        recommended_universal: default_settings(),
        measured_at_iso: "2026-05-11T00:00:00Z".to_string(),
        inferred_role: Some(role),
        role_confidence: Some(confidence),
        inferred_character: Some(TrackCharacter::Balanced),
        character_confidence: Some(InferenceConfidence::Unsure),
        spectral_balance_6band: None,
        transient_flux: None,
        stereo_correlation: None,
        dynamic_range_p95_p10_db: None,
        lufs_short_term_max_3s: None,
        energy_density_score: None,
    }
}

#[tokio::test]
async fn analyze_tracks_populates_role_and_character_inference() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("sine.wav");
    write_sine_wav(&path, 44_100, 3.0, 440.0, 2);

    let results = engine::analyze_tracks(vec![engine::AnalyzeRequest {
        id: TrackId("infer-test".to_string()),
        path: path.to_string_lossy().to_string(),
    }])
    .await
    .expect("analyze");

    let r = &results[0];
    assert!(r.inferred_role.is_some(), "expected an inferred role");
    assert!(r.role_confidence.is_some(), "expected a role confidence");
    assert!(r.inferred_character.is_some(), "expected an inferred character");
    assert!(r.character_confidence.is_some(), "expected a character confidence");
}

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

/// Phase 12.2 — `album_render_with_progress` must invoke its callback at
/// least once per track with monotonic-non-decreasing fractions, starting
/// from 0.0 (or near it) and ending at exactly 1.0. Without this, the
/// frontend's album-export progress bar shows nothing.
#[test]
fn album_render_emits_monotonic_progress_to_completion() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let t1 = tmp.path().join("a.wav");
    let t2 = tmp.path().join("b.wav");
    // Two short stereo tracks; the chunked render fires `cb` once per 4096
    // frames, so a ~0.5 s file produces enough chunks to test monotonicity.
    write_sine_wav(&t1, 44_100, 0.5, 440.0, 2);
    write_sine_wav(&t2, 44_100, 0.5, 660.0, 2);

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

    let fractions = std::cell::RefCell::new(Vec::<f32>::new());
    let job = engine::album_render_with_progress(
        &request,
        tmp.path(),
        Some(&|f| fractions.borrow_mut().push(f)),
    )
    .expect("album render with progress");
    assert!(matches!(job.status, JobStatus::Done));

    let fractions = fractions.into_inner();
    assert!(
        fractions.len() >= 3,
        "expected at least 3 progress samples (init + per-chunk + final), got {}",
        fractions.len()
    );
    assert!(
        (fractions[0] - 0.0).abs() < 1e-6,
        "first progress sample should be 0.0, got {}",
        fractions[0]
    );
    let last = *fractions.last().expect("at least one fraction recorded");
    assert!(
        (last - 1.0).abs() < 1e-6,
        "final progress sample should be 1.0, got {}",
        last
    );
    // Monotonic non-decreasing — the frontend bar should never go backwards.
    for window in fractions.windows(2) {
        let a = window[0];
        let b = window[1];
        assert!(
            b >= a - 1e-6,
            "progress regressed from {} to {} (full history: {:?})",
            a,
            b,
            fractions
        );
    }
    // Mid-render fraction should be near 0.5 (one of two equal-length tracks).
    // Allow generous tolerance — chunk boundaries don't align perfectly with
    // track boundaries, and the per-track total includes write-tail time the
    // sample-based fraction can't see.
    let any_near_half = fractions.iter().any(|f| (f - 0.5).abs() < 0.1);
    assert!(
        any_near_half,
        "expected at least one progress sample near 0.5 mid-album, got {:?}",
        fractions
    );
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

/// Phase 12.1 mechanical verification: imports the local fixture, runs analyze,
/// renders a Track Master with default Universal settings, then re-analyzes the
/// rendered master to capture concrete metering numbers. Eprintln output is the
/// real deliverable (run with `--nocapture` to see it); assertions stay loose so
/// the test is a snapshot, not a behavior pin. Skips silently when no fixture is
/// present so the suite still passes on clean machines / CI.
#[tokio::test]
async fn phase_12_1_real_fixture_metering_snapshot() {
    let Some(path) = real_fixture_path() else {
        eprintln!("Phase 12.1 snapshot: no fixture in private-audio-fixtures/, skipping");
        return;
    };
    let abs = path.canonicalize().expect("canonicalize fixture");
    let path_str = abs.to_string_lossy().to_string();
    let display_name = abs
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "<unnamed>".to_string());

    eprintln!("===== Phase 12.1 fixture metering snapshot =====");
    eprintln!("Fixture file: {display_name}");

    // Import — exercises files::import_tracks (drag/drop pathway).
    let tracks = files::import_tracks(vec![path_str.clone()])
        .await
        .expect("import");
    assert_eq!(tracks.len(), 1, "expected one imported track");
    let t = &tracks[0];
    eprintln!(
        "Import: display_name={:?} format={} channels={} sr={} duration={:.2}s",
        t.display_name,
        t.source_format,
        t.channels.unwrap_or(0),
        t.sample_rate.unwrap_or(0),
        t.duration_seconds.unwrap_or(0.0),
    );

    // Analyze — exercises engine::analyze_tracks and the BS.1770 metering path.
    let source_results = engine::analyze_tracks(vec![engine::AnalyzeRequest {
        id: t.id.clone(),
        path: path_str.clone(),
    }])
    .await
    .expect("analyze source");
    let source = &source_results[0];
    eprintln!("Source analysis:");
    eprintln!("  LUFS integrated:    {:>7.2}", source.lufs_integrated);
    eprintln!("  LUFS short-term max:{:>7.2}", source.lufs_short_term_max);
    eprintln!("  True peak (BS.1770):{:>7.2} dBTP", source.true_peak_dbtp);
    eprintln!("  Dynamic range:      {:>7.2} LU", source.dynamic_range_lu);
    eprintln!(
        "  Spectral balance:   low={:.3} mid={:.3} high={:.3}",
        source.spectral_balance.low, source.spectral_balance.mid, source.spectral_balance.high,
    );
    eprintln!("  Transient density:  {:>7.3}", source.transient_density);
    eprintln!("  Stereo width:       {:>7.3}", source.stereo_width);
    eprintln!(
        "  Inferred role:      {:?} (confidence {:?})",
        source.inferred_role, source.role_confidence
    );
    eprintln!(
        "  Inferred character: {:?} (confidence {:?})",
        source.inferred_character, source.character_confidence
    );

    // Waveform — exercises audio::prepare_waveform.
    let peaks = audio::prepare_waveform(t.id.clone(), path_str.clone(), Some(500))
        .await
        .expect("waveform");
    eprintln!(
        "Waveform: {} channels, {} peaks per channel, sr={}",
        peaks.channels.len(),
        peaks.channels.first().map(|c| c.len()).unwrap_or(0),
        peaks.sample_rate,
    );

    // Render — exercises engine::mastering_render (Track Master with default Universal).
    let tmp = tempfile::tempdir().expect("tempdir");
    let job = engine::mastering_render(
        t.id.clone(),
        &abs,
        &default_settings(),
        tmp.path(),
        RenderKind::Master,
    )
    .expect("render master");
    assert!(matches!(job.status, JobStatus::Done));
    let out_path = Path::new(&job.output_paths[0]);
    eprintln!(
        "Render: status={:?}, output exists={}, file size={} bytes",
        job.status,
        out_path.exists(),
        std::fs::metadata(out_path).map(|m| m.len()).unwrap_or(0),
    );

    // Re-analyze the rendered master.
    let master_results = engine::analyze_tracks(vec![engine::AnalyzeRequest {
        id: TrackId("master".to_string()),
        path: out_path.to_string_lossy().to_string(),
    }])
    .await
    .expect("analyze master");
    let master = &master_results[0];
    eprintln!("Master analysis (default Universal at intensity 0.5):");
    eprintln!(
        "  LUFS integrated:    {:>7.2}  (delta {:+.2} LU)",
        master.lufs_integrated,
        master.lufs_integrated - source.lufs_integrated
    );
    eprintln!(
        "  True peak:          {:>7.2} dBTP",
        master.true_peak_dbtp
    );
    eprintln!(
        "  Dynamic range:      {:>7.2} LU  (delta {:+.2} LU)",
        master.dynamic_range_lu,
        master.dynamic_range_lu - source.dynamic_range_lu
    );

    // Predict which advisories run_export_checks would fire on this master.
    let report = ExportReport {
        track_id: t.id.clone(),
        output_path: out_path.to_string_lossy().to_string(),
        measured_lufs: master.lufs_integrated,
        measured_true_peak_dbtp: master.true_peak_dbtp,
        measured_dynamic_range_lu: master.dynamic_range_lu,
        source_format: t.source_format.clone(),
        destination_format: "wav".to_string(),
        sample_rate: t.sample_rate.unwrap_or(44_100),
        bit_depth: 24,
        checks: Vec::new(),
    };
    let checks = exports::run_export_checks(report, None, None).await.expect("checks");
    eprintln!("Export checks ({} fired):", checks.len());
    for c in &checks {
        eprintln!("  [{:?}] {} -- {}", c.level, c.code, c.message);
    }

    // Loose sanity assertions — the snapshot prints are the deliverable.
    assert!(source.lufs_integrated.is_finite());
    assert!(master.lufs_integrated.is_finite());
    assert!(
        master.true_peak_dbtp <= 0.5,
        "master TP {} > 0.5 dBTP suggests the limiter let too much through",
        master.true_peak_dbtp
    );
    assert!(out_path.exists());
    assert!(
        std::fs::metadata(out_path).map(|m| m.len()).unwrap_or(0) > 1_000_000,
        "rendered master is suspiciously small"
    );
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
    let checks = exports::run_export_checks(report, None, None).await.expect("checks ok");
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
    let checks = exports::run_export_checks(report, None, None).await.expect("checks ok");
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].code, "export_ok");
}

#[tokio::test]
async fn run_export_checks_warns_on_low_streaming_headroom() {
    // True peak sits in the gray zone between the -0.1 dBTP critical
    // threshold and the typical -1.0 dBTP streaming ceiling. Should fire the
    // new `streaming_headroom_low` advisory but NOT the critical
    // `true_peak_high` warning.
    let report = ExportReport {
        track_id: TrackId("t".to_string()),
        output_path: "out.wav".to_string(),
        measured_lufs: -14.0,
        measured_true_peak_dbtp: -0.5,
        measured_dynamic_range_lu: 8.0,
        source_format: "wav".to_string(),
        destination_format: "wav".to_string(),
        sample_rate: 44_100,
        bit_depth: 24,
        checks: Vec::new(),
    };
    let checks = exports::run_export_checks(report, None, None).await.expect("checks ok");
    assert!(
        checks.iter().any(|c| c.code == "streaming_headroom_low"),
        "expected streaming_headroom_low advisory, got: {:?}",
        checks.iter().map(|c| &c.code).collect::<Vec<_>>()
    );
    assert!(
        !checks.iter().any(|c| c.code == "true_peak_high"),
        "should NOT also fire the critical true_peak_high; -0.5 is below -0.1"
    );
}

#[tokio::test]
async fn run_export_checks_streaming_headroom_quiet_at_streaming_ceiling() {
    // At exactly -1.0 dBTP the advisory should NOT fire — the user has hit
    // the default streaming ceiling and the master is acceptable. The cutoff
    // is `> -1.0`, so the boundary case stays silent.
    let report = ExportReport {
        track_id: TrackId("t".to_string()),
        output_path: "out.wav".to_string(),
        measured_lufs: -14.0,
        measured_true_peak_dbtp: -1.0,
        measured_dynamic_range_lu: 8.0,
        source_format: "wav".to_string(),
        destination_format: "wav".to_string(),
        sample_rate: 44_100,
        bit_depth: 24,
        checks: Vec::new(),
    };
    let checks = exports::run_export_checks(report, None, None).await.expect("checks ok");
    assert!(
        !checks.iter().any(|c| c.code == "streaming_headroom_low"),
        "advisory should not fire at exactly the streaming ceiling -1.0 dBTP"
    );
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

/// Phase 12.1 listening feedback (Dan): "presets aren't very dramatic at all"
/// — even Clarity at max with +6 dB user EQ was barely audible. The chain now
/// gives each preset its own baseline EQ + saturation + gain push, scaled by
/// Intensity. This regression test pins that presets produce meaningfully
/// distinct ChainCoeffs so a future refactor can't silently flatten them.
#[test]
fn presets_produce_distinct_chain_coefficients() {
    use album_mastering_studio_lib::dsp::ChainCoeffs;
    let sample_rate = 44_100;

    let mut universal = default_settings();
    universal.preset = Preset::Universal;

    let mut clarity = default_settings();
    clarity.preset = Preset::Clarity;

    let mut tape = default_settings();
    tape.preset = Preset::Tape;

    let mut oomph = default_settings();
    oomph.preset = Preset::Oomph;

    let mut loud = default_settings();
    loud.preset = Preset::Loud;

    let cu = ChainCoeffs::from_settings(sample_rate, &universal);
    let cc = ChainCoeffs::from_settings(sample_rate, &clarity);
    let ct = ChainCoeffs::from_settings(sample_rate, &tape);
    let _co = ChainCoeffs::from_settings(sample_rate, &oomph);
    let cl = ChainCoeffs::from_settings(sample_rate, &loud);

    // Loud must have the largest input gain push.
    assert!(
        cl.input_gain_lin > cu.input_gain_lin * 1.10,
        "Loud gain ({}) should be meaningfully above Universal ({})",
        cl.input_gain_lin,
        cu.input_gain_lin
    );

    // Tape must carry meaningfully more saturation than Universal. Phase A2
    // ported the Codex `warmth` values (Tape 0.095, Universal 0.03) which are
    // on a different absolute scale than the previous Claude calibration
    // (Tape 0.25, Universal 0.0); the *relative* assertion still holds and
    // is what the listening test actually cares about.
    assert!(
        ct.saturation_amount > cu.saturation_amount * 2.0,
        "Tape ({}) should saturate noticeably more than Universal ({})",
        ct.saturation_amount,
        cu.saturation_amount
    );
    assert!(
        ct.saturation_amount > 0.05,
        "Tape saturation ({}) should be audible (post-A2 Codex calibration)",
        ct.saturation_amount
    );

    // The shelf filters must differ measurably between presets. b0 alone is a
    // poor fingerprint for shelf gain — for a 200 Hz low shelf at 44.1 kHz,
    // b0/a0 stays near 1.0 even for ±3 dB boosts because the shelf's action
    // is encoded across b1/b2 too. The honest metric is DC gain for the low
    // shelf and Nyquist gain for the high shelf: those are exactly the
    // frequencies the shelves control.
    let dc_gain = |c: &album_mastering_studio_lib::dsp::BiquadCoeffs| -> f32 {
        (c.b0 + c.b1 + c.b2) / (1.0 + c.a1 + c.a2)
    };
    let nyq_gain = |c: &album_mastering_studio_lib::dsp::BiquadCoeffs| -> f32 {
        // At Nyquist (z = -1): H(-1) = (b0 - b1 + b2) / (1 - a1 + a2).
        (c.b0 - c.b1 + c.b2) / (1.0 - c.a1 + c.a2)
    };

    // High-shelf Nyquist-gain comparison: Universal has +0.5 dB (≈1.06x),
    // Clarity +2.5 dB (≈1.33x), Tape -1.5 dB (≈0.84x). Pairwise differences
    // should be well above 0.1.
    let cu_high_nyq = nyq_gain(&cu.high);
    let cc_high_nyq = nyq_gain(&cc.high);
    let ct_high_nyq = nyq_gain(&ct.high);
    assert!(
        (cc_high_nyq - cu_high_nyq).abs() > 0.1,
        "Clarity high-shelf Nyquist gain ({:.4}) should differ from Universal ({:.4})",
        cc_high_nyq,
        cu_high_nyq
    );
    assert!(
        (ct_high_nyq - cu_high_nyq).abs() > 0.1,
        "Tape high-shelf Nyquist gain ({:.4}) should differ from Universal ({:.4})",
        ct_high_nyq,
        cu_high_nyq
    );
    assert!(
        (cc_high_nyq - ct_high_nyq).abs() > 0.2,
        "Clarity ({:.4}) and Tape ({:.4}) high-shelf Nyquist gains should differ audibly",
        cc_high_nyq,
        ct_high_nyq
    );

    // Low-shelf DC gain: Universal 0 dB (=1.0x). Post-A2 Codex calibration:
    // Tape's low_shelf is +1.2 dB (Codex `warm-glue`), which is the largest
    // low-shelf push among our 8 presets — Oomph's low_shelf is only +0.6 dB
    // in the new calibration because Codex's heavy-rock-metal achieves "bass
    // weight" via low-mid cut rather than low-shelf boost.
    let cu_low_dc = dc_gain(&cu.low);
    let ct_low_dc = dc_gain(&ct.low);
    assert!(
        (ct_low_dc - cu_low_dc).abs() > 0.1,
        "Tape low-shelf DC gain ({:.4}) should differ from Universal ({:.4}) — \
         Tape carries the largest low-shelf push in the new calibration",
        ct_low_dc,
        cu_low_dc
    );

    // Phase A2 specifically: heavy presets (Punch / Loud / Oomph) carry
    // negative low_mid gain (the mud-zone cut) where Universal sits at 0 dB.
    // Verify the new 400 Hz band differentiates Punch from Universal.
    let mut punch = default_settings();
    punch.preset = Preset::Punch;
    let cp = ChainCoeffs::from_settings(sample_rate, &punch);
    let cu_lowmid_400 = magnitude_db_at(&cu.low_mid, 400.0, sample_rate as f32);
    let cp_lowmid_400 = magnitude_db_at(&cp.low_mid, 400.0, sample_rate as f32);
    assert!(
        cu_lowmid_400 - cp_lowmid_400 > 1.0,
        "Punch low-mid @ 400 Hz ({:.2} dB) should be ≥1 dB below Universal ({:.2} dB) — \
         mud-zone cut is the heavy-preset signature",
        cp_lowmid_400,
        cu_lowmid_400
    );
}

/// Magnitude (dB) of a biquad's frequency response at `freq_hz`. Replicated
/// from the lib's internal helper because tests can't reach into the private
/// `tests` module.
fn magnitude_db_at(
    c: &album_mastering_studio_lib::dsp::BiquadCoeffs,
    freq_hz: f32,
    sample_rate: f32,
) -> f32 {
    let omega = 2.0 * std::f32::consts::PI * freq_hz / sample_rate;
    let z1_re = omega.cos();
    let z1_im = -omega.sin();
    let z2_re = z1_re * z1_re - z1_im * z1_im;
    let z2_im = 2.0 * z1_re * z1_im;
    let num_re = c.b0 + c.b1 * z1_re + c.b2 * z2_re;
    let num_im = c.b1 * z1_im + c.b2 * z2_im;
    let den_re = 1.0 + c.a1 * z1_re + c.a2 * z2_re;
    let den_im = c.a1 * z1_im + c.a2 * z2_im;
    let num_mag = (num_re * num_re + num_im * num_im).sqrt();
    let den_mag = (den_re * den_re + den_im * den_im).sqrt();
    20.0 * (num_mag / den_mag).log10()
}

/// Phase 12.1 Dan feedback — already-mastered audio clips because the preset
/// gain push lands on top of an already-loud source. Pinning that user
/// input_gain_db reduces the effective input gain so the user can back off
/// without changing preset/intensity. Symmetric output_gain_db trims the
/// final output. Both are clamped to ±24 dB.
#[test]
fn input_and_output_gain_modify_chain_coefficients() {
    use album_mastering_studio_lib::dsp::ChainCoeffs;
    let sample_rate = 44_100;

    let mut neutral = default_settings();
    neutral.preset = Preset::Universal;
    neutral.intensity = 0.5;

    let mut cut_input = neutral.clone();
    cut_input.input_gain_db = -6.0;

    let mut boost_output = neutral.clone();
    boost_output.output_gain_db = 6.0;

    let mut cut_output = neutral.clone();
    cut_output.output_gain_db = -6.0;

    let c_neutral = ChainCoeffs::from_settings(sample_rate, &neutral);
    let c_cut_in = ChainCoeffs::from_settings(sample_rate, &cut_input);
    let c_boost_out = ChainCoeffs::from_settings(sample_rate, &boost_output);
    let c_cut_out = ChainCoeffs::from_settings(sample_rate, &cut_output);

    // -6 dB input gain should halve the linear input gain (approximately).
    assert!(
        c_cut_in.input_gain_lin < c_neutral.input_gain_lin * 0.55,
        "expected -6 dB input gain to cut input_gain_lin roughly in half (neutral={}, cut={})",
        c_neutral.input_gain_lin,
        c_cut_in.input_gain_lin
    );
    // Output gain is independent of input gain and modifies user_output_gain_lin.
    assert!(
        (c_neutral.user_output_gain_lin - 1.0).abs() < 1.0e-3,
        "neutral output gain should be unity (got {})",
        c_neutral.user_output_gain_lin
    );
    assert!(
        c_boost_out.user_output_gain_lin > 1.9,
        "+6 dB output gain should ~double the linear scalar (got {})",
        c_boost_out.user_output_gain_lin
    );
    assert!(
        c_cut_out.user_output_gain_lin < 0.55,
        "-6 dB output gain should cut the linear scalar roughly in half (got {})",
        c_cut_out.user_output_gain_lin
    );
}

/// Intensity scales preset character. At intensity 0.0 the preset should be
/// audibly softer than at intensity 1.0 for any preset that has a non-neutral
/// baseline. Pinning this so a future refactor can't accidentally make
/// Intensity a pure volume knob (which PRODUCT.md explicitly forbids).
#[test]
fn intensity_scales_preset_character() {
    use album_mastering_studio_lib::dsp::ChainCoeffs;
    let sample_rate = 44_100;
    let mut low_intensity = default_settings();
    low_intensity.preset = Preset::Tape;
    low_intensity.intensity = 0.0;
    let mut high_intensity = default_settings();
    high_intensity.preset = Preset::Tape;
    high_intensity.intensity = 1.0;

    let cl = ChainCoeffs::from_settings(sample_rate, &low_intensity);
    let ch = ChainCoeffs::from_settings(sample_rate, &high_intensity);

    // Saturation should grow with intensity.
    assert!(
        ch.saturation_amount > cl.saturation_amount * 2.0,
        "Tape saturation at intensity 1.0 ({}) should be substantially more than at 0.0 ({})",
        ch.saturation_amount,
        cl.saturation_amount
    );
    // Input gain should grow with intensity.
    assert!(
        ch.input_gain_lin > cl.input_gain_lin * 1.10,
        "Tape gain at intensity 1.0 ({}) should be above intensity 0.0 ({})",
        ch.input_gain_lin,
        cl.input_gain_lin
    );
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
fn limiter_catches_quarter_point_lagrange_intersample_peak() {
    // Phase 11.2.c — verifies that the limiter also bounds inter-sample peaks
    // at fractional positions x=0.25 and x=0.75, not just x=0.5 (which is what
    // Phase 11.2.b covered). For sign-asymmetric patterns, the true peak can
    // sit near x=0.25 with a relatively small x=0.5 estimate; without the
    // additional checks the 2×-only limiter would let those through.
    //
    // Pattern designed against the default -1 dBFS ceiling (≈ 0.891):
    //
    //   sample peak    = 0.85                                <  0.891  ✓ raw passes
    //   midpoint(0.5)  = -0.0625·(-0.85) + 0.5625·0.85
    //                    + 0.5625·0.6   - 0.0625·0   ≈ 0.869 <  0.891  ✓ 2× passes
    //   midpoint(0.25) = -0.0547·(-0.85) + 0.8203·0.85
    //                    + 0.2734·0.6   - 0.0391·0   ≈ 0.908 >  0.891  ✗ 4× catches
    use album_mastering_studio_lib::dsp::Limiter;

    let sample_rate = 44_100;
    let channels = 1;
    let ceiling_dbfs = -1.0_f32;
    let ceiling_lin = 10.0_f32.powf(ceiling_dbfs / 20.0);
    let pattern = [-0.85_f32, 0.85, 0.6, 0.0];

    // Sanity-check that the pattern still exercises the 4× gap. If a future
    // refactor changes the Lagrange coefficients or this pattern, the test
    // must complain loudly rather than silently pass on a degenerate case.
    let in_mid_05 = -0.0625 * pattern[0]
        + 0.5625 * pattern[1]
        + 0.5625 * pattern[2]
        - 0.0625 * pattern[3];
    let in_mid_025 = -0.0546875 * pattern[0]
        + 0.8203125 * pattern[1]
        + 0.2734375 * pattern[2]
        - 0.0390625 * pattern[3];
    assert!(
        in_mid_05.abs() < ceiling_lin,
        "test pattern broken: midpoint(0.5) = {:.4} should be below ceiling {:.4}",
        in_mid_05.abs(),
        ceiling_lin
    );
    assert!(
        in_mid_025.abs() > ceiling_lin,
        "test pattern broken: midpoint(0.25) = {:.4} should exceed ceiling {:.4}",
        in_mid_025.abs(),
        ceiling_lin
    );

    let mut limiter = Limiter::new(sample_rate, channels, ceiling_dbfs, 3.0, 50.0);
    let cycles = 1024;
    let mut output = Vec::with_capacity(pattern.len() * cycles);
    for _ in 0..cycles {
        for &s in pattern.iter() {
            let mut frame = [s];
            limiter.process_frame_inplace(&mut frame);
            output.push(frame[0]);
        }
    }

    // Skip the lookahead-delayed warmup region (3 ms at 44.1 kHz + slack so
    // the gain envelope finishes settling).
    let warmup = ((3.0e-3 * 44_100.0) as usize) + 32;
    for win in output[warmup..].windows(4) {
        let a = win[0];
        let b = win[1];
        let c = win[2];
        let d = win[3];
        let mid_025 = -0.0546875 * a + 0.8203125 * b + 0.2734375 * c - 0.0390625 * d;
        let mid_050 = -0.0625 * a + 0.5625 * b + 0.5625 * c - 0.0625 * d;
        let mid_075 = -0.0390625 * a + 0.2734375 * b + 0.8203125 * c - 0.0546875 * d;
        for (name, mid) in &[("0.25", mid_025), ("0.5", mid_050), ("0.75", mid_075)] {
            assert!(
                mid.abs() <= ceiling_lin + 0.005,
                "Lagrange-4 midpoint({}) overshoot: {:.4} > ceiling {:.4} on window {:?}",
                name,
                mid.abs(),
                ceiling_lin,
                win
            );
        }
    }
}

#[test]
fn limiter_catches_lagrange_intersample_peak() {
    // Pattern designed so the Lagrange-4 midpoint exceeds the sample peak.
    // Samples [a, b, c, d] = [0, X, X, 0] -> midpoint(b,c) = 0.5625*X + 0.5625*X
    // = 1.125 * X. For X = 0.85 the midpoint reaches 0.956 (above the -1 dBFS
    // ceiling of ~0.891) — yet every individual sample stays under the ceiling.
    let mut settings = default_settings();
    // Skip the chain's gain stage so we test the limiter alone. Set intensity
    // to zero and use the Universal preset so input gain is the preset's small
    // base (~1.5 dB at intensity 0). The signal already crosses the threshold
    // at the midpoint, so the limiter MUST attenuate.
    settings.intensity = 0.0;
    let mut chain = album_mastering_studio_lib::dsp::MasteringChain::new(44_100, 1, &settings);

    let pattern = [0.0_f32, 0.85, 0.85, 0.0];
    let mut samples = Vec::with_capacity(pattern.len() * 1024);
    for _ in 0..1024 {
        samples.extend_from_slice(&pattern);
    }
    chain.process_interleaved(&mut samples, 1);

    // Skip warmup region (limiter lookahead + a few extra frames).
    let warmup = ((3.0e-3 * 44_100.0) as usize) + 32;
    let steady = &samples[warmup..];
    let ceiling_lin = 10.0_f32.powf(-1.0 / 20.0);
    for win in steady.windows(4) {
        let mid =
            -0.0625 * win[0] + 0.5625 * win[1] + 0.5625 * win[2] - 0.0625 * win[3];
        assert!(
            mid.abs() <= ceiling_lin + 0.005,
            "inter-sample peak {mid} (abs {abs}) exceeded ceiling {ceiling_lin}",
            abs = mid.abs(),
        );
    }
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

#[test]
fn user_presets_save_list_delete_roundtrip() {
    use album_mastering_studio_lib::settings;

    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("user_presets.json");

    // Empty / missing file reads as empty list.
    let initial = settings::read_presets(&path).expect("read empty");
    assert!(initial.is_empty());

    // Write two presets.
    let p1 = UserPreset {
        id: "preset-1".to_string(),
        name: "My Loud".to_string(),
        kind: PresetKind::Track,
        settings: default_settings(),
        created_at_iso: "2026-05-11T00:00:00Z".to_string(),
    };
    let p2 = UserPreset {
        id: "preset-2".to_string(),
        name: "Acoustic Light".to_string(),
        kind: PresetKind::Album,
        settings: default_settings(),
        created_at_iso: "2026-05-11T00:00:01Z".to_string(),
    };
    settings::write_presets(&path, &[p1.clone(), p2.clone()]).expect("write");

    // Read back.
    let read = settings::read_presets(&path).expect("read");
    assert_eq!(read.len(), 2);
    assert_eq!(read[0].id, "preset-1");
    assert_eq!(read[1].name, "Acoustic Light");

    // Simulate delete: remove preset-1 and write back.
    let remaining: Vec<UserPreset> =
        read.into_iter().filter(|p| p.id != "preset-1").collect();
    settings::write_presets(&path, &remaining).expect("write after delete");
    let after = settings::read_presets(&path).expect("read after delete");
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].id, "preset-2");
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

/// Phase 12.2 — LUFS landing: when `lufs_offset_db` is set to a target
/// QUIETER than the natural chain output, the rendered file's measured
/// integrated LUFS must land at/below the target (within ±0.5 LU). Verifies
/// the downward-attenuation path of the refuse-upward policy.
#[test]
fn lufs_target_attenuates_loud_render_to_target() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = tmp.path().join("loud.wav");
    // A 1 kHz sine at 0.5 amplitude through Universal/intensity 0.5 produces
    // a master comfortably above -20 LUFS — target -28 forces a meaningful
    // attenuation that we can measure cleanly.
    write_sine_wav(&src, 44_100, 3.0, 1_000.0, 2);

    let mut settings = default_settings();
    settings.advanced.lufs_offset_db = Some(-28.0);

    let job = engine::mastering_render(
        TrackId("loud".to_string()),
        &src,
        &settings,
        tmp.path(),
        RenderKind::Master,
    )
    .expect("render");
    let out_path = Path::new(&job.output_paths[0]);

    let measured = engine::measure_integrated_lufs_at_path(out_path).expect("measure");
    assert!(
        measured.is_finite(),
        "measured LUFS must be finite, got {measured}"
    );
    assert!(
        (measured - (-28.0)).abs() < 0.5,
        "rendered LUFS {} should land within ±0.5 LU of target -28.0",
        measured
    );
}

/// Phase 12.2 — refuse-upward: when `lufs_offset_db` is set to a target
/// LOUDER than the natural chain output, the rendered file's measured LUFS
/// must STAY at the natural chain output (we refuse to amplify past the
/// limiter ceiling). Confirms the policy by rendering twice — once with
/// `lufs_offset_db = Some(target)` set very loud, once with `None` — and
/// asserting the two outputs measure within 0.1 LU of each other.
#[test]
fn lufs_target_refuses_to_amplify_quiet_render() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = tmp.path().join("modest.wav");
    // Very quiet source (0.02 amplitude ≈ -34 dBFS peak). After Custom-preset/
    // intensity-0 the chain barely lifts it, so the natural rendered LUFS will
    // be well below the loud LUFS-target slider's maximum (-6 LUFS) — the
    // refuse-upward branch will fire.
    write_sine_wav_at_amplitude(&src, 44_100, 3.0, 1_000.0, 2, 0.02);

    // Baseline: render with NO target so the chain produces its natural LUFS.
    // Custom preset + intensity 0 gives minimal coloration; lets the source
    // amplitude dominate the loudness.
    let mut baseline_settings = default_settings();
    baseline_settings.preset = Preset::Custom {
        id: "neutral".to_string(),
    };
    baseline_settings.intensity = 0.0;
    let baseline_job = engine::mastering_render(
        TrackId("baseline".to_string()),
        &src,
        &baseline_settings,
        tmp.path(),
        RenderKind::Master,
    )
    .expect("baseline render");
    let baseline_lufs =
        engine::measure_integrated_lufs_at_path(Path::new(&baseline_job.output_paths[0]))
            .expect("baseline measure");

    // With target -6 LUFS (very loud), the chain's natural output should be
    // QUIETER than the target — triggering the refuse-upward branch. The
    // rendered LUFS should equal the baseline within measurement tolerance.
    let mut amplify_settings = baseline_settings.clone();
    amplify_settings.advanced.lufs_offset_db = Some(-6.0);
    let amplify_job = engine::mastering_render(
        TrackId("amplify".to_string()),
        &src,
        &amplify_settings,
        tmp.path(),
        RenderKind::Master,
    )
    .expect("refuse-upward render");
    let refused_lufs =
        engine::measure_integrated_lufs_at_path(Path::new(&amplify_job.output_paths[0]))
            .expect("refuse measure");

    assert!(
        baseline_lufs < -6.0,
        "baseline {} should be quieter than the loud target -6 LUFS (otherwise the \
         refuse-upward branch isn't exercised)",
        baseline_lufs
    );
    assert!(
        (refused_lufs - baseline_lufs).abs() < 0.1,
        "refuse-upward should leave the render unchanged: baseline={}, with target={}",
        baseline_lufs,
        refused_lufs
    );
}

/// Phase 12.2 — end-to-end render comparison. With macro density=1.0 the
/// 5-second loud sine should land at integrated LUFS at least 2 LU lower
/// than at density=0.0. Pins the wiring from `MasteringSettings.advanced.
/// compression_density` all the way through `MasteringChain` and the
/// downstream LUFS measurement on the rendered output. The chain's per-band
/// auto-makeup (half-compensation per the design) partially offsets the raw
/// reduction; the net audible delta on a 1 kHz mid-band signal lands around
/// -2.5 LU at density=1.0 / preset=Custom / intensity=0.0. >=2 LU is well
/// above the loudness just-noticeable threshold.
#[test]
fn mastering_render_with_heavy_compression_attenuates_loud_section() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let in_path = tmp.path().join("loud_sine.wav");
    write_sine_wav(&in_path, 44_100, 5.0, 1_000.0, 2);

    let mut s0 = default_settings();
    s0.preset = Preset::Custom { id: "neutral".to_string() };
    s0.intensity = 0.0;
    s0.advanced.compression_density = Some(0.0);
    let mut s1 = s0.clone();
    s1.advanced.compression_density = Some(1.0);

    let out0_job = engine::mastering_render(
        TrackId("d0".to_string()),
        &in_path,
        &s0,
        tmp.path(),
        RenderKind::Master,
    )
    .expect("render density=0");
    let out1_job = engine::mastering_render(
        TrackId("d1".to_string()),
        &in_path,
        &s1,
        tmp.path(),
        RenderKind::Master,
    )
    .expect("render density=1");

    let lufs0 = engine::measure_integrated_lufs_at_path(Path::new(&out0_job.output_paths[0]))
        .expect("measure d0");
    let lufs1 = engine::measure_integrated_lufs_at_path(Path::new(&out1_job.output_paths[0]))
        .expect("measure d1");
    let delta_lu = lufs1 - lufs0;
    assert!(
        delta_lu <= -2.0,
        "density=1.0 render should be >=2 LU quieter than density=0.0 \
         (got {:.2} LU; d0 LUFS = {}, d1 LUFS = {})",
        delta_lu,
        lufs0,
        lufs1
    );
}

#[tokio::test]
async fn run_export_checks_warns_on_compressed_source_with_heavy_density() {
    let analysis = AnalysisResult {
        track_id: TrackId("stub".to_string()),
        lufs_integrated: -10.0,
        lufs_short_term_max: -8.0,
        true_peak_dbtp: -0.5,
        dynamic_range_lu: 4.0,
        spectral_balance: SpectralBalance {
            low: 0.33,
            mid: 0.34,
            high: 0.33,
        },
        transient_density: 0.5,
        stereo_width: 0.5,
        recommended_universal: default_settings(),
        measured_at_iso: "2026-05-12T12:00:00Z".to_string(),
        inferred_role: None,
        role_confidence: None,
        inferred_character: None,
        character_confidence: None,
        spectral_balance_6band: None,
        transient_flux: None,
        stereo_correlation: None,
        dynamic_range_p95_p10_db: None,
        lufs_short_term_max_3s: None,
        energy_density_score: None,
    };
    let mut settings = default_settings();
    settings.advanced.compression_density = Some(0.5);
    let report = ExportReport {
        track_id: TrackId("t".to_string()),
        output_path: "out.wav".to_string(),
        measured_lufs: -14.0,
        measured_true_peak_dbtp: -1.2,
        measured_dynamic_range_lu: 4.0,
        source_format: "wav".to_string(),
        destination_format: "wav".to_string(),
        sample_rate: 44_100,
        bit_depth: 24,
        checks: Vec::new(),
    };
    let checks = exports::run_export_checks(report, Some(analysis), Some(settings))
        .await
        .expect("checks ok");
    assert!(
        checks
            .iter()
            .any(|c| c.code == "comp_density_on_compressed_source"),
        "expected comp_density_on_compressed_source advisory, got: {:?}",
        checks.iter().map(|c| &c.code).collect::<Vec<_>>()
    );

    // Per-band threshold override should suppress the advisory.
    let mut settings2 = default_settings();
    settings2.advanced.compression_density = Some(0.5);
    settings2.advanced.compression_mid_threshold_db = Some(-30.0);
    let report2 = ExportReport {
        track_id: TrackId("t".to_string()),
        output_path: "out.wav".to_string(),
        measured_lufs: -14.0,
        measured_true_peak_dbtp: -1.2,
        measured_dynamic_range_lu: 4.0,
        source_format: "wav".to_string(),
        destination_format: "wav".to_string(),
        sample_rate: 44_100,
        bit_depth: 24,
        checks: Vec::new(),
    };
    let analysis2 = AnalysisResult {
        track_id: TrackId("stub".to_string()),
        lufs_integrated: -10.0,
        lufs_short_term_max: -8.0,
        true_peak_dbtp: -0.5,
        dynamic_range_lu: 4.0,
        spectral_balance: SpectralBalance {
            low: 0.33,
            mid: 0.34,
            high: 0.33,
        },
        transient_density: 0.5,
        stereo_width: 0.5,
        recommended_universal: default_settings(),
        measured_at_iso: "2026-05-12T12:00:00Z".to_string(),
        inferred_role: None,
        role_confidence: None,
        inferred_character: None,
        character_confidence: None,
        spectral_balance_6band: None,
        transient_flux: None,
        stereo_correlation: None,
        dynamic_range_p95_p10_db: None,
        lufs_short_term_max_3s: None,
        energy_density_score: None,
    };
    let checks2 = exports::run_export_checks(report2, Some(analysis2), Some(settings2))
        .await
        .expect("checks ok");
    assert!(
        !checks2
            .iter()
            .any(|c| c.code == "comp_density_on_compressed_source"),
        "per-band threshold override should suppress the advisory, got: {:?}",
        checks2.iter().map(|c| &c.code).collect::<Vec<_>>()
    );
}

fn default_settings() -> MasteringSettings {
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
        delivery_profile: types::DeliveryProfile::Custom,
        album: None,
        advanced: AdvancedSettings::default(),
    }
}

fn write_sine_wav(path: &Path, sample_rate: u32, duration_sec: f32, freq: f32, channels: u16) {
    write_sine_wav_at_amplitude(path, sample_rate, duration_sec, freq, channels, 0.5);
}

fn write_sine_wav_at_amplitude(
    path: &Path,
    sample_rate: u32,
    duration_sec: f32,
    freq: f32,
    channels: u16,
    amplitude_lin: f32,
) {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("wav create");
    let n = (sample_rate as f32 * duration_sec) as u32;
    let amplitude = (amplitude_lin * i16::MAX as f32) as i16;
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
    // Scan `private-audio-fixtures/` for any supported audio file rather than
    // hard-coding one specific filename. Tests can now run on whatever local
    // fixture Dan drops in — original name preserved, no renaming required.
    // Both paths are checked so the harness works from either the workspace
    // root or from `src-tauri/` (cargo runs tests with cwd = src-tauri).
    const FIXTURE_DIRS: [&str; 2] =
        ["../private-audio-fixtures", "private-audio-fixtures"];
    const EXTENSIONS: [&str; 8] =
        ["wav", "mp3", "flac", "m4a", "aac", "ogg", "opus", "aiff"];

    for dir in &FIXTURE_DIRS {
        let dir_path = PathBuf::from(dir);
        if !dir_path.is_dir() {
            continue;
        }
        // Sort entries so the choice is deterministic across runs when more
        // than one fixture is present.
        let mut entries: Vec<_> = match std::fs::read_dir(&dir_path) {
            Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
            Err(_) => continue,
        };
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let p = entry.path();
            let Some(ext) = p.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()) {
                return Some(p);
            }
        }
    }
    None
}

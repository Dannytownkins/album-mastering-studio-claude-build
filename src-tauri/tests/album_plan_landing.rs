//! B6 follow-up: album-plan (user-facing Album Master) render path
//! must apply ceiling-bounded LUFS landing per track — not the old
//! refuse-upward gate that was missed when B6 landed for track-render
//! and album-simple.
//!
//! This test renders a single-track album plan whose source is quiet
//! enough that the chain produces post-chain LUFS well below the
//! delivery profile target. Pre-fix, the refuse-upward gate would
//! leave the file untouched; post-fix, the ceiling-bounded landing
//! pushes it upward to target without exceeding the true-peak ceiling.

use album_mastering_studio_lib::album_render::render_album_plan_impl;
use album_mastering_studio_lib::engine::{
    measure_integrated_lufs_at_path, AlbumPlanRenderRequest, AlbumTrackRenderInput,
};
use album_mastering_studio_lib::types::{
    AdvancedSettings, AlbumArc, AlbumPlan, AlbumTrackEntry, DeliveryProfile,
    MasteringSettings, Preset, TrackId, TrackRole,
};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn write_stereo_sine_wav(
    path: &PathBuf,
    sample_rate: u32,
    duration_sec: f32,
    amplitude: f32,
) {
    let spec = WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec).expect("create wav");
    let n_frames = (sample_rate as f32 * duration_sec) as u32;
    let omega = 2.0 * std::f32::consts::PI * 1_000.0 / sample_rate as f32;
    for i in 0..n_frames {
        let v = amplitude * (omega * i as f32).sin();
        let s = (v.clamp(-1.0, 1.0) * 32_767.0).round() as i16;
        writer.write_sample(s).expect("write L");
        writer.write_sample(s).expect("write R");
    }
    writer.finalize().expect("finalize source wav");
}

fn streaming_settings() -> MasteringSettings {
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
        delivery_profile: DeliveryProfile::StreamingUniversal, // -14 LUFS / -1 dBTP
        album: None,
        advanced: AdvancedSettings::default(),
    }
}

/// Read the sample peak of a WAV at the given path in dBFS, supporting
/// both float and int sample formats. Approximates true peak for the
/// sine sources this test uses (sample vs true peak parity within ~0.1
/// dB for narrowband signals).
fn wav_sample_peak_dbfs(path: &Path) -> f32 {
    let mut reader = hound::WavReader::open(path).expect("open wav");
    let spec = reader.spec();
    let peak_lin = match spec.sample_format {
        SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.expect("read sample").abs())
            .fold(0.0_f32, f32::max),
        SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| (s.expect("read sample") as f32 / scale).abs())
                .fold(0.0_f32, f32::max)
        }
    };
    if peak_lin > 0.0 {
        20.0 * peak_lin.log10()
    } else {
        -120.0
    }
}

#[test]
fn album_plan_pushes_quiet_track_upward_to_target_bounded_by_ceiling() {
    let tmp = TempDir::new().expect("tempdir");
    let sr: u32 = 48_000;

    // Quiet sine (-20 dBFS peak ≈ -23 LUFS). Universal/intensity 0.5
    // produces post-chain LUFS well below the Streaming -14 target
    // with plenty of true-peak headroom — the upward-bounded-by-ceiling
    // path the patch needs to exercise.
    let path = tmp.path().join("quiet.wav");
    write_stereo_sine_wav(&path, sr, 4.0, 0.10);

    // Single-track plan with Custom arc + zero LUFS offsets so no arc
    // modulation muddies the assertion. AlbumTrackEntry has no character
    // bias either, so apply_album_shadow leaves the effective target at
    // the delivery profile's -14 LUFS.
    let plan = AlbumPlan {
        title: "Quiet Album Plan Landing Test".to_string(),
        arc: AlbumArc::Custom {
            lufs_offsets: vec![0.0],
        },
        tracks: vec![AlbumTrackEntry {
            track_id: TrackId("quiet".to_string()),
            position: 1,
            role: TrackRole::AlbumTrack,
            role_locked: false,
            arc_lufs_offset_db: 0.0,
            intensity_scale: 1.0,
            album_character: None,
        }],
        transitions: vec![],
        intensity: 1.0,
    };

    let request = AlbumPlanRenderRequest {
        plan,
        tracks: vec![AlbumTrackRenderInput {
            track_id: TrackId("quiet".to_string()),
            source_path: path.to_string_lossy().to_string(),
            settings: streaming_settings(),
        }],
    };

    let out_dir = tmp.path().join("out");
    let report =
        render_album_plan_impl(&request, &out_dir, None).expect("album plan render");

    assert_eq!(report.tracks.len(), 1);
    let per_track_path = Path::new(&report.tracks[0].output_path);

    // Upward landing case: chain produced ~-22 LUFS, target -14 → +8 dB
    // push. Headroom from -20 dBFS source peak to -1 dBTP ceiling is
    // ~+19 dB, so the full push is allowed. File should land near -14.
    let measured =
        measure_integrated_lufs_at_path(per_track_path).expect("measure per-track");
    assert!(
        (measured - (-14.0)).abs() < 0.5,
        "B6 album-plan: quiet track should be lifted upward to target -14 LUFS \
         (Streaming profile), got {measured:.2} LUFS"
    );

    // Ceiling protection: even with upward push, the final sample peak
    // must stay at or below -1 dBFS (the StreamingUniversal ceiling
    // within sine sample/true-peak parity).
    let peak_dbfs = wav_sample_peak_dbfs(per_track_path);
    assert!(
        peak_dbfs <= -1.0 + 0.5,
        "album-plan peak should stay near or below the -1 dBTP ceiling, got \
         {peak_dbfs:.2} dBFS"
    );

    // And the report's recorded per-track measured_lufs should also
    // reflect the landed value (within the same tolerance).
    let recorded = report.tracks[0].measured_lufs;
    assert!(
        (recorded - (-14.0)).abs() < 0.5,
        "AlbumTrackRenderRecord.measured_lufs should match the landed file, got \
         {recorded:.2}"
    );
}

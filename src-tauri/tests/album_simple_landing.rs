//! B5: album-simple (legacy) render path must apply per-track LUFS
//! landing matching the user's delivery profile target.
//!
//! Pre-B5, `album_render_with_progress` ran the chain on each track
//! and wrote the WAV without ever measuring or correcting integrated
//! LUFS — the user's delivery-profile / advanced.lufs_offset_db
//! target was silently ignored on this code path while the
//! track-render and album-plan paths both did the right thing.
//!
//! This test renders two stereo sine sources at deliberately different
//! amplitudes (one well above target, one well below) through a
//! `StreamingUniversal` profile (-14 LUFS / -1 dBTP) and asserts each
//! per-track WAV lands within ±0.5 LU of -14 — proving both the
//! downward attenuation case and the upward-bounded push case run.

use album_mastering_studio_lib::album_render::album_render_with_progress;
use album_mastering_studio_lib::engine::{
    measure_integrated_lufs_at_path, AlbumRenderRequest, AlbumTrackInput,
};
use album_mastering_studio_lib::types::{
    AdvancedSettings, DeliveryProfile, MasteringSettings, Preset, TrackId,
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

fn streaming_album_intent() -> MasteringSettings {
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

#[test]
fn album_simple_lands_each_track_at_streaming_profile_target() {
    let tmp = TempDir::new().expect("tempdir");
    let sr: u32 = 48_000;

    // Track A: hot source (-10.5 dBFS sine ≈ -13 LUFS pre-chain). Chain
    // at Universal/intensity 0.5 produces output above -14 → downward
    // attenuation case for the landing block.
    let path_loud = tmp.path().join("track_loud.wav");
    write_stereo_sine_wav(&path_loud, sr, 4.0, 0.30);

    // Track B: quiet source (-20 dBFS sine ≈ -23 LUFS pre-chain). Chain
    // output sits below -14 → upward (ceiling-bounded) push case.
    let path_quiet = tmp.path().join("track_quiet.wav");
    write_stereo_sine_wav(&path_quiet, sr, 4.0, 0.10);

    let request = AlbumRenderRequest {
        tracks: vec![
            AlbumTrackInput {
                id: TrackId("track_loud".to_string()),
                path: path_loud.to_string_lossy().to_string(),
            },
            AlbumTrackInput {
                id: TrackId("track_quiet".to_string()),
                path: path_quiet.to_string_lossy().to_string(),
            },
        ],
        album_intent: streaming_album_intent(),
        per_track_overrides: None,
    };

    let out_dir = tmp.path().join("out");
    std::fs::create_dir_all(&out_dir).expect("create out dir");

    let job =
        album_render_with_progress(&request, &out_dir, None).expect("album render");

    // output_paths[0] is the album.wav; per-track paths follow.
    assert!(
        job.output_paths.len() >= 3,
        "expected album.wav + 2 per-track WAVs, got {}",
        job.output_paths.len()
    );

    for individual_path in &job.output_paths[1..] {
        let lufs = measure_integrated_lufs_at_path(Path::new(individual_path))
            .expect("measure per-track WAV");
        assert!(
            (lufs - (-14.0)).abs() < 0.5,
            "B5: per-track WAV {individual_path} must land within ±0.5 LU of \
             StreamingUniversal target -14 LUFS, got {lufs:.2} LUFS"
        );
    }
}

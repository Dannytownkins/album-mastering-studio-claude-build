//! Phase B+ Step 8.3 — Delivery profile end-to-end.
//!
//! Renders a known-loud sine through every `DeliveryProfile` (the seven
//! non-Custom variants plus an explicit Custom) and asserts the
//! `effective_*` shadow plumbing from `MasteringSettings` →
//! `ChainCoeffs` → `mastering_render_with_progress` reaches the WAV
//! file: the measured integrated LUFS lands within ±1 LU of each
//! profile's documented target, and the WAV's bit-depth matches the
//! profile's `output_bit_depth()` (or the explicit Custom value).

use std::path::Path;

use album_mastering_studio_lib::engine;
use album_mastering_studio_lib::types::{
    AdvancedSettings, DeliveryProfile, MasteringSettings, Preset, RenderKind, TrackId,
};

const SR_HZ: u32 = 48_000;
const DURATION_SEC: f32 = 5.0;
const STEREO: u16 = 2;
const SINE_AMP: f32 = 0.3; // ≈ -10.46 dBFS sample peak
const LUFS_TOLERANCE: f32 = 1.0;

/// Write a deterministic stereo 1 kHz sine to a 16-bit WAV at the given
/// path. The chain's input decoder handles 16-bit Int just fine; we use
/// a fixed source format so the test isn't sensitive to writer details.
fn write_sine_source(path: &Path, sample_rate: u32, duration_sec: f32, freq_hz: f32) {
    let spec = hound::WavSpec {
        channels: STEREO,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create source wav");
    let n = (sample_rate as f32 * duration_sec) as u32;
    for i in 0..n {
        let t = i as f32 / sample_rate as f32;
        let s = SINE_AMP * (t * 2.0 * std::f32::consts::PI * freq_hz).sin();
        let pcm = (s * i16::MAX as f32) as i16;
        // Same value to L and R for a fully correlated stereo source.
        writer.write_sample(pcm).expect("write L");
        writer.write_sample(pcm).expect("write R");
    }
    writer.finalize().expect("finalize source wav");
}

fn settings_for(profile: DeliveryProfile, custom_advanced: Option<AdvancedSettings>) -> MasteringSettings {
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
        delivery_profile: profile,
        album: None,
        advanced: custom_advanced.unwrap_or_default(),
    }
}

fn render_and_assert(
    label: &str,
    settings: MasteringSettings,
    expected_lufs: f32,
    expected_bit_depth: u16,
) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = tmp.path().join("delivery-profile-source.wav");
    write_sine_source(&src, SR_HZ, DURATION_SEC, 1_000.0);

    let job = engine::mastering_render(
        TrackId(format!("delivery-{label}")),
        &src,
        &settings,
        tmp.path(),
        RenderKind::Master,
    )
    .unwrap_or_else(|e| panic!("{label}: render failed: {e:?}"));
    let out_path = Path::new(
        job.output_paths
            .first()
            .unwrap_or_else(|| panic!("{label}: no output path")),
    );
    assert!(out_path.exists(), "{label}: rendered WAV does not exist");

    // Measure from the WAV file (not the RenderJob.measurements) so we're
    // verifying what actually landed on disk after the writer's bit-depth
    // quantization + TPDF dither.
    let measured_lufs =
        engine::measure_integrated_lufs_at_path(out_path).expect("measure rendered LUFS");
    assert!(
        (measured_lufs - expected_lufs).abs() < LUFS_TOLERANCE,
        "{label}: rendered LUFS {measured_lufs:.2} not within ±{LUFS_TOLERANCE:.1} LU of target {expected_lufs}",
    );

    let reader = hound::WavReader::open(out_path).expect("open rendered wav");
    let spec = reader.spec();
    assert_eq!(
        spec.bits_per_sample, expected_bit_depth,
        "{label}: rendered bit-depth {} does not match profile target {expected_bit_depth}",
        spec.bits_per_sample,
    );
}

#[test]
fn delivery_profile_streaming_universal_lands_minus14_24bit() {
    render_and_assert(
        "StreamingUniversal",
        settings_for(DeliveryProfile::StreamingUniversal, None),
        -14.0,
        24,
    );
}

#[test]
fn delivery_profile_apple_music_lands_minus16_24bit() {
    render_and_assert(
        "AppleMusic",
        settings_for(DeliveryProfile::AppleMusic, None),
        -16.0,
        24,
    );
}

#[test]
fn delivery_profile_cd_lands_minus14_16bit() {
    render_and_assert("Cd", settings_for(DeliveryProfile::Cd, None), -14.0, 16);
}

#[test]
fn delivery_profile_vinyl_premaster_lands_minus18_24bit() {
    render_and_assert(
        "VinylPremaster",
        settings_for(DeliveryProfile::VinylPremaster, None),
        -18.0,
        24,
    );
}

#[test]
fn delivery_profile_loud_rock_lands_minus10p5_24bit() {
    render_and_assert(
        "LoudRock",
        settings_for(DeliveryProfile::LoudRock, None),
        -10.5,
        24,
    );
}

#[test]
fn delivery_profile_broadcast_eu_lands_minus23_24bit() {
    render_and_assert(
        "BroadcastEu",
        settings_for(DeliveryProfile::BroadcastEu, None),
        -23.0,
        24,
    );
}

#[test]
fn delivery_profile_broadcast_us_lands_minus24_24bit() {
    render_and_assert(
        "BroadcastUs",
        settings_for(DeliveryProfile::BroadcastUs, None),
        -24.0,
        24,
    );
}

#[test]
fn delivery_profile_custom_honors_explicit_advanced_fields() {
    // Custom does NOT shadow — the renderer reads
    // advanced.lufs_offset_db and advanced.bit_depth verbatim.
    let advanced = AdvancedSettings {
        lufs_offset_db: Some(-12.0),
        bit_depth: Some(16),
        ..Default::default()
    };
    render_and_assert(
        "Custom",
        settings_for(DeliveryProfile::Custom, Some(advanced)),
        -12.0,
        16,
    );
}

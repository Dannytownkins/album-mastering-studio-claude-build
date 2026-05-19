//! Export must ignore Volume Match.
//!
//! Volume Match is an A/B playback aid only. These tests render the same source
//! with VM on/off and assert the written float WAV samples match exactly.

use std::path::Path;

use album_mastering_studio_lib::engine;
use album_mastering_studio_lib::types::{
    AdvancedSettings, DeliveryProfile, MasteringSettings, Preset, RenderKind, TrackId,
};

const SR_HZ: u32 = 48_000;
const DURATION_SEC: f32 = 2.0;
const STEREO: u16 = 2;

fn write_float_sine(path: &Path) {
    let spec = hound::WavSpec {
        channels: STEREO,
        sample_rate: SR_HZ,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create source wav");
    let n_frames = (SR_HZ as f32 * DURATION_SEC) as u32;
    for i in 0..n_frames {
        let t = i as f32 / SR_HZ as f32;
        let s = 0.25 * (t * 2.0 * std::f32::consts::PI * 110.0).sin()
            + 0.08 * (t * 2.0 * std::f32::consts::PI * 1_700.0).sin();
        writer.write_sample(s).expect("write L");
        writer.write_sample(s * 0.85).expect("write R");
    }
    writer.finalize().expect("finalize source wav");
}

fn oomph_settings(volume_match: bool) -> MasteringSettings {
    MasteringSettings {
        preset: Preset::Oomph,
        intensity: 1.0,
        eq_sub_db: 0.0,
        eq_low_db: 0.0,
        eq_low_mid_db: 0.0,
        eq_mid_db: 0.0,
        eq_high_mid_db: 0.0,
        eq_high_db: 0.0,
        eq_sparkle_db: 0.0,
        volume_match,
        source_lufs_integrated: Some(-13.0),
        input_gain_db: 0.0,
        output_gain_db: 0.0,
        delivery_profile: DeliveryProfile::Custom,
        album: None,
        advanced: AdvancedSettings {
            bit_depth: Some(32),
            ..Default::default()
        },
    }
}

fn read_float_wav(path: &Path) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).expect("open rendered wav");
    let spec = reader.spec();
    assert_eq!(spec.sample_format, hound::SampleFormat::Float);
    reader
        .samples::<f32>()
        .map(|s| s.expect("read sample"))
        .collect()
}

#[test]
fn track_export_is_identical_with_volume_match_on_or_off() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = tmp.path().join("source.wav");
    write_float_sine(&src);

    let off_job = engine::mastering_render(
        TrackId("vm-off".to_string()),
        &src,
        &oomph_settings(false),
        tmp.path(),
        RenderKind::Master,
    )
    .expect("render vm off");
    let on_job = engine::mastering_render(
        TrackId("vm-on".to_string()),
        &src,
        &oomph_settings(true),
        tmp.path(),
        RenderKind::Master,
    )
    .expect("render vm on");

    let off = read_float_wav(Path::new(&off_job.output_paths[0]));
    let on = read_float_wav(Path::new(&on_job.output_paths[0]));
    assert_eq!(off.len(), on.len());
    assert_eq!(off, on, "Volume Match changed exported audio samples");
}

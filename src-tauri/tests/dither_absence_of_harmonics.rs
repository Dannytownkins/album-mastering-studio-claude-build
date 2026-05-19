//! Phase B+ Step 8.6 — TPDF dither absence-of-harmonics.
//!
//! The existing engine.rs `tpdf_dither_decorrelates_quantization_at_minus_90_dbfs`
//! unit test checks how many distinct integer values the dithered output
//! occupies. This stronger version asserts the textbook *spectral*
//! property of dither: a -90 dBFS-RMS sine driven through the 16-bit
//! TPDF-dithered writer produces a clean fundamental and *no* clearly
//! resolved odd / even harmonics above the noise floor.
//!
//! Without dither, low-level quantization on integer output produces
//! distorted harmonics correlated with the signal. With dither, the
//! quantization error becomes uncorrelated white noise — the signal
//! still rides above the floor but its harmonics do not.

use std::path::Path;

use album_mastering_studio_lib::engine;
use album_mastering_studio_lib::types::{
    AdvancedSettings, DeliveryProfile, MasteringSettings, Preset, RenderKind, TrackId,
};

const SR_HZ: u32 = 48_000;
const DURATION_SEC: f32 = 1.0;
const STEREO: u16 = 2;
/// -90 dBFS *RMS*; for a sine, peak = RMS · √2 → -86.99 dBFS peak.
const SIGNAL_LUFS_PEAK: f32 = 1.4142_f32 * 3.162_277_7e-5_f32;

fn neutral_settings_16bit() -> MasteringSettings {
    // Custom preset + zero intensity + zero EQ + zero gain so the chain
    // is approximately identity at this signal level; bit_depth = 16 so
    // the writer applies TPDF dither.
    MasteringSettings {
        preset: Preset::Custom {
            id: "dither-test".to_string(),
        },
        intensity: 0.0,
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
        advanced: AdvancedSettings {
            bit_depth: Some(16),
            ..Default::default()
        },
    }
}

/// Write a 32-bit float WAV so the source signal isn't quantized before
/// the chain sees it. Stereo with L = R; the dither test only inspects
/// the left channel after the render.
fn write_float_sine(path: &Path, sample_rate: u32, duration_sec: f32, freq_hz: f32, peak: f32) {
    let spec = hound::WavSpec {
        channels: STEREO,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create source wav");
    let n_frames = (sample_rate as f32 * duration_sec) as u32;
    let omega = 2.0 * std::f32::consts::PI * freq_hz / sample_rate as f32;
    for i in 0..n_frames {
        let s = peak * (omega * i as f32).sin();
        writer.write_sample(s).expect("write L");
        writer.write_sample(s).expect("write R");
    }
    writer.finalize().expect("finalize source wav");
}

fn read_left_channel_int16(path: &Path) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).expect("open rendered wav");
    let spec = reader.spec();
    assert_eq!(spec.bits_per_sample, 16, "test requires 16-bit rendered WAV");
    let channels = spec.channels as usize;
    let scale = 1.0 / i16::MAX as f32;
    let mut left = Vec::with_capacity(reader.duration() as usize);
    let mut frame: Vec<i32> = Vec::with_capacity(channels);
    for s in reader.samples::<i32>() {
        let v = s.expect("read int sample");
        frame.push(v);
        if frame.len() == channels {
            left.push(frame[0] as f32 * scale);
            frame.clear();
        }
    }
    left
}

fn goertzel_mag_db(samples: &[f32], sample_rate: f32, freq_hz: f32) -> f32 {
    let omega = 2.0 * std::f32::consts::PI * freq_hz / sample_rate;
    let coeff = 2.0 * omega.cos();
    let mut q1 = 0.0_f32;
    let mut q2 = 0.0_f32;
    for &s in samples {
        let q0 = coeff * q1 - q2 + s;
        q2 = q1;
        q1 = q0;
    }
    let mag = (q1 * q1 + q2 * q2 - coeff * q1 * q2).max(1e-30).sqrt();
    20.0 * (mag / samples.len() as f32).log10()
}

#[test]
fn dithered_16bit_render_of_minus_90_dbfs_sine_shows_no_harmonics() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = tmp.path().join("dither-source.wav");
    write_float_sine(&src, SR_HZ, DURATION_SEC, 1_000.0, SIGNAL_LUFS_PEAK);

    let job = engine::mastering_render(
        TrackId("dither-test".to_string()),
        &src,
        &neutral_settings_16bit(),
        tmp.path(),
        RenderKind::Master,
    )
    .expect("render dithered 16-bit wav");
    let out_path = Path::new(&job.output_paths[0]);
    let samples = read_left_channel_int16(out_path);
    assert!(
        !samples.is_empty(),
        "expected non-empty rendered samples; got 0",
    );

    let sr_f = SR_HZ as f32;
    let fundamental = goertzel_mag_db(&samples, sr_f, 1_000.0);
    let harmonics: [(f32, f32); 4] = [
        (2_000.0, goertzel_mag_db(&samples, sr_f, 2_000.0)),
        (3_000.0, goertzel_mag_db(&samples, sr_f, 3_000.0)),
        (4_000.0, goertzel_mag_db(&samples, sr_f, 4_000.0)),
        (5_000.0, goertzel_mag_db(&samples, sr_f, 5_000.0)),
    ];

    // Noise floor: average of seven well-separated non-harmonic, non-
    // commensurate frequencies. These dodge both the fundamental's
    // harmonics AND its subharmonic intermodulation products.
    let floor_probes = [1_234.0, 2_345.0, 4_567.0, 6_789.0, 8_901.0, 11_000.0, 14_500.0];
    let floor: f32 = floor_probes
        .iter()
        .map(|&f| goertzel_mag_db(&samples, sr_f, f))
        .sum::<f32>()
        / floor_probes.len() as f32;

    assert!(
        fundamental > floor + 15.0,
        "1 kHz fundamental ({fundamental:.2} dB) should rise >= 15 dB above the noise floor ({floor:.2} dB); got delta = {:.2} dB",
        fundamental - floor,
    );
    for (freq, mag) in harmonics {
        let delta = mag - floor;
        assert!(
            delta < 6.0,
            "harmonic at {freq:.0} Hz ({mag:.2} dB) sits {delta:.2} dB above noise floor ({floor:.2} dB) — expected < +6 dB. TPDF dither appears to have failed to decorrelate quantization distortion.",
        );
    }
}

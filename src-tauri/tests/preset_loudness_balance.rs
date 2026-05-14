//! Phase B+ Step 8.2 — Inter-preset loudness consistency.
//!
//! Catches the regression Dan flagged in the 2026-05-12 listening pass
//! ("Tape is dramatically louder than every other preset"). Pushes the
//! same pink-ish noise burst through every named preset at intensity
//! 0.5 and asserts the spread of post-chain integrated LUFS values stays
//! under 4 LU. The Codex preset calibration was deliberately tuned to be
//! roughly loudness-matched at default intensity; this guard fires when
//! a future calibration change accidentally breaks that property.

use album_mastering_studio_lib::dsp::MasteringChain;
use album_mastering_studio_lib::engine::measure_integrated_lufs;
use album_mastering_studio_lib::types::{
    AdvancedSettings, DeliveryProfile, MasteringSettings, Preset,
};

const SR_HZ: u32 = 48_000;
const DURATION_SEC: f32 = 2.0;
const STEREO: usize = 2;
const TEST_INTENSITY: f32 = 0.5;
const NOISE_PEAK: f32 = 0.251; // -12.04 dBFS

const PRESETS: &[(&str, Preset)] = &[
    ("Universal", Preset::Universal),
    ("Clarity", Preset::Clarity),
    ("Tape", Preset::Tape),
    ("Spatial", Preset::Spatial),
    ("Oomph", Preset::Oomph),
    ("Warmth", Preset::Warmth),
    ("Punch", Preset::Punch),
    ("Loud", Preset::Loud),
];

/// Paul Kellet six-stage pinking IIR fed by a deterministic LCG. Returns
/// a stereo interleaved buffer scaled so the sample peak (across both
/// channels) is exactly NOISE_PEAK. The same signal is routed to L and R
/// so the chain sees a perfectly correlated stereo source.
fn synth_pink_stereo(samples_per_channel: usize) -> Vec<f32> {
    let mut state: u32 = 0xCAFE_BABE;
    let mut b0p = 0.0_f32;
    let mut b1p = 0.0_f32;
    let mut b2p = 0.0_f32;
    let mut b3p = 0.0_f32;
    let mut b4p = 0.0_f32;
    let mut b5p = 0.0_f32;

    let mut pink_mono = Vec::with_capacity(samples_per_channel);
    for _ in 0..samples_per_channel {
        state = state.wrapping_mul(1_103_515_245).wrapping_add(12345);
        let w = ((state >> 16) & 0x7FFF) as f32 / 32_768.0 - 0.5;
        b0p = 0.99886 * b0p + w * 0.0555179;
        b1p = 0.99332 * b1p + w * 0.0750759;
        b2p = 0.96900 * b2p + w * 0.1538520;
        b3p = 0.86650 * b3p + w * 0.3104856;
        b4p = 0.55000 * b4p + w * 0.5329522;
        b5p = -0.7616 * b5p - w * 0.0168980;
        let b6p = w * 0.115926;
        let p = b0p + b1p + b2p + b3p + b4p + b5p + w * 0.5362 + b6p;
        pink_mono.push(p);
    }

    let measured_peak = pink_mono
        .iter()
        .map(|s| s.abs())
        .fold(0.0_f32, f32::max)
        .max(f32::MIN_POSITIVE);
    let scale = NOISE_PEAK / measured_peak;

    let mut interleaved = Vec::with_capacity(samples_per_channel * STEREO);
    for s in pink_mono {
        let scaled = s * scale;
        interleaved.push(scaled);
        interleaved.push(scaled);
    }
    interleaved
}

fn default_settings_for(preset: Preset) -> MasteringSettings {
    MasteringSettings {
        preset,
        intensity: TEST_INTENSITY,
        eq_low_db: 0.0,
        eq_low_mid_db: 0.0,
        eq_mid_db: 0.0,
        eq_high_db: 0.0,
        volume_match: false,
        source_lufs_integrated: None,
        input_gain_db: 0.0,
        output_gain_db: 0.0,
        delivery_profile: DeliveryProfile::Custom,
        album: None,
        advanced: AdvancedSettings::default(),
    }
}

fn run_through_chain(input: &[f32], preset: Preset) -> Vec<f32> {
    let settings = default_settings_for(preset);
    let mut chain = MasteringChain::new(SR_HZ, STEREO, &settings);
    let mut buf = input.to_vec();
    chain.process_interleaved(&mut buf, STEREO);
    buf
}

#[test]
fn presets_land_within_4_lu_of_each_other_at_default_intensity() {
    let samples_per_channel = (SR_HZ as f32 * DURATION_SEC) as usize;
    let input = synth_pink_stereo(samples_per_channel);

    let mut readings: Vec<(&str, f32)> = Vec::with_capacity(PRESETS.len());
    for (name, preset) in PRESETS {
        let output = run_through_chain(&input, preset.clone());
        let lufs = measure_integrated_lufs(&output, SR_HZ, STEREO as u16)
            .expect("integrated LUFS measurement should succeed on a 2 s stereo buffer");
        readings.push((*name, lufs));
    }

    let (loudest_name, loudest) = readings
        .iter()
        .cloned()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();
    let (quietest_name, quietest) = readings
        .iter()
        .cloned()
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();
    let spread = loudest - quietest;

    let detail = readings
        .iter()
        .map(|(n, v)| format!("{n}={v:.2}"))
        .collect::<Vec<_>>()
        .join(", ");

    assert!(
        spread < 4.0,
        "preset loudness spread = {spread:.2} LU ({loudest_name} {loudest:.2} → {quietest_name} {quietest:.2}); preset calibration appears unbalanced. Full readings: {detail}",
    );
}

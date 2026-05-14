//! Phase B+ Step 8.1 — Per-preset character signature.
//!
//! Pushes a deterministic 2 s noise burst (and, for saturation checks, a
//! 1 kHz sine) through every named `MasteringChain` preset at default
//! intensity 0.5 and asserts the post-chain Goertzel magnitudes at the
//! four band-tuning frequencies move in the direction the preset's
//! calibration tuple promises. Catches "did I wire preset X to the wrong
//! calibration row" regressions before they ship to Dan's ears.
//!
//! Implementation note re: the plan spec. The plan's first draft framed
//! assertions as "band X is ≥ N dB above the input." That formulation
//! folds in the chain's broadband makeup+limiter gain (~+4 dB at
//! intensity 0.5) and made "neutral bands within ±0.5 dB" unreachable —
//! the chain is designed to push toward target loudness, so every band
//! sees that broadband lift. The reformulation here asserts
//! **between-band tilts** (chain_gain at the boosted band minus
//! chain_gain at the cut/neutral band). That cancels the broadband
//! offset and tests the same property the plan intended: each preset's
//! EQ calibration is wired through and produces the documented shape.

use album_mastering_studio_lib::dsp::MasteringChain;
use album_mastering_studio_lib::types::{
    AdvancedSettings, DeliveryProfile, MasteringSettings, Preset,
};

const SR_HZ: u32 = 48_000;
const DURATION_SEC: f32 = 2.0;
const STEREO: usize = 2;
const TEST_INTENSITY: f32 = 0.5;
const NOISE_PEAK: f32 = 0.251; // -12.04 dBFS

/// Deterministic LCG → uniform white in [-0.5, 0.5); scaled to NOISE_PEAK
/// so amplitude is reproducible across machines.
fn synth_noise_stereo(samples_per_channel: usize) -> Vec<f32> {
    let mut state: u32 = 0xCAFE_BABE;
    let mut out = Vec::with_capacity(samples_per_channel * STEREO);
    for _ in 0..samples_per_channel {
        state = state.wrapping_mul(1_103_515_245).wrapping_add(12345);
        let raw = (((state >> 16) & 0x7FFF) as f32 / 32_768.0) - 0.5;
        let s = raw * (NOISE_PEAK / 0.5);
        out.push(s);
        out.push(s);
    }
    out
}

fn synth_sine_stereo(samples_per_channel: usize, freq_hz: f32) -> Vec<f32> {
    let mut out = Vec::with_capacity(samples_per_channel * STEREO);
    let dt = 1.0 / SR_HZ as f32;
    for n in 0..samples_per_channel {
        let s = 0.5 * (2.0 * std::f32::consts::PI * freq_hz * (n as f32) * dt).sin();
        out.push(s);
        out.push(s);
    }
    out
}

fn synth_antiphase_stereo(samples_per_channel: usize) -> Vec<f32> {
    let mut state: u32 = 0xCAFE_BABE;
    let mut out = Vec::with_capacity(samples_per_channel * STEREO);
    for _ in 0..samples_per_channel {
        state = state.wrapping_mul(1_103_515_245).wrapping_add(12345);
        let raw = (((state >> 16) & 0x7FFF) as f32 / 32_768.0) - 0.5;
        let s = raw * (NOISE_PEAK / 0.5);
        out.push(s);
        out.push(-s);
    }
    out
}

fn left_channel(interleaved: &[f32]) -> Vec<f32> {
    interleaved.iter().step_by(STEREO).copied().collect()
}

fn side_rms(interleaved: &[f32]) -> f32 {
    let mut sum_sq = 0.0_f64;
    let mut n = 0_usize;
    for frame in interleaved.chunks_exact(STEREO) {
        let s = (frame[0] - frame[1]) * 0.5;
        sum_sq += (s as f64) * (s as f64);
        n += 1;
    }
    if n == 0 {
        return 0.0;
    }
    ((sum_sq / n as f64).sqrt()) as f32
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

/// Chain gain at a given frequency, i.e. how many dB the chain
/// added/subtracted to that band relative to the input.
fn chain_gain_db(input_mono: &[f32], output_mono: &[f32], freq_hz: f32) -> f32 {
    goertzel_mag_db(output_mono, SR_HZ as f32, freq_hz)
        - goertzel_mag_db(input_mono, SR_HZ as f32, freq_hz)
}

/// Between-band tilt: chain_gain(high) - chain_gain(low), in dB. Cancels
/// the chain's broadband makeup gain so the assertion isolates the
/// preset's EQ-shape contribution.
fn tilt_db(input_mono: &[f32], output_mono: &[f32], high_hz: f32, low_hz: f32) -> f32 {
    chain_gain_db(input_mono, output_mono, high_hz)
        - chain_gain_db(input_mono, output_mono, low_hz)
}

/// Diagnostic helper — run with `cargo test --test preset_signature
/// dump_observed_tilts -- --ignored --nocapture` to see the actual
/// chain-gain tilts every preset produces under the test signal.
#[test]
#[ignore]
fn dump_observed_tilts() {
    let samples_per_channel = (SR_HZ as f32 * DURATION_SEC) as usize;
    let input = synth_noise_stereo(samples_per_channel);
    let input_left = left_channel(&input);
    let bands = [200.0_f32, 400.0, 1_500.0, 6_000.0];
    let presets = [
        ("Universal", Preset::Universal),
        ("Clarity", Preset::Clarity),
        ("Tape", Preset::Tape),
        ("Spatial", Preset::Spatial),
        ("Oomph", Preset::Oomph),
        ("Warmth", Preset::Warmth),
        ("Punch", Preset::Punch),
        ("Loud", Preset::Loud),
    ];
    for (name, preset) in presets.iter() {
        let output = run_through_chain(&input, preset.clone());
        let output_left = left_channel(&output);
        let gains: Vec<f32> = bands
            .iter()
            .map(|f| chain_gain_db(&input_left, &output_left, *f))
            .collect();
        let avg = gains.iter().sum::<f32>() / gains.len() as f32;
        println!(
            "{name:9}: gain(dB) @200={:+.2} @400={:+.2} @1.5k={:+.2} @6k={:+.2} | avg={:+.2}",
            gains[0], gains[1], gains[2], gains[3], avg,
        );
        for (i, freq) in bands.iter().enumerate() {
            print!("           tilt vs avg @{freq:>5.0}={:+.2} ", gains[i] - avg);
        }
        println!();
    }
}

#[test]
fn preset_signatures_match_calibration_tuples() {
    let samples_per_channel = (SR_HZ as f32 * DURATION_SEC) as usize;
    let input = synth_noise_stereo(samples_per_channel);
    let input_left = left_channel(&input);

    // Reference frequencies for the four band-tuning slots.
    let f_low = 200.0;
    let f_low_mid = 400.0;
    let f_presence = 1_500.0;
    let f_air = 6_000.0;

    // Per-preset between-band tilt assertions. Each entry is
    // (preset, label, high_hz, low_hz, predicate, expectation).
    // Thresholds are conservative bounds (~50% of observed values
    // measured against current master) so the test catches a clear
    // wiring regression without firing on small tuning drift. The
    // direction of each tilt mirrors the preset's calibration tuple
    // (see `compute_band_calibration` in dsp.rs and the Codex source
    // table at `mastering.py:96-357`).
    let cases: &[(&str, Preset, &[(&str, f32, f32, fn(f32) -> bool, &str)])] = &[
        // Universal: air shelf is the only non-zero EQ; the multiband
        // compressor flattens most of it on broadband noise. Observed
        // 6k - 1.5k tilt is +0.45 dB; require ≥ +0.2 dB.
        (
            "Universal",
            Preset::Universal,
            &[
                ("air vs presence", f_air, f_presence, |d| d >= 0.2, ">= +0.2 dB"),
            ],
        ),
        // Clarity: presence + air boosts, low-mid cut.
        (
            "Clarity",
            Preset::Clarity,
            &[
                ("air vs mud", f_air, f_low_mid, |d| d >= 0.8, ">= +0.8 dB"),
                ("presence vs mud", f_presence, f_low_mid, |d| d >= 0.5, ">= +0.5 dB"),
            ],
        ),
        // Tape: low boost, presence cut. Observed 200 - 1.5k = +1.56 dB,
        // 400 - 1.5k = +1.47 dB.
        (
            "Tape",
            Preset::Tape,
            &[
                ("low vs presence", f_low, f_presence, |d| d >= 0.8, ">= +0.8 dB"),
                ("low-mid vs presence", f_low_mid, f_presence, |d| d >= 0.8, ">= +0.8 dB"),
            ],
        ),
        // Spatial: air boost, low-mid cut. Observed 6k - 1.5k = +0.80 dB.
        (
            "Spatial",
            Preset::Spatial,
            &[
                ("air vs presence", f_air, f_presence, |d| d >= 0.4, ">= +0.4 dB"),
            ],
        ),
        // Oomph: low-mid cut, presence boost. Observed 1.5k - 400 = +1.46 dB.
        (
            "Oomph",
            Preset::Oomph,
            &[
                ("presence vs mud", f_presence, f_low_mid, |d| d >= 0.8, ">= +0.8 dB"),
            ],
        ),
        // Warmth: low boost, presence cut. Observed 200 - 1.5k = +1.91 dB.
        (
            "Warmth",
            Preset::Warmth,
            &[
                ("low vs presence", f_low, f_presence, |d| d >= 1.0, ">= +1.0 dB"),
            ],
        ),
        // Punch: deepest mud cut + presence boost. Observed 1.5k - 400 = +2.59 dB.
        (
            "Punch",
            Preset::Punch,
            &[
                ("presence vs mud", f_presence, f_low_mid, |d| d >= 1.5, ">= +1.5 dB"),
            ],
        ),
        // Loud: mud cut + presence + air boosts. Observed 1.5k - 400 = +2.16,
        // 6k - 400 = +1.39.
        (
            "Loud",
            Preset::Loud,
            &[
                ("presence vs mud", f_presence, f_low_mid, |d| d >= 1.0, ">= +1.0 dB"),
                ("air vs mud", f_air, f_low_mid, |d| d >= 0.5, ">= +0.5 dB"),
            ],
        ),
    ];

    for (preset_name, preset, checks) in cases {
        let output = run_through_chain(&input, preset.clone());
        let output_left = left_channel(&output);
        for (label, high_hz, low_hz, predicate, expectation) in *checks {
            let tilt = tilt_db(&input_left, &output_left, *high_hz, *low_hz);
            assert!(
                predicate(tilt),
                "{preset_name}: {label} tilt = {tilt:.2} dB, expected {expectation}",
            );
        }
    }
}

#[test]
fn preset_tape_introduces_third_harmonic_saturation() {
    // Codex Tape preset table has `warmth=0.095` (non-zero saturation
    // drive). A pure 1 kHz sine should pick up measurable 3 kHz energy.
    let samples_per_channel = (SR_HZ as f32 * DURATION_SEC) as usize;
    let input = synth_sine_stereo(samples_per_channel, 1_000.0);

    let input_left = left_channel(&input);
    let input_third = goertzel_mag_db(&input_left, SR_HZ as f32, 3_000.0);

    let output = run_through_chain(&input, Preset::Tape);
    let output_left = left_channel(&output);
    let output_third = goertzel_mag_db(&output_left, SR_HZ as f32, 3_000.0);

    assert!(
        output_third > input_third + 15.0,
        "Tape: 3rd-harmonic should rise >=+15 dB above the input's numerical floor ({input_third:.2} dB); got {output_third:.2} dB",
    );
}

#[test]
fn preset_warmth_introduces_third_harmonic_saturation() {
    // Codex Warmth preset table has `warmth=0.075` — slightly less drive
    // than Tape, same shape of effect.
    let samples_per_channel = (SR_HZ as f32 * DURATION_SEC) as usize;
    let input = synth_sine_stereo(samples_per_channel, 1_000.0);

    let input_left = left_channel(&input);
    let input_third = goertzel_mag_db(&input_left, SR_HZ as f32, 3_000.0);

    let output = run_through_chain(&input, Preset::Warmth);
    let output_left = left_channel(&output);
    let output_third = goertzel_mag_db(&output_left, SR_HZ as f32, 3_000.0);

    assert!(
        output_third > input_third + 10.0,
        "Warmth: 3rd-harmonic should rise >=+10 dB above input floor ({input_third:.2} dB); got {output_third:.2} dB",
    );
}

#[test]
fn preset_spatial_widener_increases_side_signal_rms() {
    // Spatial's `width=1.13` widens the side relative to the mid.
    // Antiphase stereo (L=+x, R=-x) is pure-side, so the output's side
    // RMS should grow.
    let samples_per_channel = (SR_HZ as f32 * DURATION_SEC) as usize;
    let input = synth_antiphase_stereo(samples_per_channel);
    let input_side = side_rms(&input);

    let output = run_through_chain(&input, Preset::Spatial);
    let output_side = side_rms(&output);

    let ratio = output_side / input_side.max(f32::MIN_POSITIVE);
    assert!(
        ratio > 1.1,
        "Spatial: post/pre side-RMS ratio should exceed 1.1; got {ratio:.3} (in={input_side:.5}, out={output_side:.5})",
    );
}

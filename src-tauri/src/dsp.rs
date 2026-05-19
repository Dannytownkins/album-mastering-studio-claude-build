use crate::types::*;
use std::f32::consts::PI;

// ============================================================================
// Biquad — direct form II transposed
// Coefficients computed per RBJ Audio EQ Cookbook
// ============================================================================

#[derive(Debug, Clone, Copy)]
pub struct BiquadCoeffs {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl BiquadCoeffs {
    pub fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }

    pub fn low_shelf(sample_rate: f32, freq_hz: f32, gain_db: f32, slope: f32) -> Self {
        if gain_db.abs() < 1.0e-4 {
            return Self::identity();
        }
        let a = 10.0_f32.powf(gain_db / 40.0);
        let omega = 2.0 * PI * freq_hz / sample_rate;
        let cos_o = omega.cos();
        let sin_o = omega.sin();
        let alpha = sin_o / 2.0 * ((a + 1.0 / a) * (1.0 / slope - 1.0) + 2.0).sqrt();
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_o + two_sqrt_a_alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_o);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_o - two_sqrt_a_alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos_o + two_sqrt_a_alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_o);
        let a2 = (a + 1.0) + (a - 1.0) * cos_o - two_sqrt_a_alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    pub fn high_shelf(sample_rate: f32, freq_hz: f32, gain_db: f32, slope: f32) -> Self {
        if gain_db.abs() < 1.0e-4 {
            return Self::identity();
        }
        let a = 10.0_f32.powf(gain_db / 40.0);
        let omega = 2.0 * PI * freq_hz / sample_rate;
        let cos_o = omega.cos();
        let sin_o = omega.sin();
        let alpha = sin_o / 2.0 * ((a + 1.0 / a) * (1.0 / slope - 1.0) + 2.0).sqrt();
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_o + two_sqrt_a_alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_o);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_o - two_sqrt_a_alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_o + two_sqrt_a_alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_o);
        let a2 = (a + 1.0) - (a - 1.0) * cos_o - two_sqrt_a_alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// Butterworth low-pass (RBJ cookbook, Q=0.7071 for one stage). For an
    /// LR4 crossover (-24 dB/oct), cascade two of these at the same corner.
    pub fn butter_lp(sample_rate: f32, freq_hz: f32, q: f32) -> Self {
        let omega = 2.0 * PI * freq_hz / sample_rate;
        let cos_o = omega.cos();
        let sin_o = omega.sin();
        let alpha = sin_o / (2.0 * q);
        let b0 = (1.0 - cos_o) / 2.0;
        let b1 = 1.0 - cos_o;
        let b2 = (1.0 - cos_o) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_o;
        let a2 = 1.0 - alpha;
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// Butterworth high-pass (RBJ cookbook, Q=0.7071 for one stage). Cascade
    /// two of these for an LR4 -24 dB/oct slope.
    pub fn butter_hp(sample_rate: f32, freq_hz: f32, q: f32) -> Self {
        let omega = 2.0 * PI * freq_hz / sample_rate;
        let cos_o = omega.cos();
        let sin_o = omega.sin();
        let alpha = sin_o / (2.0 * q);
        let b0 = (1.0 + cos_o) / 2.0;
        let b1 = -(1.0 + cos_o);
        let b2 = (1.0 + cos_o) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_o;
        let a2 = 1.0 - alpha;
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// ITU-R BS.1770-4 K-weighting pre-filter, Stage 1.
    ///
    /// High-shelf with analog parameters f0 = 1681.9744509555319 Hz,
    /// G = 3.999843853973347 dB, Q = 0.7071752369554196. These are NOT
    /// freely chosen — they are the constants specified by the standard,
    /// and at 48 kHz they produce the published reference coefficients
    /// (BS.1770-4 Annex 1) which downstream loudness meters compare against
    /// for conformance.
    ///
    /// Computed in f64 then narrowed to f32 so the result matches the
    /// published reference at the standard sample rates to within f32
    /// epsilon (~1e-7).
    pub fn k_weighting_pre(sample_rate: u32) -> Self {
        let fs = sample_rate as f64;
        let f0 = 1681.9744509555319_f64;
        let g = 3.999843853973347_f64;
        let q = 0.7071752369554196_f64;
        let k = (std::f64::consts::PI * f0 / fs).tan();
        let vh = 10.0_f64.powf(g / 20.0);
        let vb = vh.powf(0.4996667741545416);
        let denom = 1.0 + k / q + k * k;
        let b0 = (vh + vb * k / q + k * k) / denom;
        let b1 = 2.0 * (k * k - vh) / denom;
        let b2 = (vh - vb * k / q + k * k) / denom;
        let a1 = 2.0 * (k * k - 1.0) / denom;
        let a2 = (1.0 - k / q + k * k) / denom;
        Self {
            b0: b0 as f32,
            b1: b1 as f32,
            b2: b2 as f32,
            a1: a1 as f32,
            a2: a2 as f32,
        }
    }

    /// ITU-R BS.1770-4 K-weighting pre-filter, Stage 2 — the Revised
    /// Low-frequency B-curve (RLB) high-pass.
    ///
    /// Analog parameters f0 = 38.13547087602444 Hz, Q = 0.5003270373253953.
    /// The Q is NOT 0.7071 (the common mistake when implementing this
    /// filter); the BS.1770-specific Q gives the published response.
    ///
    /// Per the standard the b-coefficients are kept as the analog prototype
    /// (1, -2, 1) without being scaled by 1/a0 like the a-coefficients are.
    /// This introduces a ~+0.04 dB asymptotic gain at Nyquist relative to
    /// a naïvely-normalized RBJ HP, and is baked into the BS.1770 LUFS
    /// calibration offset of -0.691 dB. Reproducing the standard requires
    /// preserving this asymmetry.
    pub fn k_weighting_rlb(sample_rate: u32) -> Self {
        let fs = sample_rate as f64;
        let f0 = 38.13547087602444_f64;
        let q = 0.5003270373253953_f64;
        let k = (std::f64::consts::PI * f0 / fs).tan();
        let denom = 1.0 + k / q + k * k;
        // a coefficients are normalized (a0 implicit = 1); b coefficients
        // intentionally aren't — see doc comment above.
        let a1 = 2.0 * (k * k - 1.0) / denom;
        let a2 = (1.0 - k / q + k * k) / denom;
        Self {
            b0: 1.0,
            b1: -2.0,
            b2: 1.0,
            a1: a1 as f32,
            a2: a2 as f32,
        }
    }

    pub fn peaking(sample_rate: f32, freq_hz: f32, q: f32, gain_db: f32) -> Self {
        if gain_db.abs() < 1.0e-4 {
            return Self::identity();
        }
        let a = 10.0_f32.powf(gain_db / 40.0);
        let omega = 2.0 * PI * freq_hz / sample_rate;
        let cos_o = omega.cos();
        let alpha = omega.sin() / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_o;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_o;
        let a2 = 1.0 - alpha / a;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BiquadState {
    z1: f32,
    z2: f32,
}

impl BiquadState {
    pub fn process(&mut self, c: &BiquadCoeffs, x: f32) -> f32 {
        let y = c.b0 * x + self.z1;
        self.z1 = c.b1 * x - c.a1 * y + self.z2;
        self.z2 = c.b2 * x - c.a2 * y;
        y
    }
}

// ============================================================================
// MasteringChain — gain → 3-band EQ → optional saturation → soft-clip ceiling
// Phase 4.1 only: no real compressor, no lookahead limiter, no true-peak.
// Those land in Phase 11 (DSP audit).
// ============================================================================

// ============================================================================
// Phase A2: per-preset calibration ported from Codex.
//
// Each preset carries the 13 listening-tested numbers from
// `../album-mastering-studio/src/album_mastering_studio/mastering.py`. The
// numerical values were dialed during ~36 hours of test runs on the Codex
// side; we adopt them wholesale. Mapping was chosen by character match
// (our preset names are character-oriented; Codex's are genre-oriented):
//
//   Universal  -> streaming
//   Clarity    -> bright-air
//   Tape       -> warm-glue
//   Spatial    -> album-cohesion-cinematic
//   Oomph      -> heavy-rock-metal
//   Warmth     -> dark-smooth
//   Punch      -> djent-modern-metal
//   Loud       -> loud-aggressive
//   Custom     -> neutral (no Codex source)
//
// User-facing EQ knobs (`eq_low_db`, `eq_low_mid_db`, `eq_mid_db`,
// `eq_high_db`) add ON TOP of these preset baselines.
//
// Phase A4 (2026-05-14) — preset compressor identity is now APPLIED. Each
// preset's `compressor_threshold_dbfs` / `compressor_ratio` /
// `compressor_attack_ms` / `compressor_release_ms` shapes the multiband
// compressor by default; the user's `compression_density` macro scales
// the preset's baseline rather than replacing it (density 0 = bypass,
// density 0.5 = full preset character, density 1 = preset pushed
// further). The `Custom` preset stays at the Universal compressor
// identity but defaults to density 0 so a fresh-Custom session is still
// an identity chain.
//
// Still captured but not directly applied:
//   * target_lufs — preset loudness intent only. DeliveryProfile owns
//     actual LUFS landing through `effective_target_lufs()`, so this
//     must not grow a second measure-and-target path.
// ============================================================================

#[derive(Debug, Clone, Copy)]
pub struct PresetCalibration {
    /// 200 Hz low-shelf baseline gain in dB. Adds to `eq_low_db`.
    pub low_shelf_db: f32,
    /// 400 Hz peaking baseline gain in dB. NEW band in Phase A2. Heavy
    /// presets carry significant CUTS here (the "mud zone" between 250
    /// and 800 Hz that muddies dense arrangements). Adds to `eq_low_mid_db`.
    pub low_mid_db: f32,
    /// 1.5 kHz peaking baseline gain in dB (Codex `presence_db`). Adds
    /// to `eq_mid_db`.
    pub presence_db: f32,
    /// 6 kHz high-shelf baseline gain in dB (Codex `air_db`). Adds to
    /// `eq_high_db`.
    pub air_db: f32,
    /// Saturation drive amount (Codex `warmth`, 0..1 unitless). Drives
    /// the post-EQ tanh stage.
    pub warmth: f32,
    /// Baseline M/S widener default (Codex `stereo_width`). 1.0 = neutral,
    /// > 1 widens, < 1 narrows. The user's `advanced.width` slider takes
    /// precedence when set; this is what the preset uses out of the box.
    pub stereo_width: f32,
    /// Preset transient shaper intent. Positive values lift attacks;
    /// negative values soften them.
    pub transient_punch: f32,
    /// Per-preset subsonic high-pass cutoff in Hz. Applied as an LR4
    /// cascade before the musical EQ stages; 0.0 means identity.
    pub highpass_hz: f32,
    /// Captured target integrated LUFS. Preset intent only; actual
    /// render landing is owned by DeliveryProfile/effective_target_lufs().
    pub target_lufs: f32,
    /// Captured recommended true-peak ceiling. Not applied in A2 (would
    /// change the limiter's behavior on existing tests). Phase A3 wires
    /// this through the delivery-profile shadow.
    pub ceiling_dbfs: f32,
    /// Per-preset uniform multiband compressor threshold (dBFS), applied
    /// by default at `compression_density = 0.5`. The user's macro scales
    /// engagement (0 = bypass, 0.5 = full preset, 1.0 = preset + ~3 dB
    /// deeper drive).
    pub compressor_threshold_dbfs: f32,
    /// Per-preset uniform multiband compressor ratio, applied alongside
    /// `compressor_threshold_dbfs`. Scaled by the same engagement curve.
    pub compressor_ratio: f32,
    /// Compressor attack time constant (ms). Faster = grabs transients
    /// harder; slower = lets them through. Applied uniformly across the
    /// three bands so the preset has a single rhythmic character.
    pub compressor_attack_ms: f32,
    /// Compressor release time constant (ms). Slower = glue / density;
    /// faster = preserves movement between transients.
    pub compressor_release_ms: f32,
    /// Codex science note — terse rationale for the calibration.
    pub science_note: &'static str,
    /// Static input-gain push in dB. Codex doesn't have a direct
    /// equivalent (their reference chain carries preset loudness intent).
    /// DeliveryProfile owns LUFS landing; this remains a coarse preset
    /// gain push, not a substitute target loop.
    pub baseline_gain_push_db: f32,
}

const PRESET_UNIVERSAL: PresetCalibration = PresetCalibration {
    // Conservative target (PRESET_REFERENCE_ANALYSIS_2026-05-14, line 252).
    // Cross-genre safe default. Light transparent compressor.
    low_shelf_db: 0.2,
    low_mid_db: -0.1,
    presence_db: 0.0,
    air_db: 1.1,
    warmth: 0.03,
    stereo_width: 1.04,
    transient_punch: 0.04,
    highpass_hz: 24.0,
    target_lufs: -14.0,
    ceiling_dbfs: -1.0,
    compressor_threshold_dbfs: -16.0,
    compressor_ratio: 1.8,
    compressor_attack_ms: 15.0,
    compressor_release_ms: 250.0,
    science_note:
        "Light transparent program compression, gentle air lift, neutral mids.",
    baseline_gain_push_db: 1.2,
};

const PRESET_CLARITY: PresetCalibration = PresetCalibration {
    // Conservative target line 253. Vocal / detail / definition.
    // Drops the 1.5 kHz region, lifts air, fast release for articulation.
    low_shelf_db: 0.2,
    low_mid_db: -1.0,
    presence_db: -0.8,
    air_db: 1.7,
    warmth: 0.025,
    stereo_width: 1.02,
    transient_punch: 0.05,
    highpass_hz: 28.0,
    target_lufs: -13.4,
    ceiling_dbfs: -1.0,
    compressor_threshold_dbfs: -16.0,
    compressor_ratio: 1.8,
    compressor_attack_ms: 12.0,
    compressor_release_ms: 150.0,
    science_note:
        "Lower mids tucked, air shelf forward; fast release keeps articulation \
         open between transients.",
    baseline_gain_push_db: 0.8,
};

const PRESET_TAPE: PresetCalibration = PresetCalibration {
    // Conservative target line 254. Glue, softened top, fuller low body.
    // Heavier saturation + slower compressor for crest reduction.
    low_shelf_db: -0.2,
    low_mid_db: 0.3,
    presence_db: -1.4,
    air_db: 2.0,
    warmth: 0.10,
    stereo_width: 0.99,
    transient_punch: -0.03,
    highpass_hz: 24.0,
    target_lufs: -13.8,
    ceiling_dbfs: -1.1,
    compressor_threshold_dbfs: -22.0,
    compressor_ratio: 2.4,
    compressor_attack_ms: 30.0,
    compressor_release_ms: 400.0,
    science_note:
        "Deeper saturation and slow program compression deliver glue and a \
         visibly reduced crest factor.",
    baseline_gain_push_db: 1.5,
};

const PRESET_SPATIAL: PresetCalibration = PresetCalibration {
    // Conservative target line 255. Wide, dimensional, transparent compression.
    low_shelf_db: 0.1,
    low_mid_db: -0.8,
    presence_db: -0.3,
    air_db: 1.3,
    warmth: 0.04,
    stereo_width: 1.16,
    transient_punch: 0.03,
    highpass_hz: 24.0,
    target_lufs: -13.1,
    ceiling_dbfs: -1.0,
    compressor_threshold_dbfs: -16.0,
    compressor_ratio: 1.8,
    compressor_attack_ms: 15.0,
    compressor_release_ms: 250.0,
    science_note:
        "Widest side image with a clean low-mid floor and transparent program \
         compression — width without pumping.",
    baseline_gain_push_db: 1.0,
};

const PRESET_OOMPH: PresetCalibration = PresetCalibration {
    // Conservative target line 256. Sub/low lift + low-mid scoop.
    // Medium compressor controls the lows without flattening.
    low_shelf_db: 2.4,
    low_mid_db: -3.0,
    presence_db: -2.6,
    air_db: -0.8,
    warmth: 0.045,
    stereo_width: 0.95,
    transient_punch: 0.08,
    highpass_hz: 22.0,
    target_lufs: -12.0,
    ceiling_dbfs: -0.9,
    compressor_threshold_dbfs: -22.0,
    compressor_ratio: 2.6,
    compressor_attack_ms: 25.0,
    compressor_release_ms: 280.0,
    science_note:
        "Strong sub lift with deep low-mid scoop; medium-rate compressor \
         keeps the weight controlled rather than muddy.",
    baseline_gain_push_db: 1.8,
};

const PRESET_WARMTH: PresetCalibration = PresetCalibration {
    // Conservative target line 257. Fuller body, softer top, soft glue.
    low_shelf_db: 0.8,
    low_mid_db: 0.7,
    presence_db: -1.8,
    air_db: -0.8,
    warmth: 0.08,
    stereo_width: 0.98,
    transient_punch: -0.05,
    highpass_hz: 24.0,
    target_lufs: -14.7,
    ceiling_dbfs: -1.2,
    compressor_threshold_dbfs: -19.0,
    compressor_ratio: 2.0,
    compressor_attack_ms: 20.0,
    compressor_release_ms: 280.0,
    science_note:
        "Softened presence/air with gentle saturation and soft glue \
         compression — smooth body, no edge.",
    baseline_gain_push_db: 1.0,
};

const PRESET_PUNCH: PresetCalibration = PresetCalibration {
    // Conservative target line 258. Faster attack/release, transient-forward.
    // Higher threshold + faster release = preserves more crest than Loud.
    low_shelf_db: 0.8,
    low_mid_db: -1.8,
    presence_db: 1.6,
    air_db: 0.8,
    warmth: 0.035,
    stereo_width: 1.04,
    transient_punch: 0.14,
    highpass_hz: 28.0,
    target_lufs: -10.9,
    ceiling_dbfs: -0.8,
    compressor_threshold_dbfs: -20.0,
    compressor_ratio: 2.8,
    compressor_attack_ms: 10.0,
    compressor_release_ms: 100.0,
    science_note:
        "Tight lows, presence bite, fast attack/release keep impact and \
         forwardness without dragging into Loud territory.",
    baseline_gain_push_db: 1.6,
};

const PRESET_LOUD: PresetCalibration = PresetCalibration {
    // Conservative target line 259. Strongest density/limiting.
    // Lowest threshold, highest ratio, controlled time constants for density.
    low_shelf_db: 0.4,
    low_mid_db: -1.6,
    presence_db: 1.8,
    air_db: 1.2,
    warmth: 0.055,
    stereo_width: 1.03,
    transient_punch: 0.12,
    highpass_hz: 30.0,
    target_lufs: -10.4,
    ceiling_dbfs: -0.8,
    compressor_threshold_dbfs: -23.0,
    compressor_ratio: 3.5,
    compressor_attack_ms: 15.0,
    compressor_release_ms: 180.0,
    science_note:
        "Strongest density and limiting; assertive but not smashed — \
         enough movement remains to read as a master, not a preview.",
    baseline_gain_push_db: 2.5,
};

const PRESET_CUSTOM_NEUTRAL: PresetCalibration = PresetCalibration {
    // No source. Neutral starting slate — user drives every parameter.
    // Compressor mirrors Universal's identity so the macro still does
    // something useful when the user dials it up, but default density
    // for Custom is 0 (see `from_settings`) so a fresh-Custom session
    // is an identity chain.
    low_shelf_db: 0.0,
    low_mid_db: 0.0,
    presence_db: 0.0,
    air_db: 0.0,
    warmth: 0.0,
    stereo_width: 1.0,
    transient_punch: 0.0,
    highpass_hz: 0.0,
    target_lufs: -14.0,
    ceiling_dbfs: -1.0,
    compressor_threshold_dbfs: -16.0,
    compressor_ratio: 1.8,
    compressor_attack_ms: 15.0,
    compressor_release_ms: 200.0,
    science_note: "Neutral starting slate — user drives the chain.",
    baseline_gain_push_db: 1.5,
};

pub fn preset_calibration(preset: &Preset) -> PresetCalibration {
    match preset {
        Preset::Universal => PRESET_UNIVERSAL,
        Preset::Clarity => PRESET_CLARITY,
        Preset::Tape => PRESET_TAPE,
        Preset::Spatial => PRESET_SPATIAL,
        Preset::Oomph => PRESET_OOMPH,
        Preset::Warmth => PRESET_WARMTH,
        Preset::Punch => PRESET_PUNCH,
        Preset::Loud => PRESET_LOUD,
        Preset::Custom { .. } => PRESET_CUSTOM_NEUTRAL,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChainCoeffs {
    /// Per-preset LR4 subsonic high-pass. One Butterworth stage is stored
    /// here and run through two state stages for a 24 dB/oct slope.
    pub sub_highpass: BiquadCoeffs,
    pub low: BiquadCoeffs,
    /// Phase A2: low-mid peaking @ 400 Hz, Q=0.9. Heavy presets cut this
    /// band to clean up the mud zone (250–800 Hz). Identity biquad when
    /// preset baseline + user offset = 0 dB, so the chain stays
    /// byte-equivalent to the pre-A2 output for any neutral configuration.
    pub low_mid: BiquadCoeffs,
    pub mid: BiquadCoeffs,
    pub high: BiquadCoeffs,
    /// Phase 12.2 — surgical low-mid warmth shelf, additive on top of the
    /// preset and the main Low band. Low-shelf @ 300 Hz, slope 0.7. Slider
    /// 0..1 in `AdvancedSettings::warmth` maps to 0..+4 dB; clamped on read.
    pub warmth: BiquadCoeffs,
    /// Phase 12.2 — surgical air shelf, additive on top of the preset and
    /// the main High band. High-shelf @ 10 kHz, slope 0.7. Slider 0..1 in
    /// `AdvancedSettings::presence_air` maps to 0..+4 dB; clamped on read.
    pub presence_air: BiquadCoeffs,
    /// Phase A5 — preset transient shaper amount. Positive values lift
    /// attacks; negative values soften them. Custom defaults to 0.0.
    pub transient_amount: f32,
    pub transient_fast_attack_alpha: f32,
    pub transient_fast_release_alpha: f32,
    pub transient_slow_attack_alpha: f32,
    pub transient_slow_release_alpha: f32,
    pub input_gain_lin: f32,
    pub saturation_amount: f32,
    pub ceiling_lin: f32,
    /// Phase 12.1 — user-controllable output trim, applied AFTER the limiter
    /// and the volume-match scalar. 1.0 = no change. Boosting above 1.0 can
    /// reintroduce peaks above the ceiling (intentional — user choice; the
    /// export check will flag it).
    pub user_output_gain_lin: f32,
    /// Post-chain output gain used to align mastered playback loudness with
    /// the unprocessed source. 1.0 = no adjustment (Volume Match off).
    /// When on, set to the inverse of the input gain stage so the master
    /// comes back down to roughly the source's level for fair A/B. Approximate
    /// — doesn't account for EQ/saturation contributions, but close enough
    /// for tone judgment. Tooltip in the UI is honest about this.
    pub volume_match_gain_lin: f32,
    /// Live-preview export landing gain. Offline export applies a final,
    /// down-only LUFS trim after rendering; Mastered playback receives the
    /// same gain from the audio thread so audition level matches export level.
    /// Defaults to 1.0 in render/export paths.
    pub export_landing_gain_lin: f32,
    /// Phase 12.2 — stereo width via M/S processing. Scales the side
    /// component between EQ and saturation; 0 = mono (collapse side to zero),
    /// 1 = neutral (no-op), > 1 widens. Only consulted when the chain is
    /// running on a stereo frame. Clamped to [0.0, 2.0] in `from_settings`
    /// so an out-of-range user value can't flip phase or destabilize gain.
    pub width_side_scale: f32,
    // ----- Phase 12.2: multiband compressor coefficients -----
    /// Whether the compressor is active. `false` triggers the identity early-
    /// return in `process_frame_inplace` — byte-equivalent to the pre-slice
    /// chain output. `true` when ANY of: macro density > 1e-4, any per-band
    /// override is `Some(_)`, or link_stereo is `Some(false)`.
    pub compression_active: bool,
    pub comp_low_lp: BiquadCoeffs,
    pub comp_mid_hp: BiquadCoeffs,
    pub comp_mid_lp: BiquadCoeffs,
    pub comp_high_hp: BiquadCoeffs,
    pub comp_low_threshold_db: f32,
    pub comp_low_ratio: f32,
    pub comp_low_attack_alpha: f32,
    pub comp_low_release_alpha: f32,
    pub comp_low_makeup_db: f32,
    pub comp_low_makeup_lin: f32,
    pub comp_mid_threshold_db: f32,
    pub comp_mid_ratio: f32,
    pub comp_mid_attack_alpha: f32,
    pub comp_mid_release_alpha: f32,
    pub comp_mid_makeup_db: f32,
    pub comp_mid_makeup_lin: f32,
    pub comp_high_threshold_db: f32,
    pub comp_high_ratio: f32,
    pub comp_high_attack_alpha: f32,
    pub comp_high_release_alpha: f32,
    pub comp_high_makeup_db: f32,
    pub comp_high_makeup_lin: f32,
    /// Soft-knee width in dB (fixed at 6 dB per the design — not user-tunable
    /// in v1). Stored on the coeffs so the gain-stage code reads one source
    /// of truth.
    pub comp_knee_db: f32,
    /// Linked-stereo behavior. `true` = max(|L|,|R|) drives a shared
    /// envelope; `false` = independent per-channel envelopes per band.
    pub comp_link_stereo: bool,
}

impl ChainCoeffs {
    pub fn from_settings(sample_rate: u32, settings: &MasteringSettings) -> Self {
        let sr = sample_rate as f32;
        let intensity = settings.intensity.clamp(0.0, 1.0);

        // Per-PRODUCT.md, Intensity is a macro that "should change how hard the
        // preset works across multiple parameters" — not a volume knob. So each
        // preset gets a baseline EQ curve, saturation amount, and gain push,
        // and Intensity scales the whole preset character. At intensity = 0.5
        // (the default), the preset is at full character; below 0.5 it
        // softens toward neutral, above 0.5 it pushes harder.
        //
        //   preset_scale(intensity) = 0.4 + 1.2 * intensity
        //     intensity 0.0  -> 0.40 (preset audible but subtle)
        //     intensity 0.5  -> 1.00 (full preset character — the default)
        //     intensity 1.0  -> 1.60 (preset pushed past full)
        //
        // The user's manual Low/Mid/High EQ adds ON TOP of the preset
        // baseline, so dialing in a custom tweak doesn't erase the preset's
        // signature sound. Numbers below were tuned conservatively for a
        // first pass — Phase 12.1 listening on real material will calibrate.
        let preset_scale = 0.4 + 1.2 * intensity;

        // Phase A2: per-preset baselines come from the PresetCalibration table
        // (ported from Codex's 36-hour listening calibration). EQ map:
        //   preset.low_shelf_db  → 200 Hz low-shelf
        //   preset.low_mid_db    → 400 Hz peaking  (NEW band in A2)
        //   preset.presence_db   → 1.5 kHz peaking (our "mid")
        //   preset.air_db        → 6 kHz high-shelf (our "high")
        // The user's eq_low_db / eq_low_mid_db / eq_mid_db / eq_high_db
        // sliders add ON TOP of the scaled preset values.
        let preset = preset_calibration(&settings.preset);

        let effective_low_db = preset.low_shelf_db * preset_scale + settings.eq_low_db;
        let effective_low_mid_db =
            preset.low_mid_db * preset_scale + settings.eq_low_mid_db;
        let effective_mid_db = preset.presence_db * preset_scale + settings.eq_mid_db;
        let effective_high_db = preset.air_db * preset_scale + settings.eq_high_db;

        // Intentionally a cascaded 2-biquad Butterworth (4-pole / 24 dB per
        // octave), even though standalone HPF does not need LR4 sum-flat
        // crossover behavior. The goal here is mastering-grade subsonic
        // cleanup: firm rumble removal below the cutoff while leaving the
        // audible low band essentially intact.
        let sub_highpass = if preset.highpass_hz > 0.0 {
            BiquadCoeffs::butter_hp(sr, preset.highpass_hz.clamp(20.0, 40.0), BUTTERWORTH_Q)
        } else {
            BiquadCoeffs::identity()
        };
        let low = BiquadCoeffs::low_shelf(sr, 200.0, effective_low_db, 0.7);
        let low_mid = BiquadCoeffs::peaking(sr, 400.0, 0.9, effective_low_mid_db);
        let mid = BiquadCoeffs::peaking(sr, 1500.0, 0.8, effective_mid_db);
        let high = BiquadCoeffs::high_shelf(sr, 6000.0, effective_high_db, 0.7);

        // Compatibility shims for the rest of from_settings, which expects
        // legacy names. preset_gain_db / preset_sat / preset_width map to
        // PresetCalibration fields directly.
        let preset_gain_db = preset.baseline_gain_push_db;
        let preset_sat = preset.warmth;
        let preset_width = preset.stereo_width;

        // Phase 12.2 — Advanced warmth (low-shelf @ 300 Hz). Slider value clamped
        // into [0, 1] then scaled to a 0..+4 dB lift. When the slider is None or
        // zero, `BiquadCoeffs::low_shelf` returns identity via its built-in
        // early-return at `gain_db < 1e-4`.
        let warmth_db = settings
            .advanced
            .warmth
            .unwrap_or(0.0)
            .clamp(0.0, 1.0)
            * 4.0;
        let warmth = BiquadCoeffs::low_shelf(sr, 300.0, warmth_db, 0.7);

        // Phase 12.2 — Advanced presence/air (high-shelf @ 10 kHz). Same clamp +
        // scale pattern as warmth. Sits above the main High band (6 kHz) so the
        // two controls shape distinct perceptual regions.
        let presence_air_db = settings
            .advanced
            .presence_air
            .unwrap_or(0.0)
            .clamp(0.0, 1.0)
            * 4.0;
        let presence_air = BiquadCoeffs::high_shelf(sr, 10_000.0, presence_air_db, 0.7);
        let transient_amount = (preset.transient_punch * preset_scale).clamp(-0.25, 0.25);
        let transient_fast_attack_alpha = alpha_from_time_ms(sr, 1.0);
        let transient_fast_release_alpha = alpha_from_time_ms(sr, 35.0);
        let transient_slow_attack_alpha = alpha_from_time_ms(sr, 15.0);
        let transient_slow_release_alpha = alpha_from_time_ms(sr, 140.0);

        // Input gain = scaled preset gain push + user input gain. User input
        // gain is the standard mastering "back off the source" knob — useful
        // when an already-mastered track would otherwise clip after the
        // preset's baseline gain push lands on top of it.
        let user_input_gain_db = settings.input_gain_db.clamp(-24.0, 24.0);
        let input_gain_db = preset_gain_db * preset_scale + user_input_gain_db;
        let input_gain_lin = 10.0_f32.powf(input_gain_db / 20.0);

        let saturation_amount = preset_sat * preset_scale;

        // Phase A3 — effective ceiling: delivery profile shadows the user's
        // explicit advanced.ceiling_dbtp when the profile is non-Custom.
        let ceiling_db = settings.effective_ceiling_dbtp().clamp(-6.0, 0.0);
        let ceiling_lin = 10.0_f32.powf(ceiling_db / 20.0);

        // Post-limiter user-trim. Clamped to the same ±24 dB range as input
        // gain for symmetric extremes; default 0 dB.
        let user_output_gain_db = settings.output_gain_db.clamp(-24.0, 24.0);
        let user_output_gain_lin = 10.0_f32.powf(user_output_gain_db / 20.0);

        // Volume Match attenuation is computed AFTER the compressor block
        // below. We bind `volume_match_gain_lin` from a closure here so the
        // ChainCoeffs initializer at the bottom can use it without
        // forward-referencing.
        //
        // The previous formulation tried `attenuation = source_lufs -
        // preset.target_lufs`, but preset `target_lufs` is intent only;
        // DeliveryProfile owns actual LUFS landing. Measured
        // chain output sat up to ~4 dB above target on heavy presets
        // (Tape -9.5 vs target -13.8; Loud -7.3 vs target -10.4), which
        // left VM noticeably under-compensated and reading as "doesn't
        // really work" during listening passes.
        //
        // The new approach estimates the chain's loudness push above the
        // source from the actual deterministic gain stages — input gain,
        // average compressor makeup, a small saturation correction, and
        // the user's output trim — and attenuates by that estimate.
        // For normal settings the estimate is independent of source LUFS:
        // if estimated_push ≈ actual push, (mastered − estimated_push) ≈
        // source regardless of source level. Empirically lands within
        // ~1 dB of true source loudness across all eight presets (vs the
        // old `source_lufs - target_lufs` approach's 0.5–4.3 dB error).
        //
        // A LATER REFINEMENT (the VM cap) DOES consult
        // source_lufs_integrated — see the comment block immediately
        // before the volume_match_gain_lin computation below. The cap
        // bounds the estimate by what the limiter could plausibly let
        // through, which requires knowing the source LUFS. Without
        // source_lufs_integrated the cap can't fire and the raw
        // estimate is used unbounded. Dan reported the un-capped
        // estimate over-attenuating by 11 dB on aggressive settings
        // (Tape Intensity 100% + +13 dB input gain); the cap fixes
        // that. Most everyday settings stay below the cap threshold
        // and behave identically to the source-LUFS-independent
        // formulation.

        // Width: None means "neutral" (1.0 = leave the stereo image alone).
        // Clamp to [0, 2] so a stray slider value can't invert phase or push
        // the side past 2× — typical mastering plugins cap M/S widening here.
        let width_side_scale = settings
            .advanced
            .width
            .unwrap_or(preset_width)
            .clamp(0.0, 2.0);

        // ----- Phase A4: preset-driven multiband compressor -----
        //
        // The user's `compression_density` macro is preset-relative:
        //
        //   density 0.0  → bypass (effective threshold = 0 dBFS, ratio = 1)
        //   density 0.5  → preset baseline at full character (the default
        //                   for non-Custom presets when the user hasn't
        //                   touched the macro)
        //   density 1.0  → preset baseline pushed an extra ~3 dB harder
        //                   and +0.5 ratio (still bounded by the preset's
        //                   intent — not a slam-everything mode)
        //
        // Custom defaults to density 0.0 so a fresh Custom session is an
        // identity chain. All other presets default to density 0.5 so
        // their compressor identity audibly engages without any user
        // fiddling — that's the whole point of P1.
        //
        // Per-band overrides on `MasteringSettings.advanced.compression_*`
        // still take precedence and replace the per-preset value for that
        // band/parameter. The macro engagement only governs the FALLBACK
        // value used when an override is unset.
        let default_density_for_preset = if matches!(settings.preset, Preset::Custom { .. }) {
            0.0
        } else {
            0.5
        };
        let density = settings
            .advanced
            .compression_density
            .unwrap_or(default_density_for_preset)
            .clamp(0.0, 1.0);

        let preset_engagement = (density * 2.0).min(1.0); // 0..1, full at density >= 0.5
        let overdrive = (density * 2.0 - 1.0).max(0.0);   // 0 below 0.5, up to 1.0 at density=1
        const OVERDRIVE_THRESHOLD_DB: f32 = -3.0;
        const OVERDRIVE_RATIO: f32 = 0.5;

        let preset_threshold_db = preset.compressor_threshold_dbfs * preset_engagement
            + OVERDRIVE_THRESHOLD_DB * overdrive;
        let preset_ratio = (1.0 + (preset.compressor_ratio - 1.0) * preset_engagement
            + OVERDRIVE_RATIO * overdrive)
            .max(1.0);

        let comp_low_threshold_db = settings
            .advanced
            .compression_low_threshold_db
            .unwrap_or(preset_threshold_db);
        let comp_mid_threshold_db = settings
            .advanced
            .compression_mid_threshold_db
            .unwrap_or(preset_threshold_db);
        let comp_high_threshold_db = settings
            .advanced
            .compression_high_threshold_db
            .unwrap_or(preset_threshold_db);

        let comp_low_ratio = settings
            .advanced
            .compression_low_ratio
            .unwrap_or(preset_ratio)
            .max(1.0);
        let comp_mid_ratio = settings
            .advanced
            .compression_mid_ratio
            .unwrap_or(preset_ratio)
            .max(1.0);
        let comp_high_ratio = settings
            .advanced
            .compression_high_ratio
            .unwrap_or(preset_ratio)
            .max(1.0);

        let low_attack_ms = settings
            .advanced
            .compression_low_attack_ms
            .unwrap_or(preset.compressor_attack_ms)
            .max(0.1);
        let low_release_ms = settings
            .advanced
            .compression_low_release_ms
            .unwrap_or(preset.compressor_release_ms)
            .max(0.1);
        let mid_attack_ms = settings
            .advanced
            .compression_mid_attack_ms
            .unwrap_or(preset.compressor_attack_ms)
            .max(0.1);
        let mid_release_ms = settings
            .advanced
            .compression_mid_release_ms
            .unwrap_or(preset.compressor_release_ms)
            .max(0.1);
        let high_attack_ms = settings
            .advanced
            .compression_high_attack_ms
            .unwrap_or(preset.compressor_attack_ms)
            .max(0.1);
        let high_release_ms = settings
            .advanced
            .compression_high_release_ms
            .unwrap_or(preset.compressor_release_ms)
            .max(0.1);

        let comp_low_attack_alpha = alpha_from_time_ms(sr, low_attack_ms);
        let comp_low_release_alpha = alpha_from_time_ms(sr, low_release_ms);
        let comp_mid_attack_alpha = alpha_from_time_ms(sr, mid_attack_ms);
        let comp_mid_release_alpha = alpha_from_time_ms(sr, mid_release_ms);
        let comp_high_attack_alpha = alpha_from_time_ms(sr, high_attack_ms);
        let comp_high_release_alpha = alpha_from_time_ms(sr, high_release_ms);

        // Auto makeup: half-compensation of the threshold drop scaled by
        // (1 - 1/ratio). Splitting the compensation in half (the `/ 2.0`)
        // keeps the chain conservative — full compensation would push the
        // limiter harder on every density tweak.
        let makeup_db = |threshold_db: f32, ratio: f32| -> f32 {
            let threshold_drop_db = (-threshold_db).max(0.0);
            threshold_drop_db * (1.0 - 1.0 / ratio) / 2.0
        };
        let comp_low_makeup_db = makeup_db(comp_low_threshold_db, comp_low_ratio);
        let comp_mid_makeup_db = makeup_db(comp_mid_threshold_db, comp_mid_ratio);
        let comp_high_makeup_db = makeup_db(comp_high_threshold_db, comp_high_ratio);
        let comp_low_makeup_lin = 10.0_f32.powf(comp_low_makeup_db / 20.0);
        let comp_mid_makeup_lin = 10.0_f32.powf(comp_mid_makeup_db / 20.0);
        let comp_high_makeup_lin = 10.0_f32.powf(comp_high_makeup_db / 20.0);

        // Volume Match: estimate the chain's loudness push above the
        // source from deterministic gain stages, then attenuate by it.
        //
        //   raw_push_db ≈ input_gain
        //               + average compressor band makeup
        //               + small saturation correction (~5 × drive)
        //               + user output trim
        //
        // EQ tilt is signal-dependent (pink-noise-shaped material vs a
        // dense vocal-forward mix lands differently), so it's not in the
        // estimate. For normal settings the remaining error sits within
        // ~1 dB of true source loudness — comfortably under the
        // perceptual threshold for fair tone comparison.
        //
        // The raw estimate breaks down on extreme settings (e.g. +13 dB
        // input + +12 dB EQ + Tape Intensity 100%): the chain wants to
        // push ~20 dB, but the limiter caps actual chain LUFS at
        // roughly (ceiling - typical_crest), so real push is much less.
        // Pre-fix, VM would apply the over-estimated 20 dB attenuation
        // and drop playback 10+ dB below source (Dan: -24.9 LUFS vs
        // -13.4 source on aggressive Tape settings).
        //
        // Fix: cap the effective push at what the limiter could possibly
        // let through. (ceiling - typical_crest) is the LUFS ceiling of
        // the chain output; (chain_ceiling - source_lufs) is the maximum
        // real push. Uses 6 dB as a conservative crest-factor estimate
        // (real mastered material lands 6-12 dB peak-to-LUFS; 6 picks
        // the dense-rock end so the cap is wide enough not to fight
        // legitimate pushes).
        let volume_match_gain_lin = if settings.volume_match {
            let avg_makeup_db =
                (comp_low_makeup_db + comp_mid_makeup_db + comp_high_makeup_db) / 3.0;
            let saturation_correction_db = 5.0 * saturation_amount.max(0.0);
            let raw_push_db = input_gain_db
                + avg_makeup_db
                + saturation_correction_db
                + user_output_gain_db;
            const TYPICAL_CREST_DB: f32 = 6.0;
            let effective_push_db = if let Some(source_lufs) =
                settings.source_lufs_integrated
            {
                let ceiling_dbtp = settings.effective_ceiling_dbtp();
                let max_real_push_db =
                    (ceiling_dbtp - TYPICAL_CREST_DB - source_lufs).max(0.0);
                raw_push_db.min(max_real_push_db)
            } else {
                // No source LUFS available — fall back to raw estimate.
                // Hits when the playback driver hasn't injected the
                // current track's analysis yet (rare; the chain rebuilds
                // a moment later once analysis lands).
                raw_push_db
            };
            let attenuation_db = (-effective_push_db).clamp(-24.0, 0.0);
            10.0_f32.powf(attenuation_db / 20.0)
        } else {
            1.0
        };

        let comp_low_lp = BiquadCoeffs::butter_lp(sr, LR4_CROSSOVER_LOW_HZ, BUTTERWORTH_Q);
        let comp_mid_hp = BiquadCoeffs::butter_hp(sr, LR4_CROSSOVER_LOW_HZ, BUTTERWORTH_Q);
        let comp_mid_lp = BiquadCoeffs::butter_lp(sr, LR4_CROSSOVER_HIGH_HZ, BUTTERWORTH_Q);
        let comp_high_hp = BiquadCoeffs::butter_hp(sr, LR4_CROSSOVER_HIGH_HZ, BUTTERWORTH_Q);

        let comp_link_stereo = settings
            .advanced
            .compression_link_stereo
            .unwrap_or(true);

        // The chain skips the compressor stage entirely when no band is
        // doing anything: effective preset threshold is at the ceiling
        // (≈ 0 dBFS, i.e. density was scaled to bypass), no per-band
        // override is set, and link_stereo is at its default. This keeps
        // the "fresh Custom" identity property and the parity tests that
        // rely on it.
        let preset_compressor_inactive = preset_threshold_db > -1.0e-3;
        let comp_no_overrides = settings.advanced.compression_low_threshold_db.is_none()
            && settings.advanced.compression_low_ratio.is_none()
            && settings.advanced.compression_low_attack_ms.is_none()
            && settings.advanced.compression_low_release_ms.is_none()
            && settings.advanced.compression_mid_threshold_db.is_none()
            && settings.advanced.compression_mid_ratio.is_none()
            && settings.advanced.compression_mid_attack_ms.is_none()
            && settings.advanced.compression_mid_release_ms.is_none()
            && settings.advanced.compression_high_threshold_db.is_none()
            && settings.advanced.compression_high_ratio.is_none()
            && settings.advanced.compression_high_attack_ms.is_none()
            && settings.advanced.compression_high_release_ms.is_none();
        let comp_link_unset = !matches!(
            settings.advanced.compression_link_stereo,
            Some(false)
        );
        let compression_active = !(preset_compressor_inactive && comp_no_overrides && comp_link_unset);

        let comp_knee_db = 6.0_f32;

        Self {
            sub_highpass,
            low,
            low_mid,
            mid,
            high,
            warmth,
            presence_air,
            transient_amount,
            transient_fast_attack_alpha,
            transient_fast_release_alpha,
            transient_slow_attack_alpha,
            transient_slow_release_alpha,
            input_gain_lin,
            saturation_amount,
            ceiling_lin,
            user_output_gain_lin,
            volume_match_gain_lin,
            export_landing_gain_lin: 1.0,
            width_side_scale,
            compression_active,
            comp_low_lp,
            comp_mid_hp,
            comp_mid_lp,
            comp_high_hp,
            comp_low_threshold_db,
            comp_low_ratio,
            comp_low_attack_alpha,
            comp_low_release_alpha,
            comp_low_makeup_db,
            comp_low_makeup_lin,
            comp_mid_threshold_db,
            comp_mid_ratio,
            comp_mid_attack_alpha,
            comp_mid_release_alpha,
            comp_mid_makeup_db,
            comp_mid_makeup_lin,
            comp_high_threshold_db,
            comp_high_ratio,
            comp_high_attack_alpha,
            comp_high_release_alpha,
            comp_high_makeup_db,
            comp_high_makeup_lin,
            comp_knee_db,
            comp_link_stereo,
        }
    }
}

/// Apply an in-place M/S width transform to a stereo frame. `frame` must be
/// at least length 2; channels beyond index 1 are untouched. `side_scale`
/// scales the L-R component (0 collapses to mono, 1 is a no-op, > 1 widens).
///
/// Energy budgeting: this is the textbook lossless M/S decode/encode pair, so
/// `side_scale = 1` is exactly identity, and other scales redistribute energy
/// between mid and side without introducing gain on the post-transform signal
/// when summed across both channels. The limiter downstream catches any peak
/// excursions that widening introduces on individual channels.
#[inline]
pub(crate) fn apply_width_stereo(frame: &mut [f32], side_scale: f32) {
    if frame.len() < 2 {
        return;
    }
    let l = frame[0];
    let r = frame[1];
    let mid = 0.5 * (l + r);
    let side = 0.5 * (l - r) * side_scale;
    frame[0] = mid + side;
    frame[1] = mid - side;
}

#[derive(Debug, Clone, Default)]
pub struct ChannelState {
    sub_hp1: BiquadState,
    sub_hp2: BiquadState,
    low: BiquadState,
    /// Phase A2: state for the 400 Hz peaking band.
    low_mid: BiquadState,
    mid: BiquadState,
    high: BiquadState,
    warmth: BiquadState,
    presence_air: BiquadState,
    transient_fast_env: f32,
    transient_slow_env: f32,
    // Phase 12.2: multiband compressor — per-channel crossover network state.
    comp_split: LR4State,
    // Per-channel per-band envelope follower. Used directly when
    // `comp_link_stereo = false`; when linked, all channels' envelopes are
    // driven by the same max-of-channels detector input, but each channel
    // still keeps its own follower so the swap-on-toggle stays smooth.
    comp_low_env: f32,
    comp_mid_env: f32,
    comp_high_env: f32,
}

// ============================================================================
// Limiter — linked-stereo brick-wall limiter with lookahead.
// Phase 11.2.a: sample-peak detection, instant attack, exponential release.
// Phase 11.2.b: 2× upsample inter-sample peak via Lagrange-4 midpoint (x=0.5).
// Phase 11.2.c: 4× upsample by also evaluating x=0.25 and x=0.75. The three
//   coefficient triplets below are the 4-point Lagrange basis polynomials
//   evaluated at fractional positions 0.25, 0.5, and 0.75 between samples
//   `b` and `c`, with neighbors `a` and `d` providing curvature. ITU-R
//   BS.1770 recommends ≥ 4× oversampling for true-peak; this estimator is a
//   close approximation that avoids the cost of a polyphase FIR.
// ============================================================================

/// 4-point Lagrange interpolator coefficients at three fractional positions
/// inside a (b, c) sample pair. For samples (a, b, c, d) at indices
/// (-1, 0, 1, 2), each row gives the basis weights at one of (x = 0.25, 0.5,
/// 0.75) so that `L(x) = w[0]·a + w[1]·b + w[2]·c + w[3]·d`.
///
/// Coefficients verified by hand-computing the 4-point Lagrange polynomial at
/// each fractional position. Each row sums to 1.0 (interpolation invariant).
const LAGRANGE_INTERSAMPLE_COEFFS: [[f32; 4]; 3] = [
    [-0.0546875, 0.8203125, 0.2734375, -0.0390625], // x = 0.25
    [-0.0625, 0.5625, 0.5625, -0.0625],             // x = 0.5
    [-0.0390625, 0.2734375, 0.8203125, -0.0546875], // x = 0.75 (mirror of 0.25)
];

#[derive(Debug, Clone)]
pub struct Limiter {
    channels: usize,
    ceiling_lin: f32,
    lookahead_frames: usize,
    release_coef: f32,
    /// Ring buffer of interleaved samples sized `lookahead_frames * channels`.
    buffer: Vec<f32>,
    /// Index of the next frame to overwrite (also the oldest frame in the buffer).
    head_frame: usize,
    /// How many frames have been written so far (capped at `lookahead_frames`).
    filled_frames: usize,
    /// Current linear gain reduction (1.0 = no reduction).
    gain: f32,
    /// Reusable scratch buffer for the oldest-frame read; preallocated to avoid
    /// per-frame allocations on the audio thread.
    oldest_frame_buf: Vec<f32>,
}

impl Limiter {
    pub fn new(
        sample_rate: u32,
        channels: usize,
        ceiling_dbfs: f32,
        lookahead_ms: f32,
        release_ms: f32,
    ) -> Self {
        let ch = channels.max(1);
        let lookahead_frames =
            (((lookahead_ms / 1000.0) * sample_rate as f32).round() as usize).max(1);
        let release_coef = if release_ms > 0.0 {
            (-1.0_f32 / (release_ms / 1000.0 * sample_rate as f32)).exp()
        } else {
            0.0
        };
        let ceiling_lin = 10.0_f32.powf(ceiling_dbfs / 20.0);
        Self {
            channels: ch,
            ceiling_lin,
            lookahead_frames,
            release_coef,
            buffer: vec![0.0; lookahead_frames * ch],
            head_frame: 0,
            filled_frames: 0,
            gain: 1.0,
            oldest_frame_buf: vec![0.0; ch],
        }
    }

    /// Process one frame in place. `frame.len()` must equal `self.channels`
    /// (smaller frames are zero-padded internally).
    pub fn process_frame_inplace(&mut self, frame: &mut [f32]) {
        let ch = self.channels;
        if ch == 0 {
            return;
        }
        let head_base = self.head_frame * ch;
        // Read the OLDEST frame from the ring before overwriting.
        for i in 0..ch {
            self.oldest_frame_buf[i] = self.buffer[head_base + i];
        }
        // Write the new frame into the ring.
        for i in 0..ch {
            let s = if i < frame.len() { frame[i] } else { 0.0 };
            self.buffer[head_base + i] = s;
        }
        self.head_frame = (self.head_frame + 1) % self.lookahead_frames;
        if self.filled_frames < self.lookahead_frames {
            self.filled_frames += 1;
        }

        // Scan the buffer for the peak. Two passes:
        //   1) Raw sample peaks (linked stereo, single max across channels).
        //   2) Lagrange-4 inter-sample peaks at x ∈ {0.25, 0.5, 0.75} between
        //      every adjacent frame pair — catches the true peak across the
        //      sub-sample positions that a 4× upsample would expose. Phase
        //      11.2.b checked x=0.5 only; sign-asymmetric patterns can place
        //      the true peak near x=0.25 or x=0.75 with a relatively small
        //      x=0.5 estimate, which is what this loop now covers.
        let mut peak: f32 = 0.0;
        for &s in &self.buffer {
            let a = s.abs();
            if a > peak {
                peak = a;
            }
        }
        // Phase A4 perf: the Lagrange-4 inter-sample peak loop is the
        // dominant cost in the limiter (O(lookahead × channels × 3
        // positions) per output frame). Inter-sample peaks can only
        // exceed the raw sample peak by at most ~+1 dB on real signals
        // (Lagrange-4 estimator bound; ITU-R BS.1770 says +0.5 dB for
        // typical material). If raw peak * 1.2 (≈ +1.6 dB margin) is
        // still below the ceiling, no possible inter-sample peak can
        // cross it — skip the loop entirely. Saves the heavy work on
        // any frame with ≥1.6 dB headroom, which on quiet material is
        // basically every frame.
        const ISP_SKIP_MARGIN: f32 = 1.2;
        let frames = self.filled_frames;
        if frames >= 4 && peak * ISP_SKIP_MARGIN > self.ceiling_lin {
            for f in 1..(frames - 2) {
                for c in 0..ch {
                    let prev = self.frame_sample(f - 1, c);
                    let a = self.frame_sample(f, c);
                    let b = self.frame_sample(f + 1, c);
                    let nxt = self.frame_sample(f + 2, c);
                    for w in &LAGRANGE_INTERSAMPLE_COEFFS {
                        let mid = w[0] * prev + w[1] * a + w[2] * b + w[3] * nxt;
                        let abs_mid = mid.abs();
                        if abs_mid > peak {
                            peak = abs_mid;
                        }
                    }
                }
            }
        }

        let required = if peak > self.ceiling_lin {
            self.ceiling_lin / peak.max(1.0e-9)
        } else {
            1.0
        };

        if required < self.gain {
            // Instant attack — the lookahead gives us time to ramp the OUTPUT
            // down before the peak hits the read pointer, so an instantaneous
            // gain change here translates to a smooth dip in the audible output.
            self.gain = required;
        } else {
            // Exponential release toward `required` (which is 1.0 when no
            // reduction is currently needed).
            self.gain = required - (required - self.gain) * self.release_coef;
        }

        // Output the OLDEST frame * current gain.
        for i in 0..frame.len().min(ch) {
            frame[i] = self.oldest_frame_buf[i] * self.gain;
        }
    }

    pub fn reset(&mut self) {
        for s in self.buffer.iter_mut() {
            *s = 0.0;
        }
        self.head_frame = 0;
        self.filled_frames = 0;
        self.gain = 1.0;
    }

    /// Read the channel sample at logical frame offset `f` (0 = oldest sample
    /// still in the buffer, `filled_frames - 1` = most recently written).
    fn frame_sample(&self, f: usize, c: usize) -> f32 {
        let actual_frame = (self.head_frame + f) % self.lookahead_frames;
        self.buffer[actual_frame * self.channels + c]
    }
}

// ============================================================================
// Phase 12.2 — LR4 crossover network for the multiband compressor. 3-way
// split via cascaded-Butterworth LP+LP (low) and HP+HP (high), with the mid
// band as the HP_120 → LP_4000 cascade. LR4 sums flat across all band edges
// (mathematical property of cascaded Butterworth at the same corner, no
// magnitude bump like LR2). All four cascade pairs hold their own state per
// channel — that's 8 biquads per channel for the split.
// ============================================================================

const LR4_CROSSOVER_LOW_HZ: f32 = 120.0;
const LR4_CROSSOVER_HIGH_HZ: f32 = 4000.0;
const BUTTERWORTH_Q: f32 = 0.707_106_8; // sqrt(2)/2

/// Per-channel filter memory for the LR4 split: two LP stages for the low
/// band, two HP stages and two LP stages for the mid band, two HP stages for
/// the high band. Default = all zero (no signal in history).
#[derive(Debug, Clone, Default)]
pub struct LR4State {
    pub low_lp1: BiquadState,
    pub low_lp2: BiquadState,
    pub mid_hp1: BiquadState,
    pub mid_hp2: BiquadState,
    pub mid_lp1: BiquadState,
    pub mid_lp2: BiquadState,
    pub high_hp1: BiquadState,
    pub high_hp2: BiquadState,
}

/// Test-only entry point: splits a single sample at sample_rate = 44_100 with
/// the LR4 crossovers fixed at 120 Hz and 4000 Hz. Production callers use
/// `MasteringChain::process_frame_inplace`, which fetches the coefficients
/// from `ChainCoeffs` (sample-rate-aware) and walks the same biquads in the
/// same order.
#[cfg(test)]
pub(crate) fn split_lr4_into_bands(x: f32, state: &mut LR4State) -> (f32, f32, f32) {
    let sr = 44_100.0f32;
    let low_lp_c = BiquadCoeffs::butter_lp(sr, LR4_CROSSOVER_LOW_HZ, BUTTERWORTH_Q);
    let mid_hp_c = BiquadCoeffs::butter_hp(sr, LR4_CROSSOVER_LOW_HZ, BUTTERWORTH_Q);
    let mid_lp_c = BiquadCoeffs::butter_lp(sr, LR4_CROSSOVER_HIGH_HZ, BUTTERWORTH_Q);
    let high_hp_c = BiquadCoeffs::butter_hp(sr, LR4_CROSSOVER_HIGH_HZ, BUTTERWORTH_Q);
    let low_a = state.low_lp1.process(&low_lp_c, x);
    let low = state.low_lp2.process(&low_lp_c, low_a);
    let mid_after_hp1 = state.mid_hp1.process(&mid_hp_c, x);
    let mid_after_hp2 = state.mid_hp2.process(&mid_hp_c, mid_after_hp1);
    let mid_after_lp1 = state.mid_lp1.process(&mid_lp_c, mid_after_hp2);
    let mid = state.mid_lp2.process(&mid_lp_c, mid_after_lp1);
    let high_a = state.high_hp1.process(&high_hp_c, x);
    let high = state.high_hp2.process(&high_hp_c, high_a);
    (low, mid, high)
}

/// Peak-detector envelope follower. One-pole smoothing with separate attack
/// and release time constants. `env_n = (alpha * env_{n-1}) + ((1 - alpha) *
/// |x_n|)` where `alpha = exp(-1 / (time_ms/1000 * sr))`. The selected alpha
/// depends on whether the signal is rising (use attack) or decaying (use
/// release).
#[derive(Debug, Clone)]
pub struct EnvelopeFollower {
    pub env: f32,
    pub alpha_attack: f32,
    pub alpha_release: f32,
}

impl EnvelopeFollower {
    pub fn new(sample_rate: f32, attack_ms: f32, release_ms: f32) -> Self {
        Self {
            env: 0.0,
            alpha_attack: alpha_from_time_ms(sample_rate, attack_ms),
            alpha_release: alpha_from_time_ms(sample_rate, release_ms),
        }
    }

    #[inline]
    pub fn process(&mut self, x_abs: f32) -> f32 {
        let alpha = if x_abs > self.env {
            self.alpha_attack
        } else {
            self.alpha_release
        };
        self.env = alpha * self.env + (1.0 - alpha) * x_abs;
        self.env
    }

    pub fn reset(&mut self) {
        self.env = 0.0;
    }
}

#[inline]
fn alpha_from_time_ms(sample_rate: f32, time_ms: f32) -> f32 {
    if time_ms <= 0.0 || sample_rate <= 0.0 {
        return 0.0;
    }
    (-1.0_f32 / (time_ms * 0.001 * sample_rate)).exp()
}

#[inline]
fn process_transient_shaper_sample(
    x: f32,
    amount: f32,
    fast_env: &mut f32,
    slow_env: &mut f32,
    fast_attack_alpha: f32,
    fast_release_alpha: f32,
    slow_attack_alpha: f32,
    slow_release_alpha: f32,
) -> f32 {
    let detector = x.abs();
    let fast_alpha = if detector > *fast_env {
        fast_attack_alpha
    } else {
        fast_release_alpha
    };
    let slow_alpha = if detector > *slow_env {
        slow_attack_alpha
    } else {
        slow_release_alpha
    };
    *fast_env = fast_alpha * *fast_env + (1.0 - fast_alpha) * detector;
    *slow_env = slow_alpha * *slow_env + (1.0 - slow_alpha) * detector;

    let diff = (*fast_env - *slow_env) / ((*slow_env).max(1.0e-4));
    let gain = (1.0 + amount * diff.clamp(-1.0, 1.0)).clamp(0.75, 1.25);
    x * gain
}

// ============================================================================
// BS.1770-4 momentary LUFS meter — conformant.
//
// Two-stage K-weighting prefilter (exact ITU-R BS.1770-4 coefficients via
// BiquadCoeffs::k_weighting_pre / k_weighting_rlb) followed by a true
// rectangular 400 ms sliding mean-square window. Output is converted to
// LUFS via M = -0.691 + 10·log10(sum_of_channel_energies).
//
// Stereo channel weights are 1.0 / 1.0 (no surround compensation). Energy
// gating (-70 LUFS absolute / -10 LU relative) is skipped — that's defined
// only for INTEGRATED loudness, not the momentary readout which by definition
// shows whatever is playing right now.
//
// Phase A1 of the Codex port plan: this replaces the previous one-pole IIR
// approximation and the 1500 Hz / slope-0.4 / Q-0.7071 K-weighting with the
// BS.1770-4 reference filters and a literal ring-buffer window.
// ============================================================================

const LUFS_MOMENTARY_WINDOW_MS: f64 = 400.0;
const LUFS_BS1770_OFFSET: f32 = -0.691;

#[derive(Debug, Clone)]
pub struct MomentaryLufs {
    /// BS.1770-4 K-weighting prefilter: Stage 1 = high-shelf @ 1681.97 Hz
    /// (+4 dB, Q≈0.7071), Stage 2 = RLB high-pass @ 38.14 Hz (Q≈0.5003).
    hs_coeffs: BiquadCoeffs,
    hp_coeffs: BiquadCoeffs,
    hs_state: [BiquadState; 2],
    hp_state: [BiquadState; 2],
    /// Ring buffer of per-sample summed channel-energy (l_k² + r_k²) over
    /// the most recent 400 ms. Sized at construction to
    /// `400 ms × sample_rate`. Running sum is maintained incrementally
    /// (add new, subtract displaced) so the per-frame cost is O(1) rather
    /// than O(window_size).
    ring: Vec<f64>,
    ring_pos: usize,
    ring_sum: f64,
    /// `false` until the ring has wrapped once. Before that, the sum is
    /// over fewer than `ring.len()` samples and `lufs()` returns -120.0.
    ring_filled: bool,
}

impl MomentaryLufs {
    pub fn new(sample_rate: u32) -> Self {
        let window_samples =
            ((LUFS_MOMENTARY_WINDOW_MS * 0.001 * sample_rate as f64).round() as usize).max(1);
        Self {
            hs_coeffs: BiquadCoeffs::k_weighting_pre(sample_rate),
            hp_coeffs: BiquadCoeffs::k_weighting_rlb(sample_rate),
            hs_state: [BiquadState::default(); 2],
            hp_state: [BiquadState::default(); 2],
            ring: vec![0.0; window_samples],
            ring_pos: 0,
            ring_sum: 0.0,
            ring_filled: false,
        }
    }

    /// Feed one stereo frame (left, right) and return the current momentary
    /// LUFS readout. For mono input pass the same sample for both channels;
    /// the BS.1770 sum-of-channels convention then produces a +3 LU
    /// stereo-vs-mono offset, which is by design of the standard.
    #[inline]
    pub fn process_frame(&mut self, left: f32, right: f32) -> f32 {
        let l_hs = self.hs_state[0].process(&self.hs_coeffs, left);
        let l_hp = self.hp_state[0].process(&self.hp_coeffs, l_hs);
        let r_hs = self.hs_state[1].process(&self.hs_coeffs, right);
        let r_hp = self.hp_state[1].process(&self.hp_coeffs, r_hs);
        let energy = (l_hp as f64) * (l_hp as f64) + (r_hp as f64) * (r_hp as f64);
        // Sliding-sum bookkeeping: replace the oldest slot, fix up the
        // running sum, clamp negatives that arise from f64 cancellation
        // drift over very long sessions.
        let displaced = self.ring[self.ring_pos];
        self.ring[self.ring_pos] = energy;
        self.ring_sum = (self.ring_sum - displaced + energy).max(0.0);
        self.ring_pos += 1;
        if self.ring_pos >= self.ring.len() {
            self.ring_pos = 0;
            self.ring_filled = true;
        }
        self.lufs()
    }

    /// Current momentary LUFS readout. Returns `-120.0` until the 400 ms
    /// ring has filled (i.e. fewer than 400 ms of audio have been fed),
    /// so the UI doesn't flash a junk number at the start of playback.
    pub fn lufs(&self) -> f32 {
        if !self.ring_filled {
            return -120.0;
        }
        let n = self.ring.len() as f64;
        let mean = self.ring_sum / n;
        if mean <= 1.0e-12 {
            return -120.0;
        }
        (LUFS_BS1770_OFFSET as f64 + 10.0 * mean.log10()) as f32
    }

    pub fn reset(&mut self) {
        self.hs_state = [BiquadState::default(); 2];
        self.hp_state = [BiquadState::default(); 2];
        for v in self.ring.iter_mut() {
            *v = 0.0;
        }
        self.ring_pos = 0;
        self.ring_sum = 0.0;
        self.ring_filled = false;
    }
}

// ============================================================================
// Phase 12.2 P3+ — BS.1770-4 integrated LUFS meter.
//
// Where MomentaryLufs is a 400 ms sliding readout for "what's playing right
// now," IntegratedLufs aggregates over the whole listen-through per the
// BS.1770-4 algorithm:
//   1. K-weight each sample (shared prefilter shape with MomentaryLufs, but
//      separate filter state so the two integrators can be reset independently).
//   2. Compute mean-square energy over 400 ms rectangular blocks at 75 %
//      overlap — a new block is emitted every 100 ms.
//   3. Absolute gate: drop blocks below -70 LUFS.
//   4. Relative gate: drop blocks below (mean of absolute-gated blocks - 10 LU).
//   5. Integrated loudness = -0.691 + 10·log10(mean of remaining block energies).
//
// We cache the computed value at block-emit time (every 100 ms) instead of
// recomputing on every UI tick — the O(N) re-scan grows with listen-through
// length and would otherwise add up over multi-track sessions.
// ============================================================================

const LUFS_INTEGRATED_BLOCK_MS: f32 = 400.0;
const LUFS_INTEGRATED_STEP_MS: f32 = 100.0;
const LUFS_ABS_GATE_LUFS: f32 = -70.0;
const LUFS_REL_GATE_LU: f32 = 10.0;

#[derive(Debug, Clone)]
pub struct IntegratedLufs {
    hs_coeffs: BiquadCoeffs,
    hp_coeffs: BiquadCoeffs,
    hs_state: [BiquadState; 2],
    hp_state: [BiquadState; 2],
    /// Ring buffer of per-sample channel-summed squared values (post K-weighting),
    /// sized to one 400 ms block. The running sum is maintained incrementally as
    /// each new sample replaces the oldest, so the per-frame cost is O(1) rather
    /// than O(block_size).
    ring: Vec<f64>,
    ring_pos: usize,
    ring_sum: f64,
    ring_filled: bool,
    block_size: usize,
    block_step: usize,
    samples_since_step: usize,
    /// (block_mean_sq, block_loudness) for every block that passed the absolute
    /// gate. Storing pre-computed block_loudness keeps the relative-gate scan
    /// in the cheap addition/compare regime — no log10 in the hot recompute.
    blocks: Vec<(f64, f32)>,
    /// Cached BS.1770-4 integrated value. Recomputed only at block-emit time
    /// (every 100 ms), so `lufs()` is O(1) for UI ticks.
    cached_lufs: f32,
}

impl IntegratedLufs {
    pub fn new(sample_rate: u32) -> Self {
        let sr = sample_rate as f32;
        // Phase A1 of the port plan moved the momentary meter onto the
        // BS.1770-4 reference K-weighting builders. The integrated meter
        // is conceptually the same K-weighting prefilter followed by a
        // block-energy aggregator with gating — so it inherits the same
        // reference filters here for consistency.
        let hs_coeffs = BiquadCoeffs::k_weighting_pre(sample_rate);
        let hp_coeffs = BiquadCoeffs::k_weighting_rlb(sample_rate);
        let block_size =
            ((LUFS_INTEGRATED_BLOCK_MS * 0.001 * sr).round() as usize).max(1);
        let block_step =
            ((LUFS_INTEGRATED_STEP_MS * 0.001 * sr).round() as usize).max(1);
        Self {
            hs_coeffs,
            hp_coeffs,
            hs_state: [BiquadState::default(); 2],
            hp_state: [BiquadState::default(); 2],
            ring: vec![0.0; block_size],
            ring_pos: 0,
            ring_sum: 0.0,
            ring_filled: false,
            block_size,
            block_step,
            samples_since_step: 0,
            blocks: Vec::new(),
            cached_lufs: -120.0,
        }
    }

    /// Feed one stereo frame (left, right). Returns the current integrated
    /// LUFS reading (cached between block boundaries, so cheap).
    #[inline]
    pub fn process_frame(&mut self, left: f32, right: f32) -> f32 {
        let l_hs = self.hs_state[0].process(&self.hs_coeffs, left);
        let l_hp = self.hp_state[0].process(&self.hp_coeffs, l_hs);
        let r_hs = self.hs_state[1].process(&self.hs_coeffs, right);
        let r_hp = self.hp_state[1].process(&self.hp_coeffs, r_hs);
        let energy = (l_hp as f64) * (l_hp as f64) + (r_hp as f64) * (r_hp as f64);
        // Slide the ring window: subtract the value being displaced, add the new.
        let displaced = self.ring[self.ring_pos];
        self.ring[self.ring_pos] = energy;
        self.ring_sum = self.ring_sum - displaced + energy;
        self.ring_pos += 1;
        if self.ring_pos >= self.block_size {
            self.ring_pos = 0;
            self.ring_filled = true;
        }
        self.samples_since_step += 1;
        if self.samples_since_step >= self.block_step && self.ring_filled {
            self.samples_since_step = 0;
            // Guard against negative ring_sum from f64 cancellation drift over
            // long sessions. The true mean-square is non-negative by definition.
            let block_mean_sq = (self.ring_sum / self.block_size as f64).max(0.0);
            if block_mean_sq > 1.0e-12 {
                let block_loudness =
                    (LUFS_BS1770_OFFSET as f64 + 10.0 * block_mean_sq.log10()) as f32;
                if block_loudness >= LUFS_ABS_GATE_LUFS {
                    self.blocks.push((block_mean_sq, block_loudness));
                    self.cached_lufs = self.compute_integrated();
                }
            }
        }
        self.cached_lufs
    }

    fn compute_integrated(&self) -> f32 {
        if self.blocks.is_empty() {
            return -120.0;
        }
        let abs_gated_sum: f64 = self.blocks.iter().map(|&(e, _)| e).sum();
        let abs_gated_mean = abs_gated_sum / self.blocks.len() as f64;
        if abs_gated_mean <= 1.0e-12 {
            return -120.0;
        }
        let abs_gated_lufs = LUFS_BS1770_OFFSET as f64 + 10.0 * abs_gated_mean.log10();
        let rel_threshold = (abs_gated_lufs - LUFS_REL_GATE_LU as f64) as f32;
        let mut rel_gated_sum = 0.0f64;
        let mut rel_gated_count = 0u64;
        for &(e, loudness) in &self.blocks {
            if loudness >= rel_threshold {
                rel_gated_sum += e;
                rel_gated_count += 1;
            }
        }
        if rel_gated_count == 0 {
            return -120.0;
        }
        let rel_gated_mean = rel_gated_sum / rel_gated_count as f64;
        if rel_gated_mean <= 1.0e-12 {
            return -120.0;
        }
        (LUFS_BS1770_OFFSET as f64 + 10.0 * rel_gated_mean.log10()) as f32
    }

    /// Current integrated LUFS readout (cached, O(1)). `-120.0` until at least
    /// one 400 ms block has been completed and passed the absolute gate.
    pub fn lufs(&self) -> f32 {
        self.cached_lufs
    }

    pub fn reset(&mut self) {
        self.hs_state = [BiquadState::default(); 2];
        self.hp_state = [BiquadState::default(); 2];
        for v in self.ring.iter_mut() {
            *v = 0.0;
        }
        self.ring_pos = 0;
        self.ring_sum = 0.0;
        self.ring_filled = false;
        self.samples_since_step = 0;
        self.blocks.clear();
        self.cached_lufs = -120.0;
    }
}

/// Phase 12.2 — per-band gain-reduction snapshots. `MasteringChain` writes
/// per-frame max-|reduction_db| into these atomics; the audio thread reads
/// via `swap` on the 50 ms snapshot cycle, mirroring the existing
/// `peak_linear` pattern. Integer storage (|reduction_db| * 100 as u32) avoids
/// the IEEE 754 sign-bit ordering edge case for negative dB values. 0 = no
/// reduction in the window.
#[derive(Debug, Default)]
pub struct GrSnapshotSlots {
    pub low: std::sync::Arc<std::sync::atomic::AtomicU32>,
    pub mid: std::sync::Arc<std::sync::atomic::AtomicU32>,
    pub high: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl Clone for GrSnapshotSlots {
    fn clone(&self) -> Self {
        Self {
            low: self.low.clone(),
            mid: self.mid.clone(),
            high: self.high.clone(),
        }
    }
}

pub struct MasteringChain {
    pub coeffs: ChainCoeffs,
    pub states: Vec<ChannelState>,
    pub limiter: Limiter,
    pub gr_snapshots: GrSnapshotSlots,
}

const LIMITER_LOOKAHEAD_MS: f32 = 3.0;
const LIMITER_RELEASE_MS: f32 = 50.0;

impl MasteringChain {
    pub fn new(sample_rate: u32, channels: usize, settings: &MasteringSettings) -> Self {
        let coeffs = ChainCoeffs::from_settings(sample_rate, settings);
        let states = (0..channels).map(|_| ChannelState::default()).collect();
        let ceiling_dbfs = settings.effective_ceiling_dbtp().clamp(-6.0, 0.0);
        let limiter = Limiter::new(
            sample_rate,
            channels,
            ceiling_dbfs,
            LIMITER_LOOKAHEAD_MS,
            LIMITER_RELEASE_MS,
        );
        Self {
            coeffs,
            states,
            limiter,
            gr_snapshots: GrSnapshotSlots::default(),
        }
    }

    /// Construct a chain that writes gain-reduction snapshots into the
    /// provided shared atomic slots. Used by `MasteringSource` so the
    /// audio thread's `AudioThreadState` shares the same atomics with the
    /// chain inside the running source.
    pub fn new_with_gr_snapshots(
        sample_rate: u32,
        channels: usize,
        settings: &MasteringSettings,
        gr_snapshots: GrSnapshotSlots,
    ) -> Self {
        let mut chain = Self::new(sample_rate, channels, settings);
        chain.gr_snapshots = gr_snapshots;
        chain
    }

    /// Build a sibling chain that inherits the current filter + limiter state
    /// but uses fresh coefficients. Used by `MasteringSource` to crossfade
    /// between old and new coefficients without re-ringing the filters or
    /// dropping the limiter's gain envelope from zero state.
    pub fn with_coeffs_inheriting_state(coeffs: ChainCoeffs, prior: &Self) -> Self {
        Self {
            coeffs,
            states: prior.states.clone(),
            limiter: prior.limiter.clone(),
            gr_snapshots: prior.gr_snapshots.clone(),
        }
    }

    /// Process one interleaved frame in place. Runs gain → 3-band EQ →
    /// (optional stereo width) → saturation per channel, then the
    /// linked-stereo lookahead limiter across the frame. Width is inserted
    /// between EQ and saturation so the M/S decode sees the equalized signal
    /// but isn't fed through the non-linear stage twice; widening then
    /// saturating preserves the chosen stereo image instead of having the
    /// non-linearity smear it back toward mono.
    pub fn process_frame_inplace(&mut self, frame: &mut [f32]) {
        let channels = frame.len().min(self.states.len());
        if channels == 0 {
            return;
        }
        // Pass 1: per-channel input gain + 4-band EQ.
        // Phase A2: low-mid peaking band inserted between low and mid so the
        // mud-zone cleanup (250–800 Hz) sits in the natural frequency order.
        for ch in 0..channels {
            let state = &mut self.states[ch];
            let mut y = frame[ch] * self.coeffs.input_gain_lin;
            let hp1 = state.sub_hp1.process(&self.coeffs.sub_highpass, y);
            y = state.sub_hp2.process(&self.coeffs.sub_highpass, hp1);
            y = state.low.process(&self.coeffs.low, y);
            y = state.low_mid.process(&self.coeffs.low_mid, y);
            y = state.mid.process(&self.coeffs.mid, y);
            y = state.high.process(&self.coeffs.high, y);
            y = state.warmth.process(&self.coeffs.warmth, y);
            y = state.presence_air.process(&self.coeffs.presence_air, y);
            frame[ch] = y;
        }
        // Phase 12.2 — 3-band multiband downward compressor (LR4 split,
        // peak-detector envelope followers, soft 6 dB knee, auto makeup).
        // Position: between presence_air (end of EQ) and width (start of M/S
        // / saturation). Identity early-return when inactive — preserves
        // byte-equivalence with all existing real-fixture tests when the
        // slider is untouched.
        if self.coeffs.compression_active {
            self.apply_multiband_compressor(frame, channels);
        }
        // Transient shaping sits after the multiband compressor so Punch/Loud
        // can add attack without immediately feeding that lift into compressor
        // gain reduction. Musical tuning is still per-preset, but the topology
        // keeps the shaper's mechanical effect predictable.
        for ch in 0..channels {
            let state = &mut self.states[ch];
            if self.coeffs.transient_amount.abs() > 1.0e-5 {
                frame[ch] = process_transient_shaper_sample(
                    frame[ch],
                    self.coeffs.transient_amount,
                    &mut state.transient_fast_env,
                    &mut state.transient_slow_env,
                    self.coeffs.transient_fast_attack_alpha,
                    self.coeffs.transient_fast_release_alpha,
                    self.coeffs.transient_slow_attack_alpha,
                    self.coeffs.transient_slow_release_alpha,
                );
            }
        }
        // Width: only meaningful for stereo. The `≈ 1` guard skips the M/S
        // dance when the user hasn't touched the slider, keeping the
        // mono-summed signal mathematically identical to the pre-Phase-12.2
        // chain output for backward compatibility with existing tests.
        if channels == 2 && (self.coeffs.width_side_scale - 1.0).abs() > 1.0e-5 {
            apply_width_stereo(frame, self.coeffs.width_side_scale);
        }
        // Pass 2: per-channel saturation. Pulled out of pass 1 so width can
        // sit between EQ and the non-linear stage.
        if self.coeffs.saturation_amount > 0.0 {
            let drive = 1.0 + self.coeffs.saturation_amount * 2.0;
            let denom = drive.tanh().max(1.0e-3);
            for ch in 0..channels {
                frame[ch] = (frame[ch] * drive).tanh() / denom;
            }
        }
        self.limiter.process_frame_inplace(frame);
        // Volume Match: applied AFTER the limiter so the limiter still sees
        // the full post-gain peaks (and bounds them to the ceiling). The VM
        // scalar then attenuates the limited output down to source-matched
        // level for fair A/B comparison.
        if (self.coeffs.volume_match_gain_lin - 1.0).abs() > 1.0e-4 {
            for s in frame.iter_mut() {
                *s *= self.coeffs.volume_match_gain_lin;
            }
        }
        // User output gain — final trim. Applied last so it scales the
        // already-processed, already-limited signal. Boosting here CAN push
        // peaks above the ceiling (the user is asking for that level); the
        // export receipt's true-peak check catches it.
        if (self.coeffs.user_output_gain_lin - 1.0).abs() > 1.0e-4 {
            for s in frame.iter_mut() {
                *s *= self.coeffs.user_output_gain_lin;
            }
        }
        // Live export-preview gain: mirrors the offline render's final
        // down-only LUFS landing trim, which happens after the full chain.
        if (self.coeffs.export_landing_gain_lin - 1.0).abs() > 1.0e-4 {
            for s in frame.iter_mut() {
                *s *= self.coeffs.export_landing_gain_lin;
            }
        }
    }

    fn apply_multiband_compressor(&mut self, frame: &mut [f32], channels: usize) {
        let mut bands: [[f32; 3]; 2] = [[0.0; 3]; 2];
        let ch_active = channels.min(2);
        for ch in 0..ch_active {
            let state = &mut self.states[ch];
            let x = frame[ch];
            let low_a = state.comp_split.low_lp1.process(&self.coeffs.comp_low_lp, x);
            let low = state.comp_split.low_lp2.process(&self.coeffs.comp_low_lp, low_a);
            let m1 = state.comp_split.mid_hp1.process(&self.coeffs.comp_mid_hp, x);
            let m2 = state.comp_split.mid_hp2.process(&self.coeffs.comp_mid_hp, m1);
            let m3 = state.comp_split.mid_lp1.process(&self.coeffs.comp_mid_lp, m2);
            let mid = state.comp_split.mid_lp2.process(&self.coeffs.comp_mid_lp, m3);
            let h1 = state.comp_split.high_hp1.process(&self.coeffs.comp_high_hp, x);
            let high = state.comp_split.high_hp2.process(&self.coeffs.comp_high_hp, h1);
            bands[ch] = [low, mid, high];
        }

        let mut gain_lin: [[f32; 3]; 2] = [[1.0; 3]; 2];
        let mut max_gr_db_low: f32 = 0.0;
        let mut max_gr_db_mid: f32 = 0.0;
        let mut max_gr_db_high: f32 = 0.0;
        let knee = self.coeffs.comp_knee_db;
        let link = self.coeffs.comp_link_stereo;
        let band_params: [(f32, f32, f32, f32); 3] = [
            (
                self.coeffs.comp_low_threshold_db,
                self.coeffs.comp_low_ratio,
                self.coeffs.comp_low_attack_alpha,
                self.coeffs.comp_low_release_alpha,
            ),
            (
                self.coeffs.comp_mid_threshold_db,
                self.coeffs.comp_mid_ratio,
                self.coeffs.comp_mid_attack_alpha,
                self.coeffs.comp_mid_release_alpha,
            ),
            (
                self.coeffs.comp_high_threshold_db,
                self.coeffs.comp_high_ratio,
                self.coeffs.comp_high_attack_alpha,
                self.coeffs.comp_high_release_alpha,
            ),
        ];

        for b in 0..3 {
            let (thr_db, ratio, alpha_a, alpha_r) = band_params[b];
            let mut linked_x: f32 = 0.0;
            if link {
                for ch in 0..ch_active {
                    let a = bands[ch][b].abs();
                    if a > linked_x {
                        linked_x = a;
                    }
                }
            }
            for ch in 0..ch_active {
                let detector = if link {
                    linked_x
                } else {
                    bands[ch][b].abs()
                };
                let env_ref = match b {
                    0 => &mut self.states[ch].comp_low_env,
                    1 => &mut self.states[ch].comp_mid_env,
                    _ => &mut self.states[ch].comp_high_env,
                };
                let alpha = if detector > *env_ref { alpha_a } else { alpha_r };
                *env_ref = alpha * (*env_ref) + (1.0 - alpha) * detector;
                let env = *env_ref;
                let env_db = if env <= 1.0e-7 {
                    -140.0
                } else {
                    20.0 * env.log10()
                };
                let half_knee = knee * 0.5;
                let gr_db = if env_db < thr_db - half_knee {
                    0.0
                } else if env_db > thr_db + half_knee {
                    (env_db - thr_db) * (1.0 - 1.0 / ratio)
                } else {
                    let x = env_db - (thr_db - half_knee);
                    let t = x / knee;
                    let above = (env_db - thr_db) * (1.0 - 1.0 / ratio);
                    t * t * above.max(0.0)
                };
                let gr_abs = gr_db.max(0.0);
                // Phase A4 perf: skip the powf entirely when this band/
                // channel is sub-threshold (the dominant case on quiet
                // material — avoids ~6 powf calls per stereo frame at
                // 44.1k = ~265k powf/sec). When reduction IS happening,
                // use exp(g * LN10/20) instead of powf — exp is ~2× faster
                // than powf for non-trivial bases.
                let g_lin = if gr_abs <= 0.0 {
                    1.0
                } else {
                    const DB_TO_LIN: f32 = std::f32::consts::LN_10 / 20.0;
                    (-gr_abs * DB_TO_LIN).exp()
                };
                gain_lin[ch][b] = g_lin;
                match b {
                    0 => {
                        if gr_abs > max_gr_db_low {
                            max_gr_db_low = gr_abs;
                        }
                    }
                    1 => {
                        if gr_abs > max_gr_db_mid {
                            max_gr_db_mid = gr_abs;
                        }
                    }
                    _ => {
                        if gr_abs > max_gr_db_high {
                            max_gr_db_high = gr_abs;
                        }
                    }
                }
            }
        }

        for ch in 0..ch_active {
            let [low, mid, high] = bands[ch];
            let y = low * gain_lin[ch][0] * self.coeffs.comp_low_makeup_lin
                + mid * gain_lin[ch][1] * self.coeffs.comp_mid_makeup_lin
                + high * gain_lin[ch][2] * self.coeffs.comp_high_makeup_lin;
            frame[ch] = y;
        }

        use std::sync::atomic::Ordering;
        let to_u = |db: f32| (db.max(0.0) * 100.0) as u32;
        self.gr_snapshots.low.fetch_max(to_u(max_gr_db_low), Ordering::Relaxed);
        self.gr_snapshots.mid.fetch_max(to_u(max_gr_db_mid), Ordering::Relaxed);
        self.gr_snapshots.high.fetch_max(to_u(max_gr_db_high), Ordering::Relaxed);
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32], channels: usize) {
        if channels == 0 || self.states.is_empty() {
            return;
        }
        for frame in samples.chunks_mut(channels) {
            self.process_frame_inplace(frame);
        }
    }

    /// Per-sample API. Bypasses the linked-stereo limiter (which needs a full
    /// frame to compute peaks). Used only as a legacy path for callers that
    /// haven't been migrated to `process_frame_inplace`; the soft-clip ceiling
    /// stays in place as a degraded fallback.
    pub fn process_sample(&mut self, sample: f32, channel: usize) -> f32 {
        let idx = if self.states.is_empty() {
            return sample;
        } else {
            channel.min(self.states.len() - 1)
        };
        let state = &mut self.states[idx];
        let mut y = sample * self.coeffs.input_gain_lin;
        let hp1 = state.sub_hp1.process(&self.coeffs.sub_highpass, y);
        y = state.sub_hp2.process(&self.coeffs.sub_highpass, hp1);
        y = state.low.process(&self.coeffs.low, y);
        y = state.mid.process(&self.coeffs.mid, y);
        y = state.high.process(&self.coeffs.high, y);
        y = state.warmth.process(&self.coeffs.warmth, y);
        y = state.presence_air.process(&self.coeffs.presence_air, y);
        if self.coeffs.compression_active {
            let state = &mut self.states[idx];
            let low_a = state.comp_split.low_lp1.process(&self.coeffs.comp_low_lp, y);
            let low = state.comp_split.low_lp2.process(&self.coeffs.comp_low_lp, low_a);
            let m1 = state.comp_split.mid_hp1.process(&self.coeffs.comp_mid_hp, y);
            let m2 = state.comp_split.mid_hp2.process(&self.coeffs.comp_mid_hp, m1);
            let m3 = state.comp_split.mid_lp1.process(&self.coeffs.comp_mid_lp, m2);
            let mid = state.comp_split.mid_lp2.process(&self.coeffs.comp_mid_lp, m3);
            let h1 = state.comp_split.high_hp1.process(&self.coeffs.comp_high_hp, y);
            let high = state.comp_split.high_hp2.process(&self.coeffs.comp_high_hp, h1);
            let bands = [low, mid, high];
            let band_params: [(f32, f32, f32, f32); 3] = [
                (
                    self.coeffs.comp_low_threshold_db,
                    self.coeffs.comp_low_ratio,
                    self.coeffs.comp_low_attack_alpha,
                    self.coeffs.comp_low_release_alpha,
                ),
                (
                    self.coeffs.comp_mid_threshold_db,
                    self.coeffs.comp_mid_ratio,
                    self.coeffs.comp_mid_attack_alpha,
                    self.coeffs.comp_mid_release_alpha,
                ),
                (
                    self.coeffs.comp_high_threshold_db,
                    self.coeffs.comp_high_ratio,
                    self.coeffs.comp_high_attack_alpha,
                    self.coeffs.comp_high_release_alpha,
                ),
            ];
            let makeup_lin = [
                self.coeffs.comp_low_makeup_lin,
                self.coeffs.comp_mid_makeup_lin,
                self.coeffs.comp_high_makeup_lin,
            ];
            let knee = self.coeffs.comp_knee_db;
            let mut sum_y = 0.0f32;
            for b in 0..3 {
                let (thr_db, ratio, alpha_a, alpha_r) = band_params[b];
                let env_ref = match b {
                    0 => &mut state.comp_low_env,
                    1 => &mut state.comp_mid_env,
                    _ => &mut state.comp_high_env,
                };
                let detector = bands[b].abs();
                let alpha = if detector > *env_ref { alpha_a } else { alpha_r };
                *env_ref = alpha * (*env_ref) + (1.0 - alpha) * detector;
                let env = *env_ref;
                let env_db = if env <= 1.0e-7 {
                    -140.0
                } else {
                    20.0 * env.log10()
                };
                let half_knee = knee * 0.5;
                let gr_db = if env_db < thr_db - half_knee {
                    0.0
                } else if env_db > thr_db + half_knee {
                    (env_db - thr_db) * (1.0 - 1.0 / ratio)
                } else {
                    let x = env_db - (thr_db - half_knee);
                    let t = x / knee;
                    let above = (env_db - thr_db) * (1.0 - 1.0 / ratio);
                    t * t * above.max(0.0)
                };
                let g_lin = 10.0_f32.powf(-gr_db.max(0.0) / 20.0);
                sum_y += bands[b] * g_lin * makeup_lin[b];
            }
            y = sum_y;
        }
        // Legacy path mirrors process_frame_inplace topology: compressor
        // first, then transient shaping, so transient lift is not immediately
        // folded back into compressor gain reduction.
        if self.coeffs.transient_amount.abs() > 1.0e-5 {
            let state = &mut self.states[idx];
            y = process_transient_shaper_sample(
                y,
                self.coeffs.transient_amount,
                &mut state.transient_fast_env,
                &mut state.transient_slow_env,
                self.coeffs.transient_fast_attack_alpha,
                self.coeffs.transient_fast_release_alpha,
                self.coeffs.transient_slow_attack_alpha,
                self.coeffs.transient_slow_release_alpha,
            );
        }
        if self.coeffs.saturation_amount > 0.0 {
            let drive = 1.0 + self.coeffs.saturation_amount * 2.0;
            y = (y * drive).tanh() / drive.tanh().max(1.0e-3);
        }
        let ceiling = self.coeffs.ceiling_lin;
        if y.abs() > ceiling {
            let over = y.abs() - ceiling;
            let shaped = ceiling + over.tanh() * 0.05;
            y = y.signum() * shaped;
        }
        y
    }

    pub fn reset_states(&mut self) {
        for state in self.states.iter_mut() {
            *state = ChannelState::default();
        }
        self.limiter.reset();
    }
}

// ============================================================================
// Tests — Phase 12.2 stereo width / M-S processing.
// `apply_width_stereo` is tested directly so the M/S math is pinned without
// having to drive samples through the full limiter lookahead. A separate
// integration test exercises the wiring inside `process_frame_inplace`.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    /// Magnitude (in dB) of a biquad's frequency response at a given Hz value.
    /// Evaluates the transfer function `H(z) = (b0 + b1*z^-1 + b2*z^-2) /
    /// (1 + a1*z^-1 + a2*z^-2)` at `z = e^(j*omega)` where `omega = 2*pi*f/sr`.
    /// Used to verify shelf gain at well-below-corner and well-above-corner
    /// frequencies without running audio through the chain.
    fn biquad_magnitude_db_at(c: &BiquadCoeffs, freq_hz: f32, sample_rate: f32) -> f32 {
        let omega = 2.0 * std::f32::consts::PI * freq_hz / sample_rate;
        let cos_o = omega.cos();
        let sin_o = omega.sin();
        let z1_re = cos_o;
        let z1_im = -sin_o;
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

    fn cascaded_magnitude_db_at(
        c: &BiquadCoeffs,
        stages: usize,
        freq_hz: f32,
        sample_rate: f32,
    ) -> f32 {
        biquad_magnitude_db_at(c, freq_hz, sample_rate) * stages as f32
    }

    #[test]
    fn preset_highpass_cutoffs_are_mastering_subsonic_or_neutral_custom() {
        let presets = [
            PRESET_UNIVERSAL,
            PRESET_CLARITY,
            PRESET_TAPE,
            PRESET_SPATIAL,
            PRESET_OOMPH,
            PRESET_WARMTH,
            PRESET_PUNCH,
            PRESET_LOUD,
        ];
        for preset in presets {
            assert!(
                (20.0..=40.0).contains(&preset.highpass_hz),
                "preset HPF must stay in the mastering subsonic range, got {}",
                preset.highpass_hz
            );
        }
        assert_eq!(PRESET_CUSTOM_NEUTRAL.highpass_hz, 0.0);
    }

    #[test]
    fn preset_highpass_is_lr4_subsonic_filter_before_eq() {
        let mut settings = default_master_settings();
        settings.preset = Preset::Universal;
        let coeffs = ChainCoeffs::from_settings(48_000, &settings);

        let sub_10_hz = cascaded_magnitude_db_at(&coeffs.sub_highpass, 2, 10.0, 48_000.0);
        let audible_100_hz = cascaded_magnitude_db_at(&coeffs.sub_highpass, 2, 100.0, 48_000.0);

        assert!(
            sub_10_hz < -25.0,
            "Universal HPF should strongly attenuate 10 Hz rumble, got {sub_10_hz:.2} dB"
        );
        assert!(
            audible_100_hz.abs() < 0.2,
            "Universal HPF should leave 100 Hz essentially intact, got {audible_100_hz:.2} dB"
        );
    }

    #[test]
    fn custom_highpass_is_identity() {
        let mut settings = default_master_settings();
        settings.preset = Preset::Custom { id: "neutral".into() };
        let coeffs = ChainCoeffs::from_settings(48_000, &settings);
        assert!(approx_eq(coeffs.sub_highpass.b0, 1.0, 1e-6));
        assert!(approx_eq(coeffs.sub_highpass.b1, 0.0, 1e-6));
        assert!(approx_eq(coeffs.sub_highpass.b2, 0.0, 1e-6));
    }

    #[test]
    fn preset_transient_punch_wires_into_chain_coefficients() {
        let mut punch = default_master_settings();
        punch.preset = Preset::Punch;
        punch.intensity = 0.5;
        let punch_coeffs = ChainCoeffs::from_settings(48_000, &punch);

        let mut warmth = default_master_settings();
        warmth.preset = Preset::Warmth;
        warmth.intensity = 0.5;
        let warmth_coeffs = ChainCoeffs::from_settings(48_000, &warmth);

        let mut custom = default_master_settings();
        custom.preset = Preset::Custom { id: "neutral".into() };
        let custom_coeffs = ChainCoeffs::from_settings(48_000, &custom);

        assert!(
            punch_coeffs.transient_amount > 0.1,
            "Punch should carry a positive transient amount, got {}",
            punch_coeffs.transient_amount
        );
        assert!(
            warmth_coeffs.transient_amount < -0.04,
            "Warmth should carry a softened transient amount, got {}",
            warmth_coeffs.transient_amount
        );
        assert!(approx_eq(custom_coeffs.transient_amount, 0.0, 1e-6));
    }

    #[test]
    fn transient_shaper_positive_amount_boosts_attack() {
        let mut fast = 0.0;
        let mut slow = 0.0;
        let fast_attack = alpha_from_time_ms(48_000.0, 1.0);
        let fast_release = alpha_from_time_ms(48_000.0, 35.0);
        let slow_attack = alpha_from_time_ms(48_000.0, 15.0);
        let slow_release = alpha_from_time_ms(48_000.0, 140.0);

        let out = process_transient_shaper_sample(
            0.5,
            0.2,
            &mut fast,
            &mut slow,
            fast_attack,
            fast_release,
            slow_attack,
            slow_release,
        );

        assert!(
            out > 0.5,
            "positive transient amount should boost an attack, got input 0.5 output {out}"
        );
    }

    #[test]
    fn transient_shaper_negative_amount_softens_attack() {
        let mut fast = 0.0;
        let mut slow = 0.0;
        let fast_attack = alpha_from_time_ms(48_000.0, 1.0);
        let fast_release = alpha_from_time_ms(48_000.0, 35.0);
        let slow_attack = alpha_from_time_ms(48_000.0, 15.0);
        let slow_release = alpha_from_time_ms(48_000.0, 140.0);

        let out = process_transient_shaper_sample(
            0.5,
            -0.2,
            &mut fast,
            &mut slow,
            fast_attack,
            fast_release,
            slow_attack,
            slow_release,
        );

        assert!(
            out < 0.5,
            "negative transient amount should soften an attack, got input 0.5 output {out}"
        );
    }

    #[test]
    fn process_sample_legacy_path_applies_transient_after_compression() {
        let mut settings = default_master_settings();
        settings.preset = Preset::Punch;
        settings.intensity = 0.5;
        settings.advanced.compression_density = Some(1.0);
        let mut shaped = MasteringChain::new(48_000, 1, &settings);
        let mut unshaped = MasteringChain::new(48_000, 1, &settings);
        unshaped.coeffs.transient_amount = 0.0;

        let shaped_out = shaped.process_sample(0.5, 0);
        let unshaped_out = unshaped.process_sample(0.5, 0);

        assert!(
            shaped_out > unshaped_out,
            "legacy process_sample should apply positive transient shaping after compression; shaped={shaped_out} unshaped={unshaped_out}"
        );
    }

    /// Width 0 collapses the stereo image to mono. For an L = sine, R = -sine
    /// input (pure-side signal), this means both channels go to zero.
    #[test]
    fn apply_width_stereo_zero_collapses_to_mono() {
        let mut frame = [0.5f32, -0.5];
        apply_width_stereo(&mut frame, 0.0);
        assert!(
            approx_eq(frame[0], frame[1], 1e-6),
            "width=0 must produce L == R, got L={} R={}",
            frame[0],
            frame[1],
        );
        // Mid of (0.5, -0.5) is 0, so the mono signal is zero.
        assert!(
            frame[0].abs() < 1e-6,
            "L=-R input + width=0 should be silence, got {}",
            frame[0]
        );
    }

    /// Width 1.0 is exactly identity for any stereo input.
    #[test]
    fn apply_width_stereo_one_is_identity() {
        let mut frame = [0.3f32, -0.7];
        apply_width_stereo(&mut frame, 1.0);
        assert!(approx_eq(frame[0], 0.3, 1e-6), "L drift at width=1: got {}", frame[0]);
        assert!(approx_eq(frame[1], -0.7, 1e-6), "R drift at width=1: got {}", frame[1]);
    }

    /// Width 1.5 amplifies the side component. Hand-computed expected values
    /// pinned: L=0.3, R=-0.7 → mid=-0.2, side=0.5; after 1.5× → side=0.75 →
    /// L=0.55, R=-0.95.
    #[test]
    fn apply_width_stereo_one_point_five_amplifies_side() {
        let mut frame = [0.3f32, -0.7];
        apply_width_stereo(&mut frame, 1.5);
        assert!(
            approx_eq(frame[0], 0.55, 1e-6),
            "L at width=1.5: expected 0.55, got {}",
            frame[0]
        );
        assert!(
            approx_eq(frame[1], -0.95, 1e-6),
            "R at width=1.5: expected -0.95, got {}",
            frame[1]
        );
    }

    /// Mid-only signal (L = R) must be unchanged regardless of width.
    /// Confirms width adjusts only the side component.
    #[test]
    fn apply_width_stereo_does_not_touch_pure_mid_signal() {
        for &w in &[0.0_f32, 0.5, 1.0, 1.5, 2.0] {
            let mut frame = [0.42f32, 0.42];
            apply_width_stereo(&mut frame, w);
            assert!(
                approx_eq(frame[0], 0.42, 1e-6) && approx_eq(frame[1], 0.42, 1e-6),
                "pure-mid signal changed under width={}, got L={} R={}",
                w,
                frame[0],
                frame[1],
            );
        }
    }

    /// Mono frame (length 1) is a no-op — guard prevents indexing past the
    /// frame's length.
    #[test]
    fn apply_width_stereo_no_op_on_mono_frame() {
        let mut frame = [0.7f32];
        apply_width_stereo(&mut frame, 0.0);
        assert!(approx_eq(frame[0], 0.7, 1e-6));
    }

    /// ChainCoeffs maps `Advanced.width = None` to a neutral side scale of 1.0
    /// — the slider being untouched must never alter the stereo image.
    #[test]
    fn chain_coeffs_default_width_is_neutral() {
        let settings = MasteringSettings {
            preset: Preset::Custom { id: "t".to_string() },
            intensity: 0.0,
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
        };
        let c = ChainCoeffs::from_settings(44_100, &settings);
        assert!(
            approx_eq(c.width_side_scale, 1.0, 1e-6),
            "untouched Advanced.width should map to 1.0, got {}",
            c.width_side_scale
        );
    }

    /// Out-of-range user width values are clamped, not honored — a 5.0 user
    /// value can't push the side past 2× (which is already the wide end of
    /// what mastering plugins typically expose).
    #[test]
    fn chain_coeffs_clamps_width_into_safe_range() {
        let base = MasteringSettings {
            preset: Preset::Custom { id: "t".to_string() },
            intensity: 0.0,
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
            advanced: AdvancedSettings {
                width: Some(5.0),
                ..AdvancedSettings::default()
            },
        };
        let c = ChainCoeffs::from_settings(44_100, &base);
        assert!(
            approx_eq(c.width_side_scale, 2.0, 1e-6),
            "user width=5.0 should clamp to 2.0, got {}",
            c.width_side_scale
        );

        let mut neg = base.clone();
        neg.advanced.width = Some(-1.0);
        let c_neg = ChainCoeffs::from_settings(44_100, &neg);
        assert!(
            approx_eq(c_neg.width_side_scale, 0.0, 1e-6),
            "user width=-1.0 should clamp to 0.0, got {}",
            c_neg.width_side_scale
        );
    }

    /// End-to-end: drive a stereo (L=sine, R=-sine) signal through the chain
    /// with width=0, neutral preset, neutral EQ, no saturation. After the
    /// limiter lookahead, the output should be silent because the M/S
    /// transform collapsed the pure-side signal to mono and mid is zero.
    #[test]
    fn process_frame_applies_width_inside_full_chain() {
        let settings = MasteringSettings {
            preset: Preset::Custom { id: "t".to_string() },
            intensity: 0.0,
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
            advanced: AdvancedSettings {
                width: Some(0.0),
                ..AdvancedSettings::default()
            },
        };
        let mut chain = MasteringChain::new(44_100, 2, &settings);
        let mut last = [0.0f32, 0.0];
        for n in 0..2_048 {
            // Pure-side signal: L = +sine, R = -sine.
            let s = 0.4 * (n as f32 * 2.0 * std::f32::consts::PI * 1000.0 / 44_100.0).sin();
            let mut frame = [s, -s];
            chain.process_frame_inplace(&mut frame);
            last = [frame[0], frame[1]];
        }
        // After the limiter's 3 ms lookahead (~132 frames) the chain's output
        // should reflect the M/S collapse: both channels at silence.
        assert!(
            last[0].abs() < 1e-3,
            "width=0 inside chain should silence pure-side signal, got L={}",
            last[0]
        );
        assert!(
            last[1].abs() < 1e-3,
            "width=0 inside chain should silence pure-side signal, got R={}",
            last[1]
        );
    }

    /// Companion: with width=1.0 the same pure-side signal must NOT be
    /// silenced — proves the silence in the prior test is from width=0, not
    /// some upstream bug.
    #[test]
    fn process_frame_with_neutral_width_preserves_side_signal() {
        let settings = MasteringSettings {
            preset: Preset::Custom { id: "t".to_string() },
            intensity: 0.0,
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
            advanced: AdvancedSettings {
                width: Some(1.0),
                ..AdvancedSettings::default()
            },
        };
        let mut chain = MasteringChain::new(44_100, 2, &settings);
        let mut peak_l = 0.0f32;
        let mut peak_r = 0.0f32;
        for n in 0..2_048 {
            let s = 0.4 * (n as f32 * 2.0 * std::f32::consts::PI * 1000.0 / 44_100.0).sin();
            let mut frame = [s, -s];
            chain.process_frame_inplace(&mut frame);
            if frame[0].abs() > peak_l {
                peak_l = frame[0].abs();
            }
            if frame[1].abs() > peak_r {
                peak_r = frame[1].abs();
            }
        }
        assert!(
            peak_l > 0.1 && peak_r > 0.1,
            "width=1 must pass the side signal through, got peak L={} R={}",
            peak_l,
            peak_r
        );
    }

    /// Phase 12.2 — warmth control. When `Advanced.warmth = None`, the chain's
    /// warmth biquad must be identity (b0 = 1.0, all other coeffs ~0) so the
    /// untouched-slider path is byte-equivalent to the pre-slice chain output.
    #[test]
    fn warmth_default_is_identity() {
        let settings = MasteringSettings {
            preset: Preset::Custom { id: "t".to_string() },
            intensity: 0.0,
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
        };
        let c = ChainCoeffs::from_settings(44_100, &settings);
        assert!(approx_eq(c.warmth.b0, 1.0, 1e-6), "warmth.b0 should be 1.0, got {}", c.warmth.b0);
        assert!(approx_eq(c.warmth.b1, 0.0, 1e-6), "warmth.b1 should be 0.0, got {}", c.warmth.b1);
        assert!(approx_eq(c.warmth.b2, 0.0, 1e-6), "warmth.b2 should be 0.0, got {}", c.warmth.b2);
        assert!(approx_eq(c.warmth.a1, 0.0, 1e-6), "warmth.a1 should be 0.0, got {}", c.warmth.a1);
        assert!(approx_eq(c.warmth.a2, 0.0, 1e-6), "warmth.a2 should be 0.0, got {}", c.warmth.a2);
    }

    /// Phase 12.2 — warmth control. Slider at 1.0 must lift the 300 Hz low
    /// frequencies by close to the design's max of +4 dB and leave the high
    /// frequencies near 0 dB. Pins both the magnitude AND the shelf shape.
    #[test]
    fn warmth_at_one_lifts_300hz_band() {
        let settings = MasteringSettings {
            preset: Preset::Custom { id: "t".to_string() },
            intensity: 0.0,
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
            advanced: AdvancedSettings {
                warmth: Some(1.0),
                ..AdvancedSettings::default()
            },
        };
        let c = ChainCoeffs::from_settings(44_100, &settings);

        let gain_low = biquad_magnitude_db_at(&c.warmth, 100.0, 44_100.0);
        assert!(
            gain_low > 3.0,
            "warmth=1.0 should give >+3 dB at 100 Hz (below shelf corner), got {} dB",
            gain_low
        );

        let gain_high = biquad_magnitude_db_at(&c.warmth, 5_000.0, 44_100.0);
        assert!(
            gain_high.abs() < 0.5,
            "warmth=1.0 should leave 5 kHz near 0 dB, got {} dB",
            gain_high
        );
    }

    /// Phase 12.2 — warmth control clamping. Out-of-range slider values (5.0,
    /// -1.0) must clamp into [0, 1] before mapping to dB, so a runaway value
    /// can't push the shelf past +4 dB or invert gain.
    #[test]
    fn chain_coeffs_clamps_warmth_into_range() {
        let make = |w: f32| MasteringSettings {
            preset: Preset::Custom { id: "t".to_string() },
            intensity: 0.0,
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
            advanced: AdvancedSettings {
                warmth: Some(w),
                ..AdvancedSettings::default()
            },
        };
        let c_high = ChainCoeffs::from_settings(44_100, &make(5.0));
        let c_max = ChainCoeffs::from_settings(44_100, &make(1.0));
        assert!(approx_eq(c_high.warmth.b0, c_max.warmth.b0, 1e-6),
            "warmth=5.0 should clamp to 1.0 (b0 mismatch: {} vs {})",
            c_high.warmth.b0, c_max.warmth.b0);

        let c_neg = ChainCoeffs::from_settings(44_100, &make(-1.0));
        let c_zero = ChainCoeffs::from_settings(44_100, &make(0.0));
        assert!(approx_eq(c_neg.warmth.b0, c_zero.warmth.b0, 1e-6),
            "warmth=-1.0 should clamp to 0.0 (b0 mismatch: {} vs {})",
            c_neg.warmth.b0, c_zero.warmth.b0);
    }

    /// Phase 12.2 — presence_air control. Default `None` must produce an
    /// identity biquad, matching the warmth default contract.
    #[test]
    fn presence_air_default_is_identity() {
        let settings = MasteringSettings {
            preset: Preset::Custom { id: "t".to_string() },
            intensity: 0.0,
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
        };
        let c = ChainCoeffs::from_settings(44_100, &settings);
        assert!(approx_eq(c.presence_air.b0, 1.0, 1e-6),
            "presence_air.b0 should be 1.0, got {}", c.presence_air.b0);
        assert!(approx_eq(c.presence_air.b1, 0.0, 1e-6));
        assert!(approx_eq(c.presence_air.b2, 0.0, 1e-6));
        assert!(approx_eq(c.presence_air.a1, 0.0, 1e-6));
        assert!(approx_eq(c.presence_air.a2, 0.0, 1e-6));
    }

    /// Phase 12.2 — presence_air control. Slider at 1.0 must lift the 10 kHz
    /// high frequencies by close to +4 dB and leave the low frequencies near
    /// 0 dB. Mirror-image of the warmth test.
    #[test]
    fn presence_air_at_one_lifts_10khz_band() {
        let settings = MasteringSettings {
            preset: Preset::Custom { id: "t".to_string() },
            intensity: 0.0,
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
            advanced: AdvancedSettings {
                presence_air: Some(1.0),
                ..AdvancedSettings::default()
            },
        };
        let c = ChainCoeffs::from_settings(44_100, &settings);

        let gain_high = biquad_magnitude_db_at(&c.presence_air, 18_000.0, 44_100.0);
        assert!(
            gain_high > 3.0,
            "presence_air=1.0 should give >+3 dB at 18 kHz (above shelf corner), got {} dB",
            gain_high
        );

        let gain_low = biquad_magnitude_db_at(&c.presence_air, 1_000.0, 44_100.0);
        assert!(
            gain_low.abs() < 0.5,
            "presence_air=1.0 should leave 1 kHz near 0 dB, got {} dB",
            gain_low
        );
    }

    // ====================================================================
    // Phase 12.2 — multiband compressor tests. Closed-form math where
    // possible; otherwise pin behavior by feeding known-amplitude steady
    // signals through `MasteringChain` and observing steady-state output.
    // ====================================================================

    fn default_master_settings() -> MasteringSettings {
        MasteringSettings {
            preset: Preset::Custom { id: "t".to_string() },
            intensity: 0.0,
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

    #[test]
    fn compression_density_default_is_identity() {
        let c = ChainCoeffs::from_settings(44_100, &default_master_settings());
        assert!(
            !c.compression_active,
            "default settings must set compression_active = false (got true)"
        );
    }

    #[test]
    fn lr4_crossover_sums_flat_at_unity() {
        // LR4 sums flat in magnitude (Linkwitz–Riley property) but the band
        // network has non-zero group delay, so sample-equality of L+M+H with
        // x is impossible — they're equal only as time-shifted signals. We
        // pin the magnitude property by RMS equality: the energy of the
        // summed bands matches the energy of the input within ~0.1 dB.
        let sr = 44_100.0f32;
        for &freq in &[60.0f32, 1_000.0, 8_000.0] {
            let mut state = LR4State::default();
            for n in 0..1024 {
                let x = (n as f32 * 2.0 * std::f32::consts::PI * freq / sr).sin();
                let _ = split_lr4_into_bands(x, &mut state);
            }
            let mut sum_in_sq = 0.0f64;
            let mut sum_out_sq = 0.0f64;
            let measure_len: usize = 4096;
            for n in 1024..(1024 + measure_len) {
                let x = (n as f32 * 2.0 * std::f32::consts::PI * freq / sr).sin();
                let (l, m, h) = split_lr4_into_bands(x, &mut state);
                sum_in_sq += (x as f64).powi(2);
                sum_out_sq += ((l + m + h) as f64).powi(2);
            }
            let rms_in = (sum_in_sq / measure_len as f64).sqrt();
            let rms_out = (sum_out_sq / measure_len as f64).sqrt();
            let ratio_db = 20.0 * (rms_out / rms_in.max(1e-9)).log10();
            assert!(
                ratio_db.abs() < 0.12,
                "LR4 summing flatness (RMS) violated at {} Hz: |L+M+H| / |x| = {:.3} dB (rms_in={}, rms_out={})",
                freq,
                ratio_db,
                rms_in,
                rms_out
            );
        }
    }

    #[test]
    fn compression_density_at_one_attenuates_loud_signal() {
        let sr = 44_100;
        let freq = 1_000.0f32;
        let amp = 0.8f32;
        let mut s0 = default_master_settings();
        s0.advanced.compression_density = Some(0.0);
        let mut s1 = default_master_settings();
        s1.advanced.compression_density = Some(1.0);
        let mut chain0 = MasteringChain::new(sr, 2, &s0);
        let mut chain1 = MasteringChain::new(sr, 2, &s1);
        let settle = (0.4 * sr as f32) as usize;
        let measure = (0.2 * sr as f32) as usize;
        let mut sum0 = 0.0f64;
        let mut sum1 = 0.0f64;
        for n in 0..(settle + measure) {
            let x = amp * (n as f32 * 2.0 * std::f32::consts::PI * freq / sr as f32).sin();
            let mut f0 = [x, x];
            let mut f1 = [x, x];
            chain0.process_frame_inplace(&mut f0);
            chain1.process_frame_inplace(&mut f1);
            if n >= settle {
                sum0 += (f0[0] as f64).powi(2);
                sum1 += (f1[0] as f64).powi(2);
            }
        }
        let rms0 = (sum0 / measure as f64).sqrt() as f32;
        let rms1 = (sum1 / measure as f64).sqrt() as f32;
        let delta_db = 20.0 * (rms1 / rms0.max(1e-9)).log10();
        assert!(
            delta_db <= -3.0,
            "density=1.0 should attenuate the loud mid-band sine by >=3 dB \
             vs density=0.0; got delta = {:.2} dB (rms0={}, rms1={})",
            delta_db,
            rms0,
            rms1
        );
    }

    #[test]
    fn compression_per_band_override_replaces_macro() {
        let mut s = default_master_settings();
        s.advanced.compression_density = Some(0.0);
        s.advanced.compression_mid_threshold_db = Some(-30.0);
        let c = ChainCoeffs::from_settings(44_100, &s);
        assert!(
            (c.comp_mid_threshold_db - (-30.0)).abs() < 1e-4,
            "mid threshold should be -30, got {}",
            c.comp_mid_threshold_db
        );
        assert!(
            c.comp_low_threshold_db.abs() < 1e-4,
            "low threshold should be macro (0 dBFS at density=0), got {}",
            c.comp_low_threshold_db
        );
        assert!(
            c.comp_high_threshold_db.abs() < 1e-4,
            "high threshold should be macro (0 dBFS at density=0), got {}",
            c.comp_high_threshold_db
        );
    }

    #[test]
    fn envelope_follower_attack_release_time_constants() {
        let sr = 44_100.0f32;
        let mut env = EnvelopeFollower::new(sr, 10.0, 100.0);
        let attack_samples = (sr * 0.010) as usize;
        let mut last = 0.0f32;
        for _ in 0..attack_samples {
            last = env.process(1.0);
        }
        assert!(
            last >= 0.63,
            "after 10 ms (attack tau) of step input, env should be >= 0.63 \
             (1 - 1/e); got {}",
            last
        );
        let release_samples = (sr * 0.100) as usize;
        for _ in 0..release_samples {
            last = env.process(0.0);
        }
        assert!(
            last <= 0.37,
            "after 100 ms (release tau) of zero input, env should be <= 0.37 \
             (1/e); got {}",
            last
        );
    }

    #[test]
    fn compression_linked_stereo_applies_same_gain_to_both_channels() {
        // RMS-based comparison: per-sample ratios blow up at sine zero
        // crossings, so we measure gain via energy ratios. In linked mode,
        // both channels see the same band-gain envelope (driven by the louder
        // channel) — the L:R output-vs-input dB ratios should match within
        // a small tolerance. In unlinked mode, the quiet channel barely
        // triggers reduction while the loud one is hammered — the two ratios
        // diverge.
        let sr = 44_100;
        let freq = 1_000.0f32;
        let mut s_linked = default_master_settings();
        s_linked.advanced.compression_density = Some(1.0);
        s_linked.advanced.compression_link_stereo = Some(true);
        let mut s_unlinked = s_linked.clone();
        s_unlinked.advanced.compression_link_stereo = Some(false);
        let mut linked = MasteringChain::new(sr, 2, &s_linked);
        let mut unlinked = MasteringChain::new(sr, 2, &s_unlinked);
        let settle = (0.4 * sr as f32) as usize;
        let measure = (0.2 * sr as f32) as usize;
        let mut sum_l_in_sq = 0.0f64;
        let mut sum_r_in_sq = 0.0f64;
        let mut sum_l_lk_sq = 0.0f64;
        let mut sum_r_lk_sq = 0.0f64;
        let mut sum_l_un_sq = 0.0f64;
        let mut sum_r_un_sq = 0.0f64;
        for n in 0..(settle + measure) {
            let phase = n as f32 * 2.0 * std::f32::consts::PI * freq / sr as f32;
            let l_in = 0.8 * phase.sin();
            let r_in = 0.05 * phase.sin();
            let mut f_l = [l_in, r_in];
            let mut f_u = [l_in, r_in];
            linked.process_frame_inplace(&mut f_l);
            unlinked.process_frame_inplace(&mut f_u);
            if n >= settle {
                sum_l_in_sq += (l_in as f64).powi(2);
                sum_r_in_sq += (r_in as f64).powi(2);
                sum_l_lk_sq += (f_l[0] as f64).powi(2);
                sum_r_lk_sq += (f_l[1] as f64).powi(2);
                sum_l_un_sq += (f_u[0] as f64).powi(2);
                sum_r_un_sq += (f_u[1] as f64).powi(2);
            }
        }
        let to_db = |out_sq: f64, in_sq: f64| -> f32 {
            (10.0 * (out_sq / in_sq.max(1e-30)).log10()) as f32
        };
        let lk_l_db = to_db(sum_l_lk_sq, sum_l_in_sq);
        let lk_r_db = to_db(sum_r_lk_sq, sum_r_in_sq);
        let un_l_db = to_db(sum_l_un_sq, sum_l_in_sq);
        let un_r_db = to_db(sum_r_un_sq, sum_r_in_sq);
        // Linked: L and R should see the same dB change (the loud-L envelope
        // drives both channels).
        assert!(
            (lk_l_db - lk_r_db).abs() < 1.0,
            "linked stereo should give matching gain to L and R; \
             L delta = {:.2} dB, R delta = {:.2} dB",
            lk_l_db,
            lk_r_db
        );
        // Unlinked: the loud L gets reduced; the quiet R sees almost no
        // reduction. Difference should be material (>= 3 dB).
        assert!(
            (un_l_db - un_r_db).abs() > 3.0,
            "unlinked stereo should diverge L vs R; L delta = {:.2} dB, \
             R delta = {:.2} dB (linked was L={:.2}, R={:.2})",
            un_l_db,
            un_r_db,
            lk_l_db,
            lk_r_db
        );
    }

    #[test]
    fn compression_makeup_gain_compensates_threshold_drop() {
        // Phase A4: at density=0.5 the preset_engagement is 1.0, so the
        // compressor lands on the preset's threshold/ratio identity. Custom
        // mirrors Universal: threshold = -16 dBFS, ratio = 1.8.
        // makeup = (-(-16)) * (1 - 1/1.8) / 2 = 16 * 0.4444 / 2 = 3.555 dB.
        let mut s = default_master_settings();
        s.advanced.compression_density = Some(0.5);
        let c = ChainCoeffs::from_settings(44_100, &s);
        let expected_makeup = 16.0 * (1.0 - 1.0 / 1.8) / 2.0;
        assert!(
            (c.comp_mid_makeup_db - expected_makeup).abs() < 0.1,
            "mid makeup_db at density=0.5 with Custom (-16/1.8) should be {expected_makeup:.3} dB, got {}",
            c.comp_mid_makeup_db
        );
        let sr = 44_100;
        let freq = 1_000.0f32;
        let amp = 0.1f32;
        let mut chain = MasteringChain::new(sr, 2, &s);
        let settle = (0.4 * sr as f32) as usize;
        let measure = (0.2 * sr as f32) as usize;
        let mut sum_in = 0.0f64;
        let mut sum_out = 0.0f64;
        for n in 0..(settle + measure) {
            let x = amp * (n as f32 * 2.0 * std::f32::consts::PI * freq / sr as f32).sin();
            let mut f = [x, x];
            chain.process_frame_inplace(&mut f);
            if n >= settle {
                sum_in += (x as f64).powi(2);
                sum_out += (f[0] as f64).powi(2);
            }
        }
        let in_db = 10.0 * (sum_in / measure as f64).log10();
        let out_db = 10.0 * (sum_out / measure as f64).log10();
        let delta_db = (out_db - in_db) as f32;
        assert!(
            (delta_db - expected_makeup).abs() < 1.5,
            "sub-threshold sine should see ~+{expected_makeup:.2} dB makeup at density=0.5; got delta = {:.2} dB",
            delta_db
        );
    }

    #[test]
    fn compression_clamps_density_into_range() {
        // Phase A4: density clamps to [0,1]. With Custom mirror of
        // Universal (threshold=-16 dBFS), density=5.0 → clamped to 1.0 →
        // preset_engagement=1, overdrive=1 → effective threshold =
        // -16 + (-3) = -19 dBFS.
        let mut s_high = default_master_settings();
        s_high.advanced.compression_density = Some(5.0);
        let c_high = ChainCoeffs::from_settings(44_100, &s_high);
        assert!(
            (c_high.comp_mid_threshold_db - (-19.0)).abs() < 1e-3,
            "density=5.0 should clamp to 1.0 (Custom threshold = -16 dBFS + overdrive -3 = -19 dBFS); got {}",
            c_high.comp_mid_threshold_db
        );
        // density=-1.0 → clamped to 0 → preset_engagement=0 → effective
        // threshold = 0 dBFS regardless of preset.
        let mut s_neg = default_master_settings();
        s_neg.advanced.compression_density = Some(-1.0);
        let c_neg = ChainCoeffs::from_settings(44_100, &s_neg);
        assert!(
            c_neg.comp_mid_threshold_db.abs() < 1e-3,
            "density=-1.0 should clamp to 0.0 (threshold = 0 dBFS); got {}",
            c_neg.comp_mid_threshold_db
        );
    }

    /// Feed `seconds` of a sine at `amp_dbfs` (peak) into the integrator and
    /// return the final integrated LUFS reading. Used by the tests below to
    /// build representative listening-pass signals.
    fn feed_sine(meter: &mut IntegratedLufs, sample_rate: u32, seconds: f32, freq_hz: f32, amp_dbfs: f32) {
        let amp_lin = 10.0_f32.powf(amp_dbfs / 20.0);
        let total = (sample_rate as f32 * seconds) as u32;
        let omega = 2.0 * std::f32::consts::PI * freq_hz / sample_rate as f32;
        for n in 0..total {
            let s = amp_lin * (omega * n as f32).sin();
            meter.process_frame(s, s);
        }
    }

    fn feed_silence(meter: &mut IntegratedLufs, sample_rate: u32, seconds: f32) {
        let total = (sample_rate as f32 * seconds) as u32;
        for _ in 0..total {
            meter.process_frame(0.0, 0.0);
        }
    }

    /// Sanity check: a steady 1 kHz sine at -23 dBFS peak (≈ -26 dBFS RMS for a
    /// pure sine, ≈ -23 LUFS after K-weighting + sum-of-channels — the K-shelf
    /// adds ~+2 dB at 1 kHz and stereo summation adds +3 dB to the channel-mean
    /// energy) should integrate to roughly -22 LUFS. We allow ±2 LU because the
    /// K-weighting magnitude at 1 kHz depends on the exact RBJ shelf shape.
    #[test]
    fn integrated_lufs_steady_sine_lands_near_expected() {
        let sr = 48_000;
        let mut meter = IntegratedLufs::new(sr);
        feed_sine(&mut meter, sr, 3.0, 1000.0, -23.0);
        let integrated = meter.lufs();
        assert!(
            integrated > -26.0 && integrated < -18.0,
            "1 kHz -23 dBFS sine should integrate to ~ -22 LUFS, got {integrated}"
        );
    }

    /// Absolute gate: a signal where half the time is well below -70 LUFS
    /// (silence) and half is at a normal listening level should integrate near
    /// the loud-half value, not midway between loud and silent. Validates that
    /// the absolute gate is dropping silent blocks per BS.1770-4.
    #[test]
    fn integrated_lufs_absolute_gate_drops_silence() {
        let sr = 48_000;
        let mut meter = IntegratedLufs::new(sr);
        // Sandwich: 2 s sine, 2 s silence, 2 s sine. Without gating, the silence
        // would pull the mean down by ~3 dB (half the energy gone). With the
        // absolute gate at -70 LUFS, the silence blocks fall out and the
        // integrated value tracks the sine sections.
        feed_sine(&mut meter, sr, 2.0, 1000.0, -20.0);
        feed_silence(&mut meter, sr, 2.0);
        feed_sine(&mut meter, sr, 2.0, 1000.0, -20.0);
        let integrated = meter.lufs();
        // Sine-only baseline for comparison.
        let mut baseline = IntegratedLufs::new(sr);
        feed_sine(&mut baseline, sr, 3.0, 1000.0, -20.0);
        let baseline_lufs = baseline.lufs();
        assert!(
            (integrated - baseline_lufs).abs() < 1.5,
            "absolute gate should reject silence — sandwich integrated = {integrated}, sine-only baseline = {baseline_lufs}"
        );
    }

    /// Relative gate: the BS.1770-4 algorithm drops blocks more than 10 LU
    /// below the absolute-gated mean. So a clip with mostly loud material and a
    /// short -20 LU dip should land near the loud-section LUFS, not pulled
    /// down by the dip.
    #[test]
    fn integrated_lufs_relative_gate_drops_quiet_tail() {
        let sr = 48_000;
        let mut meter = IntegratedLufs::new(sr);
        // 4 seconds at -18 dBFS, then 1 second at -55 dBFS (≈ -55 LUFS — passes
        // the absolute gate but should be well below the relative gate).
        feed_sine(&mut meter, sr, 4.0, 1000.0, -18.0);
        feed_sine(&mut meter, sr, 1.0, 1000.0, -55.0);
        let integrated_with_tail = meter.lufs();

        let mut baseline = IntegratedLufs::new(sr);
        feed_sine(&mut baseline, sr, 4.0, 1000.0, -18.0);
        let baseline_loud = baseline.lufs();

        assert!(
            (integrated_with_tail - baseline_loud).abs() < 1.0,
            "relative gate should drop -55 LUFS tail; got integrated = {integrated_with_tail}, baseline (no tail) = {baseline_loud}"
        );
    }

    /// Until the first 400 ms block has filled, the integrated reading should
    /// stay at the -120.0 sentinel (UI uses this to suppress the readout).
    #[test]
    fn integrated_lufs_returns_sentinel_until_first_block() {
        let sr = 48_000;
        let mut meter = IntegratedLufs::new(sr);
        // 100 ms — less than one block, so no block has been emitted yet.
        feed_sine(&mut meter, sr, 0.1, 1000.0, -20.0);
        assert!(
            meter.lufs() <= -119.0,
            "should return -120.0 sentinel before first block fills, got {}",
            meter.lufs()
        );
    }

    /// Reset clears all integrator state — feeding the same signal post-reset
    /// should yield the same reading as a fresh instance.
    #[test]
    fn integrated_lufs_reset_zeroes_state() {
        let sr = 48_000;
        let mut meter = IntegratedLufs::new(sr);
        feed_sine(&mut meter, sr, 2.0, 1000.0, -10.0);
        let loud_lufs = meter.lufs();
        meter.reset();
        assert!(
            meter.lufs() <= -119.0,
            "reset must return to sentinel, got {}",
            meter.lufs()
        );
        feed_sine(&mut meter, sr, 2.0, 1000.0, -23.0);
        let after_reset = meter.lufs();
        assert!(
            (after_reset - loud_lufs) < -8.0,
            "post-reset reading should reflect the new (quieter) material; got after_reset = {after_reset}, prior loud = {loud_lufs}"
        );
    }

    // ========================================================================
    // Phase A1: BS.1770-4 K-weighting + momentary LUFS conformance tests.
    // ========================================================================

    /// At 48 kHz the K-weighting Stage 1 (high-shelf) coefficients must match
    /// the published ITU-R BS.1770-4 Annex 1 reference within 1e-6. f32
    /// epsilon at this magnitude is ~1.2e-7, so the f64-computed coefficients
    /// narrowed to f32 storage easily land inside the tolerance.
    #[test]
    fn k_weighting_pre_matches_bs1770_reference_at_48k() {
        let c = BiquadCoeffs::k_weighting_pre(48_000);
        assert!(
            (c.b0 - 1.535_124_85_f32).abs() < 1.0e-6,
            "b0: expected ~1.53512486, got {}",
            c.b0
        );
        assert!(
            (c.b1 - (-2.691_696_2_f32)).abs() < 1.0e-6,
            "b1: expected ~-2.69169619, got {}",
            c.b1
        );
        assert!(
            (c.b2 - 1.198_392_8_f32).abs() < 1.0e-6,
            "b2: expected ~1.19839281, got {}",
            c.b2
        );
        assert!(
            (c.a1 - (-1.690_659_3_f32)).abs() < 1.0e-6,
            "a1: expected ~-1.69065929, got {}",
            c.a1
        );
        assert!(
            (c.a2 - 0.732_480_8_f32).abs() < 1.0e-6,
            "a2: expected ~0.73248077, got {}",
            c.a2
        );
    }

    /// At 48 kHz the K-weighting Stage 2 (RLB high-pass) coefficients must
    /// match the published ITU-R BS.1770-4 Annex 1 reference within 1e-6.
    /// Note that the b values are kept un-normalized per the standard
    /// (b0 = 1, b1 = -2, b2 = 1), which adds a ~+0.04 dB asymptotic gain at
    /// Nyquist relative to a naïvely-normalized HP biquad.
    #[test]
    fn k_weighting_rlb_matches_bs1770_reference_at_48k() {
        let c = BiquadCoeffs::k_weighting_rlb(48_000);
        assert!((c.b0 - 1.0_f32).abs() < 1.0e-6, "b0 expected 1.0, got {}", c.b0);
        assert!((c.b1 - (-2.0_f32)).abs() < 1.0e-6, "b1 expected -2.0, got {}", c.b1);
        assert!((c.b2 - 1.0_f32).abs() < 1.0e-6, "b2 expected 1.0, got {}", c.b2);
        assert!(
            (c.a1 - (-1.990_047_5_f32)).abs() < 1.0e-6,
            "a1: expected ~-1.99004745, got {}",
            c.a1
        );
        assert!(
            (c.a2 - 0.990_072_3_f32).abs() < 1.0e-6,
            "a2: expected ~0.99007225, got {}",
            c.a2
        );
    }

    /// Phase B+ Step 8.7 — K-weighting transfer-function lock.
    ///
    /// Asserts the BS.1770-4 K-weighting cascade (pre-filter + RLB
    /// high-pass, applied in series so their dB magnitudes add) holds
    /// at canonical test frequencies. A future filter-form change can't
    /// silently shift the curve without firing this test.
    ///
    /// Targets here are the *measured* response of our existing
    /// BS.1770-4-compliant coefficients at 48 kHz (verified bit-exact
    /// against the published reference by
    /// `k_weighting_pre_matches_bs1770_reference_at_48k` and the RLB
    /// sibling test). The plan v1's "0 dB at 1 kHz" pitched targets
    /// conflated the *filter* response with the *LUFS reading* (which
    /// subtracts the standard -0.691 dB gating offset); the actual
    /// filter cascade at 1 kHz inherently sits near +0.7 dB, and the
    /// -0.691 offset is applied later in the LUFS formula, not in the
    /// filter. So these targets lock our true response.
    #[test]
    fn k_weighting_cascade_matches_bs1770_reference_at_48k() {
        let sr = 48_000_f32;
        let pre = BiquadCoeffs::k_weighting_pre(48_000);
        let rlb = BiquadCoeffs::k_weighting_rlb(48_000);
        // (frequency Hz, target cascade gain dB) — values measured on
        // our coefficients at 48 kHz; tolerance covers small f32 rounding.
        let cases: &[(f32, f32)] = &[
            (20.0, -13.28),
            (60.0, -2.90),
            (100.0, -1.13),
            (1_000.0, 0.70),
            (2_000.0, 3.07),
            (6_000.0, 4.03),
            (10_000.0, 4.04),
        ];
        for &(f, expected_db) in cases {
            let pre_db = biquad_magnitude_db_at(&pre, f, sr);
            let rlb_db = biquad_magnitude_db_at(&rlb, f, sr);
            let cascade_db = pre_db + rlb_db;
            let delta = cascade_db - expected_db;
            assert!(
                delta.abs() < 0.3,
                "K-weight cascade @{f:.0} Hz: expected {expected_db:+.2} dB ±0.3, got {cascade_db:+.3} dB (pre={pre_db:+.3}, rlb={rlb_db:+.3}, delta={delta:+.3})",
            );
        }
    }

    /// Momentary meter must hold the silence sentinel until the rectangular
    /// 400 ms window has filled. Feed 200 ms of -20 dBFS audio (well above
    /// silence floor); meter should still report -120 because the ring isn't
    /// yet wrapped.
    #[test]
    fn momentary_lufs_returns_sentinel_before_window_fills() {
        let sr = 48_000;
        let mut meter = MomentaryLufs::new(sr);
        let n = (sr as f32 * 0.2) as u32;
        let amp = 0.1_f32; // -20 dBFS peak
        let omega = 2.0 * std::f32::consts::PI * 1000.0 / sr as f32;
        for i in 0..n {
            let s = amp * (omega * i as f32).sin();
            meter.process_frame(s, s);
        }
        assert!(
            meter.lufs() <= -119.0,
            "should still be at sentinel after 200 ms, got {}",
            meter.lufs()
        );
    }

    /// Pink noise at -23 dBFS RMS routed to a single channel must read
    /// -23 LUFS ± 0.5 LU once the 400 ms window has filled. This is the
    /// BS.1770 calibration anchor — the entire K-weighting + LUFS-offset
    /// derivation is constructed so that this signal lands at exactly
    /// -23 LUFS for an ideal pink generator.
    ///
    /// We use Paul Kellet's IIR pink approximation, which has ~±0.2 dB
    /// spectral deviation from an ideal 1/f curve across the audio band;
    /// the K-weighting net gain on Kellet-pink is therefore not exactly
    /// the calibrated +0.691 dB, hence the ±0.5 LU window.
    #[test]
    fn momentary_lufs_pink_noise_at_minus_23_dbfs_reads_minus_23_within_half_lu() {
        let sr = 48_000_u32;
        let n_samples = (sr as f32 * 1.0) as usize; // 1 s — well past the 400 ms window
        // Deterministic LCG → Paul Kellet pinking IIR.
        let mut state: u32 = 0xCAFE_BABE;
        let mut b0p = 0.0_f32;
        let mut b1p = 0.0_f32;
        let mut b2p = 0.0_f32;
        let mut b3p = 0.0_f32;
        let mut b4p = 0.0_f32;
        let mut b5p = 0.0_f32;
        let mut b6p;
        let mut pink = Vec::with_capacity(n_samples);
        for _ in 0..n_samples {
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
            let w = ((state >> 16) & 0x7FFF) as f32 / 32768.0 - 0.5; // ~±0.5 uniform
            b0p = 0.99886 * b0p + w * 0.0555179;
            b1p = 0.99332 * b1p + w * 0.0750759;
            b2p = 0.96900 * b2p + w * 0.1538520;
            b3p = 0.86650 * b3p + w * 0.3104856;
            b4p = 0.55000 * b4p + w * 0.5329522;
            b5p = -0.7616 * b5p - w * 0.0168980;
            let p = b0p + b1p + b2p + b3p + b4p + b5p + w * 0.5362;
            b6p = w * 0.115926;
            // b6p is used as part of the next iteration's output but Kellet's
            // canonical form folds it into the current p; we approximate by
            // adding the current `b6p` carry. Mirror the standard.
            pink.push(p + b6p);
        }
        // Calibrate pink to -23 dBFS RMS.
        let measured_rms: f32 = (pink.iter().map(|&x| x * x).sum::<f32>()
            / n_samples as f32)
            .sqrt();
        let target_rms = 10.0_f32.powf(-23.0 / 20.0);
        let scale = target_rms / measured_rms;
        // Feed to the LEFT channel only so the BS.1770 sum-of-channels
        // produces the mono-pink anchor reading (-23 LUFS); routing the
        // same signal to both channels would add +3 LU per the standard.
        let mut meter = MomentaryLufs::new(sr);
        for &p in &pink {
            meter.process_frame(p * scale, 0.0);
        }
        let reading = meter.lufs();
        assert!(
            (reading - (-23.0)).abs() < 0.5,
            "pink at -23 dBFS RMS (L-channel only) should read -23 LUFS ± 0.5 LU, got {reading}"
        );
    }

    // ========================================================================
    // Phase A2: low-mid band frequency response sanity check.
    // ========================================================================

    /// New 400 Hz Q=0.9 peaking biquad should produce ~+6 dB at 400 Hz when
    /// configured at +6 dB gain, and ~0 dB at 100 Hz and 1500 Hz (well below
    /// and above the band centre). Verifies the band lives in the mud zone
    /// without bleeding into the existing low-shelf (200 Hz) or peaking-mid
    /// (1500 Hz) bands.
    #[test]
    fn low_mid_band_centred_at_400hz_with_q_point_9() {
        let sr = 48_000.0_f32;
        let coeffs = BiquadCoeffs::peaking(sr, 400.0, 0.9, 6.0);
        let at_400 = biquad_magnitude_db_at(&coeffs, 400.0, sr);
        let at_100 = biquad_magnitude_db_at(&coeffs, 100.0, sr);
        let at_1500 = biquad_magnitude_db_at(&coeffs, 1500.0, sr);
        assert!(
            (at_400 - 6.0).abs() < 0.3,
            "400 Hz @ +6 dB gain: expected ~+6 dB, got {:.3}",
            at_400
        );
        assert!(
            at_100.abs() < 1.5,
            "100 Hz (well below band): expected ~0 dB, got {:.3}",
            at_100
        );
        assert!(
            at_1500.abs() < 1.5,
            "1500 Hz (above band): expected ~0 dB, got {:.3}",
            at_1500
        );
    }

    // ========================================================================
    // Volume Match attenuation formula.
    // ========================================================================

    /// Phase A4 hotfix-2: VM attenuation is computed from the chain's
    /// deterministic gain stages (input gain + average compressor makeup +
    /// saturation correction + user output trim), not from
    /// `effective_target_lufs()`. Custom (default_master_settings) gives
    /// the simplest math: input_gain = 1.5 (Custom baseline_gain_push) ×
    /// preset_scale 0.4 at intensity 0.0 = 0.6 dB; makeup = 0 (density
    /// default = 0 for Custom, no compression engaged); saturation = 0;
    /// output = 0. chain_push = 0.6, attenuation ≈ -0.6 dB → gain ≈ 0.933.
    #[test]
    fn volume_match_attenuates_by_estimated_chain_push() {
        let mut s = default_master_settings();
        s.volume_match = true;
        let c = ChainCoeffs::from_settings(48_000, &s);
        // Custom + intensity 0.0 + density default(0) → only the gain push
        // contributes. preset_scale = 0.4 + 1.2 * 0.0 = 0.4.
        let expected_push_db = 1.5 * 0.4; // 0.6 dB
        let expected = 10.0_f32.powf(-expected_push_db / 20.0);
        assert!(
            (c.volume_match_gain_lin - expected).abs() < 1e-3,
            "VM should attenuate by estimated chain push = -{:.2} dB; got {} (expected {})",
            expected_push_db,
            c.volume_match_gain_lin,
            expected
        );
    }

    /// VM attenuation grows when compression engages: density=0.5 on a
    /// non-Custom preset adds the preset's compressor makeup gain to the
    /// estimated chain push. The chain output is louder, so VM must
    /// attenuate more to land at source level.
    #[test]
    fn volume_match_includes_compressor_makeup() {
        let mut s = default_master_settings();
        s.preset = Preset::Loud; // threshold -23, ratio 3.5 → biggest makeup
        s.intensity = 0.5;
        s.volume_match = true;
        // density=0 → bypass → no makeup
        s.advanced.compression_density = Some(0.0);
        let c_no_comp = ChainCoeffs::from_settings(48_000, &s);
        // density=0.5 → full preset compressor → full makeup
        s.advanced.compression_density = Some(0.5);
        let c_full_comp = ChainCoeffs::from_settings(48_000, &s);
        assert!(
            c_full_comp.volume_match_gain_lin < c_no_comp.volume_match_gain_lin,
            "VM with compressor engaged should attenuate MORE than VM without; \
             no_comp gain = {}, full_comp gain = {}",
            c_no_comp.volume_match_gain_lin,
            c_full_comp.volume_match_gain_lin
        );
    }

    /// VM attenuation is independent of `source_lufs_integrated`
    /// WHEN THE CAP DOESN'T FIRE. The raw chain-push estimate is
    /// purely a function of the chain's own gain stages (input gain,
    /// makeup, saturation, output trim). For the test's specific
    /// scenario — default_master_settings + intensity 0 — raw_push
    /// is small (~0.6 dB) and stays well below the limiter-bounded
    /// cap, so the cap doesn't kick in and source_lufs doesn't
    /// influence the result.
    ///
    /// **Do NOT use this test as evidence that VM is globally
    /// source-LUFS-independent.** It isn't. The cap (added after
    /// Dan's "VM lands 11 dB below source on aggressive settings"
    /// repro) DOES consult source_lufs_integrated; see the
    /// `volume_match_caps_attenuation_at_limiter_bound` test for
    /// the dependent case. This test gates the un-capped path
    /// specifically.
    #[test]
    fn volume_match_uncapped_estimate_independent_of_source_lufs() {
        let mut a = default_master_settings();
        a.volume_match = true;
        a.source_lufs_integrated = Some(-10.0);
        let mut b = a.clone();
        b.source_lufs_integrated = Some(-25.0);
        let ca = ChainCoeffs::from_settings(48_000, &a);
        let cb = ChainCoeffs::from_settings(48_000, &b);
        assert!(
            (ca.volume_match_gain_lin - cb.volume_match_gain_lin).abs() < 1e-6,
            "uncapped VM estimate must be source-LUFS-independent on \
             default_master_settings + intensity 0 (raw_push small \
             enough that the cap never fires); got a={}, b={}",
            ca.volume_match_gain_lin,
            cb.volume_match_gain_lin
        );
    }

    /// VM never amplifies: when the chain's deterministic gain stages
    /// sum NEGATIVE (e.g., user has dialed a negative output trim that
    /// outweighs the preset push), the attenuation would mathematically
    /// be positive (gain > 1.0). Clamp to 0 dB so VM never accidentally
    /// pushes the signal into the limiter ceiling.
    #[test]
    fn volume_match_never_amplifies() {
        let mut s = default_master_settings();
        s.volume_match = true;
        s.input_gain_db = -6.0;  // negative input
        s.output_gain_db = -6.0; // negative output
        // chain_push ≈ -6 (input) + 0 (no comp on Custom) + 0 + -6 (output) = -12
        // → attenuation = -(-12) = +12, clamped to 0 → gain_lin = 1.0.
        let c = ChainCoeffs::from_settings(48_000, &s);
        assert!(
            (c.volume_match_gain_lin - 1.0).abs() < 1e-4,
            "VM should clamp negative chain-push to 0 dB attenuation; got {}",
            c.volume_match_gain_lin
        );
    }

    /// VM off (default) leaves the gain at 1.0 regardless of source/target.
    #[test]
    fn volume_match_off_is_unity_gain() {
        let mut s = default_master_settings();
        s.volume_match = false;
        s.source_lufs_integrated = Some(-19.0);
        let c = ChainCoeffs::from_settings(48_000, &s);
        assert!((c.volume_match_gain_lin - 1.0).abs() < 1e-6);
    }

    /// Regression for the "VM lands 11 dB below source on aggressive
    /// settings" bug Dan reported on Tape preset at Intensity 100% with
    /// +13 dB input gain and heavy EQ. The raw chain-push estimate
    /// (input gain + comp makeup + saturation) would push 18-22 dB, but
    /// the limiter actually caps post-chain LUFS at roughly
    /// (ceiling - typical_crest). VM should attenuate by the REAL push
    /// (~6 dB for source -13 LUFS / ceiling -1 dBTP), not the
    /// over-estimated raw push.
    ///
    /// Asserts the effective attenuation stays bounded by what the
    /// limiter could plausibly let through, so post-VM playback lands
    /// near source LUFS rather than 10+ dB below it.
    #[test]
    fn volume_match_caps_attenuation_at_limiter_bound() {
        let mut s = default_master_settings();
        s.preset = Preset::Tape;
        s.intensity = 1.0;
        s.input_gain_db = 13.0;
        s.volume_match = true;
        s.source_lufs_integrated = Some(-13.0);
        // Default ceiling -1.0 dBTP (Custom profile + None advanced).
        // max_real_push = ceiling - typical_crest - source
        //               = -1 - 6 - (-13) = 6 dB
        // So VM should attenuate AT MOST 6 dB, not the raw 18-22 dB
        // the chain-stage sum would produce.
        let c = ChainCoeffs::from_settings(48_000, &s);
        let attenuation_db = 20.0 * c.volume_match_gain_lin.log10();
        assert!(
            attenuation_db >= -8.0,
            "VM cap should prevent over-attenuation on extreme settings — \
             max real push given (ceiling -1, source -13) is 6 dB, plus a \
             little numerical slack; got {:.2} dB attenuation (gain = {})",
            attenuation_db,
            c.volume_match_gain_lin
        );
    }

    /// VM cap requires `source_lufs_integrated` to compute the
    /// limiter-bound max push. When source LUFS isn't injected (rare:
    /// the playback driver populates it from the current track's
    /// analysis), VM falls back to the raw chain-push estimate. Verifies
    /// the same extreme settings produce the OLD (unsafely-large)
    /// attenuation when source LUFS is None — proves the cap fires
    /// only when the data needed to compute it is present.
    #[test]
    fn volume_match_falls_back_to_raw_push_when_source_lufs_missing() {
        let mut s = default_master_settings();
        s.preset = Preset::Tape;
        s.intensity = 1.0;
        s.input_gain_db = 13.0;
        s.volume_match = true;
        s.source_lufs_integrated = None; // <- explicit
        let c = ChainCoeffs::from_settings(48_000, &s);
        let attenuation_db = 20.0 * c.volume_match_gain_lin.log10();
        // Raw push at these settings is well over 10 dB — no cap means
        // VM applies it. Asserts the fallback branch is wired.
        assert!(
            attenuation_db <= -10.0,
            "Without source_lufs_integrated the cap can't fire and VM \
             should apply the raw chain-push estimate (>= 10 dB on these \
             aggressive settings); got {:.2} dB",
            attenuation_db
        );
    }

    // ========================================================================
    // Phase A3: DeliveryProfile shadowing.
    // ========================================================================

    /// When the user picks a non-Custom profile, the chain's effective
    /// ceiling and effective target LUFS must come from the profile,
    /// not from the user's explicit advanced fields. This verifies the
    /// shadow flows all the way to ChainCoeffs.
    #[test]
    fn delivery_profile_shadows_ceiling_in_chain() {
        let mut s = default_master_settings();
        // User set ceiling to -3.0 explicitly, but BroadcastUs profile says
        // -2.0 — the profile should win since it's non-Custom.
        s.advanced.ceiling_dbtp = Some(-3.0);
        s.delivery_profile = DeliveryProfile::BroadcastUs;
        let c = ChainCoeffs::from_settings(48_000, &s);
        let expected = 10.0_f32.powf(-2.0 / 20.0);
        assert!(
            (c.ceiling_lin - expected).abs() < 1.0e-4,
            "BroadcastUs profile should shadow ceiling to -2.0 dBTP; got ceiling_lin = {} (expected {})",
            c.ceiling_lin,
            expected
        );
    }

    /// Custom profile must NOT shadow — the user's explicit advanced
    /// fields pass through unchanged.
    #[test]
    fn delivery_profile_custom_preserves_user_ceiling() {
        let mut s = default_master_settings();
        s.advanced.ceiling_dbtp = Some(-3.0);
        s.delivery_profile = DeliveryProfile::Custom;
        let c = ChainCoeffs::from_settings(48_000, &s);
        let expected = 10.0_f32.powf(-3.0 / 20.0);
        assert!(
            (c.ceiling_lin - expected).abs() < 1.0e-4,
            "Custom profile should NOT shadow; user's -3.0 dBTP must pass through; got ceiling_lin = {} (expected {})",
            c.ceiling_lin,
            expected
        );
    }

    /// effective_target_lufs returns the profile's value when non-Custom
    /// even if the user has set lufs_offset_db explicitly.
    #[test]
    fn delivery_profile_target_lufs_shadows_user_value() {
        let mut s = default_master_settings();
        s.advanced.lufs_offset_db = Some(-9.0);
        s.delivery_profile = DeliveryProfile::AppleMusic;
        assert_eq!(s.effective_target_lufs(), Some(-16.0));
    }

    /// effective_bit_depth returns the profile's value when non-Custom.
    #[test]
    fn delivery_profile_bit_depth_shadow() {
        let mut s = default_master_settings();
        s.advanced.bit_depth = Some(24);
        s.delivery_profile = DeliveryProfile::Cd;
        assert_eq!(s.effective_bit_depth(), 16);
    }

    /// Serde round-trip: a DeliveryProfile-bearing MasteringSettings
    /// serializes and deserializes back to the same value.
    #[test]
    fn delivery_profile_serde_round_trip() {
        let mut s = default_master_settings();
        s.delivery_profile = DeliveryProfile::VinylPremaster;
        let json = serde_json::to_string(&s).expect("serialize");
        let parsed: MasteringSettings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.delivery_profile, DeliveryProfile::VinylPremaster);
    }

    // ========================================================================
    // Phase B Step 1: AlbumPlan serde round-trip.
    // ========================================================================

    /// AlbumPlan with 4 tracks + 3 transitions (Direct, Gap 1.5 s, Direct)
    /// must serialize and deserialize back to the same shape.
    #[test]
    fn album_plan_serde_round_trip() {
        let plan = AlbumPlan {
            title: "Test Album".to_string(),
            arc: AlbumArc::Preset {
                preset: AlbumArcKind::Cinematic,
            },
            tracks: vec![
                AlbumTrackEntry {
                    track_id: TrackId("t1".to_string()),
                    position: 1,
                    role: TrackRole::Opener,
                    role_locked: false,
                    arc_lufs_offset_db: -2.1,
                    intensity_scale: 1.0,
                    album_character: None,
                },
                AlbumTrackEntry {
                    track_id: TrackId("t2".to_string()),
                    position: 2,
                    role: TrackRole::AlbumTrack,
                    role_locked: true,
                    arc_lufs_offset_db: 0.0,
                    intensity_scale: 0.95,
                    album_character: None,
                },
                AlbumTrackEntry {
                    track_id: TrackId("t3".to_string()),
                    position: 3,
                    role: TrackRole::Single,
                    role_locked: false,
                    arc_lufs_offset_db: 1.8,
                    intensity_scale: 1.1,
                    album_character: None,
                },
                AlbumTrackEntry {
                    track_id: TrackId("t4".to_string()),
                    position: 4,
                    role: TrackRole::Closer,
                    role_locked: false,
                    arc_lufs_offset_db: -1.4,
                    intensity_scale: 0.85,
                    album_character: None,
                },
            ],
            transitions: vec![
                TransitionSpec::direct(),
                TransitionSpec::gap(1.5),
                TransitionSpec::direct(),
            ],
            intensity: 1.0,
        };
        let json = serde_json::to_string(&plan).expect("serialize");
        let parsed: AlbumPlan = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.title, plan.title);
        assert_eq!(parsed.tracks.len(), 4);
        assert_eq!(parsed.tracks[0].role, TrackRole::Opener);
        assert_eq!(parsed.tracks[1].role_locked, true);
        assert_eq!(parsed.tracks[2].arc_lufs_offset_db, 1.8);
        assert_eq!(parsed.transitions.len(), 3);
        assert!(matches!(
            parsed.transitions[1].kind,
            TransitionKind::Gap
        ));
        assert_eq!(parsed.transitions[1].duration_seconds, 1.5);
        match parsed.arc {
            AlbumArc::Preset { preset } => assert_eq!(preset, AlbumArcKind::Cinematic),
            AlbumArc::Custom { .. } => panic!("expected Preset arc, got Custom"),
        }
    }

    /// MasteringSettings with an album plan round-trips through .ams.json.
    /// Verifies the `Option<AlbumPlan>` field doesn't break the existing
    /// settings shape.
    #[test]
    fn mastering_settings_with_album_plan_round_trip() {
        let mut s = default_master_settings();
        s.album = Some(AlbumPlan::default());
        let json = serde_json::to_string(&s).expect("serialize");
        let parsed: MasteringSettings =
            serde_json::from_str(&json).expect("deserialize");
        assert!(parsed.album.is_some());
        let album = parsed.album.unwrap();
        assert_eq!(album.intensity, 1.0);
        assert_eq!(album.tracks.len(), 0);
        match album.arc {
            AlbumArc::Preset { preset } => assert_eq!(preset, AlbumArcKind::Cinematic),
            AlbumArc::Custom { .. } => panic!("expected default Preset"),
        }
    }

    /// Older `.ams.json` projects with no `album` field load with `None`.
    #[test]
    fn mastering_settings_album_field_defaults_to_none() {
        let json = r#"{
            "preset": {"kind": "universal"},
            "intensity": 0.5,
            "eq_low_db": 0.0,
            "eq_mid_db": 0.0,
            "eq_high_db": 0.0,
            "volume_match": false,
            "advanced": {}
        }"#;
        let parsed: MasteringSettings =
            serde_json::from_str(json).expect("deserialize");
        assert!(parsed.album.is_none());
    }

    /// Backward compatibility: an older .ams.json that lacks the field
    /// loads with the StreamingUniversal default.
    #[test]
    fn delivery_profile_serde_default_for_older_files() {
        let json = r#"{
            "preset": {"kind": "universal"},
            "intensity": 0.5,
            "eq_low_db": 0.0,
            "eq_mid_db": 0.0,
            "eq_high_db": 0.0,
            "volume_match": false,
            "advanced": {}
        }"#;
        let parsed: MasteringSettings = serde_json::from_str(json).expect("deserialize");
        assert_eq!(parsed.delivery_profile, DeliveryProfile::StreamingUniversal);
        assert_eq!(parsed.eq_low_mid_db, 0.0);
    }

    /// Heavy presets (Punch / Loud / Oomph) must carry low-mid CUTS in the
    /// 1.25–2.2 dB range to clean up the mud zone. Reads the calibration
    /// values directly from the const table so a future numeric tweak
    /// breaks this test and forces a re-think.
    #[test]
    fn heavy_presets_cut_low_mid_band() {
        assert!(
            PRESET_PUNCH.low_mid_db <= -1.5 && PRESET_PUNCH.low_mid_db >= -2.5,
            "Punch should cut low-mid; got {}",
            PRESET_PUNCH.low_mid_db
        );
        assert!(
            PRESET_LOUD.low_mid_db <= -1.0 && PRESET_LOUD.low_mid_db >= -2.0,
            "Loud should cut low-mid; got {}",
            PRESET_LOUD.low_mid_db
        );
        // Phase A4 conservative-target retune deepened Oomph's mud-zone
        // scoop from -1.25 to -3.0 dB (analysis doc line 256). The bound
        // moved with it; if a future tune softens this past -2.5 the
        // preset stops cleaning up bass-forward material and the
        // distinctness contract test will fail anyway.
        assert!(
            PRESET_OOMPH.low_mid_db <= -2.5 && PRESET_OOMPH.low_mid_db >= -3.5,
            "Oomph should cut low-mid hard (target -3.0 dB); got {}",
            PRESET_OOMPH.low_mid_db
        );
    }

    /// Reset clears the ring; lufs() returns sentinel until the window
    /// re-fills.
    #[test]
    fn momentary_lufs_reset_returns_to_sentinel() {
        let sr = 48_000;
        let mut meter = MomentaryLufs::new(sr);
        let n = (sr as f32 * 0.5) as u32;
        let amp = 0.1_f32;
        let omega = 2.0 * std::f32::consts::PI * 1000.0 / sr as f32;
        for i in 0..n {
            let s = amp * (omega * i as f32).sin();
            meter.process_frame(s, s);
        }
        assert!(meter.lufs() > -119.0, "should have a reading before reset");
        meter.reset();
        assert!(
            meter.lufs() <= -119.0,
            "reset must return to sentinel, got {}",
            meter.lufs()
        );
    }
}

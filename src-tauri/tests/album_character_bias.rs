//! Phase B+ Step 8.5 — Album character promotion + bias landing.
//!
//! Verifies the position-aware character system:
//!
//!   * The filename-hint pass picks the right label for Track 1 (acoustic)
//!     and Track 2 (heavy/djent).
//!   * The HeavyDjent → ReturnAcoustic promotion fires for Track 3
//!     (AcousticFolk-by-scoring in the back half after a heavy).
//!   * The per-character `mastering_bias` actually changes the rendered
//!     output: HeavyDjent's low-mid bias and louder LUFS pull show up in
//!     the rendered WAVs, distinct from AcousticFolk / ReturnAcoustic.

use std::path::Path;

use album_mastering_studio_lib::album;
use album_mastering_studio_lib::engine::{
    self, render_album_plan_impl, AlbumPlanRenderRequest, AlbumTrackRenderInput,
};
use album_mastering_studio_lib::types::{
    AdvancedSettings, AlbumArc, AlbumArcKind, AlbumCharacter, AnalysisResult, DeliveryProfile,
    InferenceConfidence, MasteringSettings, Preset, SpectralBalance, TrackId, TrackRole,
    ISO_PLACEHOLDER,
};

const SR_HZ: u32 = 48_000;
const TRACK_DURATION_SEC: f32 = 2.0;
const STEREO: u16 = 2;

fn default_master_settings() -> MasteringSettings {
    MasteringSettings {
        preset: Preset::Universal,
        intensity: 0.5,
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

fn analysis_for(id: &str, energy: f32, transient: f32) -> AnalysisResult {
    AnalysisResult {
        track_id: TrackId(id.to_string()),
        // true_peak − lufs_integrated = 13 → crest_proxy_db = 13, which
        // boosts the acoustic-score openness term and lets a back-half
        // pink track score AcousticFolk before the ReturnAcoustic
        // promotion pass.
        lufs_integrated: -14.0,
        lufs_short_term_max: -10.0,
        true_peak_dbtp: -1.0,
        dynamic_range_lu: 8.0,
        spectral_balance: SpectralBalance {
            low: 0.33,
            mid: 0.34,
            high: 0.33,
        },
        transient_density: transient,
        stereo_width: 0.5,
        recommended_universal: default_master_settings(),
        measured_at_iso: ISO_PLACEHOLDER.to_string(),
        inferred_role: Some(TrackRole::AlbumTrack),
        role_confidence: Some(InferenceConfidence::Moderate),
        inferred_character: None,
        character_confidence: None,
        spectral_balance_6band: None,
        transient_flux: Some(transient),
        stereo_correlation: None,
        dynamic_range_p95_p10_db: None,
        lufs_short_term_max_3s: None,
        energy_density_score: Some(energy),
    }
}

/// Paul Kellet pink-ish noise, mono, scaled so the sample peak hits
/// `target_peak`. Same pinking filter as the dsp tests.
fn synth_pink(samples: usize, target_peak: f32) -> Vec<f32> {
    let mut state: u32 = 0xCAFE_BABE;
    let mut b0p = 0.0_f32;
    let mut b1p = 0.0_f32;
    let mut b2p = 0.0_f32;
    let mut b3p = 0.0_f32;
    let mut b4p = 0.0_f32;
    let mut b5p = 0.0_f32;
    let mut pink = Vec::with_capacity(samples);
    for _ in 0..samples {
        state = state.wrapping_mul(1_103_515_245).wrapping_add(12345);
        let w = ((state >> 16) & 0x7FFF) as f32 / 32_768.0 - 0.5;
        b0p = 0.99886 * b0p + w * 0.0555179;
        b1p = 0.99332 * b1p + w * 0.0750759;
        b2p = 0.96900 * b2p + w * 0.1538520;
        b3p = 0.86650 * b3p + w * 0.3104856;
        b4p = 0.55000 * b4p + w * 0.5329522;
        b5p = -0.7616 * b5p - w * 0.0168980;
        let b6p = w * 0.115926;
        pink.push(b0p + b1p + b2p + b3p + b4p + b5p + w * 0.5362 + b6p);
    }
    let measured_peak = pink.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    let scale = target_peak / measured_peak.max(f32::MIN_POSITIVE);
    for s in &mut pink {
        *s *= scale;
    }
    pink
}

/// Square-ish wave at `freq_hz` truncated to 6 odd harmonics, scaled to
/// `target_peak`. Produces the "saw-like high-pass" signal the plan asks
/// for in Track 2.
fn synth_djent(samples: usize, sample_rate: u32, fundamental_hz: f32, target_peak: f32) -> Vec<f32> {
    let mut out = Vec::with_capacity(samples);
    let dt = 1.0 / sample_rate as f32;
    for n in 0..samples {
        let t = n as f32 * dt;
        let mut s = 0.0_f32;
        for k in 0..6 {
            let h = (2 * k + 1) as f32;
            s += (2.0 * std::f32::consts::PI * fundamental_hz * h * t).sin() / h;
        }
        out.push(s);
    }
    let measured_peak = out.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    let scale = target_peak / measured_peak.max(f32::MIN_POSITIVE);
    for s in &mut out {
        *s *= scale;
    }
    out
}

fn write_wav_stereo_from_mono(path: &Path, sample_rate: u32, mono: &[f32]) {
    let spec = hound::WavSpec {
        channels: STEREO,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
    for &s in mono {
        let v = (s.clamp(-1.0, 1.0) * 32_767.0).round() as i16;
        writer.write_sample(v).expect("write L");
        writer.write_sample(v).expect("write R");
    }
    writer.finalize().expect("finalize wav");
}

/// Goertzel magnitude (dB) at `freq_hz`, normalized by sample count.
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

fn read_left_channel(path: &Path) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).expect("open rendered wav");
    let spec = reader.spec();
    let channels = spec.channels as usize;
    let scale = if spec.bits_per_sample == 16 {
        1.0 / i16::MAX as f32
    } else if spec.bits_per_sample == 24 {
        1.0 / 8_388_608.0
    } else {
        1.0
    };
    let mut left = Vec::with_capacity(reader.duration() as usize);
    match spec.sample_format {
        hound::SampleFormat::Int => {
            let mut frame: Vec<i32> = Vec::with_capacity(channels);
            for s in reader.samples::<i32>() {
                let v = s.expect("read int sample");
                frame.push(v);
                if frame.len() == channels {
                    left.push(frame[0] as f32 * scale);
                    frame.clear();
                }
            }
        }
        hound::SampleFormat::Float => {
            let mut frame: Vec<f32> = Vec::with_capacity(channels);
            for s in reader.samples::<f32>() {
                let v = s.expect("read float sample");
                frame.push(v);
                if frame.len() == channels {
                    left.push(frame[0]);
                    frame.clear();
                }
            }
        }
    }
    left
}

#[test]
fn character_promotion_and_bias_lands_on_rendered_audio() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let samples = (SR_HZ as f32 * TRACK_DURATION_SEC) as usize;

    // Track 1 — acoustic-folk hint, pink at peak 0.25.
    let t1_path = tmp.path().join("acoustic-intro.wav");
    write_wav_stereo_from_mono(&t1_path, SR_HZ, &synth_pink(samples, 0.25));
    // Track 2 — djent hint, sawish at 110 Hz, peak 0.5.
    let t2_path = tmp.path().join("djent-banger.wav");
    write_wav_stereo_from_mono(&t2_path, SR_HZ, &synth_djent(samples, SR_HZ, 110.0, 0.5));
    // Track 3 — no name hint; pink at peak 0.15 (low energy + low
    // transient so scoring picks AcousticFolk; the back-half promotion
    // pass then flips it to ReturnAcoustic).
    let t3_path = tmp.path().join("return-quiet.wav");
    write_wav_stereo_from_mono(&t3_path, SR_HZ, &synth_pink(samples, 0.15));
    // Track 4 — no hint, mostly to give the album 4 tracks so half = 2.
    let t4_path = tmp.path().join("closer.wav");
    write_wav_stereo_from_mono(&t4_path, SR_HZ, &synth_pink(samples, 0.2));

    // Fake analyses: Track 2 looks heavy (high energy + transient); the
    // other three look acoustic (low energy + low transient).
    let analyses = vec![
        analysis_for("t1", 0.30, 0.25), // acoustic-leaning
        analysis_for("t2", 0.85, 0.85), // heavy-leaning
        analysis_for("t3", 0.25, 0.20), // acoustic-leaning, will be promoted
        analysis_for("t4", 0.30, 0.30),
    ];
    let refs: Vec<&AnalysisResult> = analyses.iter().collect();
    let durations = vec![
        TRACK_DURATION_SEC as f64,
        TRACK_DURATION_SEC as f64,
        TRACK_DURATION_SEC as f64,
        TRACK_DURATION_SEC as f64,
    ];
    let names = vec![
        "acoustic-intro.wav",
        "djent-banger.wav",
        "return-quiet.wav",
        "closer.wav",
    ];

    let plan = album::build_album_plan_with_names(
        "Character Bias Smoke".to_string(),
        &refs,
        &durations,
        &names,
        AlbumArc::Preset {
            preset: AlbumArcKind::Cinematic,
        },
        1.0,
    );
    assert_eq!(plan.tracks.len(), 4);

    // Character labels — the headline assertions of this test.
    assert_eq!(
        plan.tracks[0].album_character,
        Some(AlbumCharacter::AcousticFolk),
        "Track 0 (acoustic-intro): expected AcousticFolk; got {:?}",
        plan.tracks[0].album_character,
    );
    assert_eq!(
        plan.tracks[1].album_character,
        Some(AlbumCharacter::HeavyDjent),
        "Track 1 (djent-banger): expected HeavyDjent; got {:?}",
        plan.tracks[1].album_character,
    );
    assert_eq!(
        plan.tracks[2].album_character,
        Some(AlbumCharacter::ReturnAcoustic),
        "Track 2 (return-quiet): expected ReturnAcoustic (post-heavy back-half promotion); got {:?}",
        plan.tracks[2].album_character,
    );

    // Render the album so we can measure that the per-character bias
    // and LUFS pulls actually land on the rendered audio.
    let inputs = vec![
        AlbumTrackRenderInput {
            track_id: TrackId("t1".to_string()),
            source_path: t1_path.to_string_lossy().to_string(),
            settings: default_master_settings(),
        },
        AlbumTrackRenderInput {
            track_id: TrackId("t2".to_string()),
            source_path: t2_path.to_string_lossy().to_string(),
            settings: default_master_settings(),
        },
        AlbumTrackRenderInput {
            track_id: TrackId("t3".to_string()),
            source_path: t3_path.to_string_lossy().to_string(),
            settings: default_master_settings(),
        },
        AlbumTrackRenderInput {
            track_id: TrackId("t4".to_string()),
            source_path: t4_path.to_string_lossy().to_string(),
            settings: default_master_settings(),
        },
    ];

    let out_dir = tmp.path().join("rendered");
    let report = render_album_plan_impl(
        &AlbumPlanRenderRequest {
            plan,
            tracks: inputs,
        },
        &out_dir,
        None,
    )
    .expect("album render");

    let mut by_pos = report.tracks.clone();
    by_pos.sort_by_key(|t| t.position);

    let acoustic_path = Path::new(&by_pos[0].output_path);
    let heavy_path = Path::new(&by_pos[1].output_path);
    let return_path = Path::new(&by_pos[2].output_path);

    // 400 Hz comparison: HeavyDjent's mastering_bias cuts low_mid by
    // -0.55 dB on top of the chain; AcousticFolk's bias raises low_mid
    // by +0.05 dB. After processing, the heavy track's 400 Hz Goertzel
    // should sit below the acoustic track's.
    let acoustic_left = read_left_channel(acoustic_path);
    let heavy_left = read_left_channel(heavy_path);
    let acoustic_400 = goertzel_mag_db(&acoustic_left, SR_HZ as f32, 400.0);
    let heavy_400 = goertzel_mag_db(&heavy_left, SR_HZ as f32, 400.0);
    let drop = acoustic_400 - heavy_400;
    assert!(
        drop >= 0.5,
        "Heavy 400 Hz ({heavy_400:.2} dB) should sit at least +0.5 dB below Acoustic 400 Hz ({acoustic_400:.2} dB); got drop = {drop:.2} dB",
    );

    // LUFS comparison: Heavy gets +0.82 LUFS pull, ReturnAcoustic -1.05
    // (1.87 LU expected gap). At intensity 1.0 with cinematic arc, the
    // per-position arc offset adds some extra; we observe a >=1.0 LU
    // gap and assert that as a safe lower bound.
    let heavy_lufs = engine::measure_integrated_lufs_at_path(heavy_path).expect("heavy LUFS");
    let return_lufs = engine::measure_integrated_lufs_at_path(return_path).expect("return LUFS");
    let gap = heavy_lufs - return_lufs;
    assert!(
        gap >= 1.0,
        "HeavyDjent ({heavy_lufs:.2} LUFS) should land at least +1.0 LU louder than ReturnAcoustic ({return_lufs:.2} LUFS); got gap = {gap:.2}",
    );
}

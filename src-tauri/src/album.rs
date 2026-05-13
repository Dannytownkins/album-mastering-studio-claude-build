//! Phase B — Album Master mode planning.
//!
//! Pure math + plan-construction for album-mode rendering. The runtime
//! render path lives in `engine.rs::render_album` (Phase B Step 3); this
//! module is responsible for:
//!
//! * Resampling the 6-point arc curves (ported from Codex's
//!   `arc.py::ARC_PRESETS`) to actual track count via cosine easing.
//! * Computing the per-track LUFS offset = arc_offset +
//!   source_compensation + character_offset, exactly the same composition
//!   Codex's `arc.py::build_album_arc` does.
//! * Mapping per-track `AnalysisResult` data into the per-track plan
//!   entries (role with first/last/short-track special cases).
//!
//! All measurements are read from `AnalysisResult` — Phase A5 plumbed
//! `transient_flux` and `energy_density_score` precisely to make this
//! module easy to write. The planner is deterministic and does not touch
//! disk or audio buffers.

use crate::types::*;

/// Cosine-eased resample of a 6-point curve to `n` output samples.
/// Ported verbatim from Codex's `arc.py::_resample_curve` (lines 202–218).
///
/// `n == 0` returns an empty vec; `n == 1` returns the first point.
pub fn resample_arc_curve(curve: [f32; 6], n: usize) -> Vec<f32> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![curve[0]];
    }
    let mut out = Vec::with_capacity(n);
    let last = (curve.len() - 1) as f32; // 5
    let denom = (n as f32 - 1.0).max(1.0);
    for i in 0..n {
        let position = i as f32 * last / denom;
        let lower = position.floor() as usize;
        let upper = (lower + 1).min(curve.len() - 1);
        let fraction = position - lower as f32;
        // Cosine ease — Codex uses (1 - cos(π·t)) / 2 which maps t=0 to 0,
        // t=0.5 to 0.5, t=1 to 1 with the slope tangent to the endpoints
        // (smoother than linear interp).
        let eased = 0.5 - 0.5 * (std::f32::consts::PI * fraction).cos();
        out.push(curve[lower] * (1.0 - eased) + curve[upper] * eased);
    }
    out
}

/// Per-track LUFS offset driven by inferred character. Captures Codex's
/// per-character "this section should sit a bit louder/quieter than the
/// album-intent target" intent (`arc.py:287-299`). Codex labels
/// (acoustic_folk / transition / heavy_djent / return_acoustic) don't map
/// 1:1 to our `TrackCharacter` enum (we don't carry position-aware labels
/// like "return_acoustic"), so we project the four-label intent onto our
/// five-label enum via the intrinsic-character axis:
///
/// * `Sparse`   → -0.72 dB  (Codex `acoustic_folk`). Extra -0.25 at
///                first track (Codex's acoustic-opener bonus).
///                Extra -0.20 at last track (Codex's return_acoustic
///                last-track bonus).
/// * `Dense`    → +0.82 dB  (Codex `heavy_djent`).
/// * `Bright`, `Dark`, `Balanced`, `None` → 0 dB.
pub fn character_loudness_offset(
    character: Option<TrackCharacter>,
    index: usize,
    count: usize,
) -> f32 {
    let Some(c) = character else { return 0.0 };
    let mut offset = match c {
        TrackCharacter::Sparse => -0.72,
        TrackCharacter::Dense => 0.82,
        TrackCharacter::Bright | TrackCharacter::Dark | TrackCharacter::Balanced => {
            0.0
        }
    };
    if matches!(c, TrackCharacter::Sparse) {
        if index == 0 {
            offset -= 0.25;
        }
        if count > 1 && index == count - 1 {
            offset -= 0.20;
        }
    }
    offset
}

/// Per-track LUFS offset from the arc curve and source energy. Mirror of
/// Codex's `arc.py:125-128`:
///
///   arc_offset    = (curve[i] - 0.5) * 3.2 * intensity
///   source_comp   = (0.5 - energy_density) * 0.45
///   char_offset   = character_loudness_offset(character, i, n)
///   track_offset  = arc_offset + source_comp + char_offset
///
/// `energy_density` defaults to 0.5 (neutral) when the analysis didn't
/// produce a score, so a track with no `energy_density_score` simply gets
/// no source compensation.
pub fn track_loudness_offset(
    curve_value: f32,
    energy_density: Option<f32>,
    character: Option<TrackCharacter>,
    intensity: f32,
    index: usize,
    count: usize,
) -> f32 {
    let energy = energy_density.unwrap_or(0.5);
    let arc_offset = (curve_value - 0.5) * 3.2 * intensity;
    let source_comp = (0.5 - energy) * 0.45;
    let char_offset = character_loudness_offset(character, index, count);
    arc_offset + source_comp + char_offset
}

/// Inferred role at album-plan position. Rules:
///
///   index = 0     → Opener (overrides per-track inference)
///   index = N-1   → Closer
///   short (<90 s) AND low transient (flux < 0.4, or transient_density < 0.4
///                 when flux is unavailable) → Interlude
///   else → per-track `AnalysisResult.inferred_role` if present, else
///          `AlbumTrack`
pub fn role_at_position(
    analysis: &AnalysisResult,
    index: usize,
    count: usize,
    duration_seconds: f64,
) -> TrackRole {
    if count > 1 {
        if index == 0 {
            return TrackRole::Opener;
        }
        if index == count - 1 {
            return TrackRole::Closer;
        }
    }
    let transient_signal = analysis
        .transient_flux
        .unwrap_or(analysis.transient_density);
    if duration_seconds > 0.0 && duration_seconds < 90.0 && transient_signal < 0.4 {
        return TrackRole::Interlude;
    }
    analysis.inferred_role.unwrap_or(TrackRole::AlbumTrack)
}

/// Default transition between two adjacent tracks. Simple heuristic for
/// v1: gap any pair where either side is `Interlude` (the natural place
/// for an album to "breathe"); everything else butt-splices.
pub fn default_transition_for(
    left: &AlbumTrackEntry,
    right: &AlbumTrackEntry,
) -> TransitionSpec {
    if matches!(
        left.role,
        TrackRole::Interlude
    ) || matches!(right.role, TrackRole::Interlude)
    {
        TransitionSpec::gap(0.8)
    } else {
        TransitionSpec::direct()
    }
}

/// Resolve an `AlbumArc` into a per-track LUFS-offset table. For the
/// `Preset` variant this runs the full arc → resample → per-track-offset
/// pipeline; for `Custom` it simply returns the user's manual offsets
/// (padded with 0.0 when the user passed fewer values than tracks).
fn resolve_arc_offsets(
    arc: &AlbumArc,
    analyses: &[&AnalysisResult],
    intensity: f32,
    characters: &[Option<TrackCharacter>],
) -> Vec<f32> {
    let n = analyses.len();
    match arc {
        AlbumArc::Preset { preset } => {
            let curve = resample_arc_curve(preset.curve(), n);
            (0..n)
                .map(|i| {
                    track_loudness_offset(
                        curve.get(i).copied().unwrap_or(0.5),
                        analyses[i].energy_density_score,
                        characters[i],
                        intensity,
                        i,
                        n,
                    )
                })
                .collect()
        }
        AlbumArc::Custom { lufs_offsets } => (0..n)
            .map(|i| lufs_offsets.get(i).copied().unwrap_or(0.0))
            .collect(),
    }
}

/// Build a full `AlbumPlan` from the user's track order, per-track
/// analyses, and arc / intensity choice. Default transitions are filled
/// in via `default_transition_for`. The caller is expected to:
///
///   * Pass `analyses` in playback order (same as `track_paths`).
///   * Provide `durations` in seconds (from `ImportedTrack.duration_seconds`
///     or 0.0 when unknown).
///   * Pass the album title.
///   * Pass the user-chosen `AlbumArc` and `intensity` (clamped to `[0, 2]`).
///
/// Re-planning after the user reorders tracks or changes the arc should
/// preserve `role_locked` overrides — call this function with fresh data
/// and merge `role_locked` from the prior plan onto the new entries.
pub fn build_album_plan(
    title: String,
    analyses: &[&AnalysisResult],
    durations: &[f64],
    arc: AlbumArc,
    intensity: f32,
) -> AlbumPlan {
    let n = analyses.len();
    let intensity = intensity.clamp(0.0, 2.0);
    if n == 0 {
        return AlbumPlan {
            title,
            arc,
            tracks: Vec::new(),
            transitions: Vec::new(),
            intensity,
        };
    }
    let characters: Vec<Option<TrackCharacter>> =
        analyses.iter().map(|a| a.inferred_character).collect();
    let offsets = resolve_arc_offsets(&arc, analyses, intensity, &characters);

    let mut tracks: Vec<AlbumTrackEntry> = Vec::with_capacity(n);
    for (i, analysis) in analyses.iter().enumerate() {
        let duration = durations.get(i).copied().unwrap_or(0.0);
        let role = role_at_position(analysis, i, n, duration);
        tracks.push(AlbumTrackEntry {
            track_id: analysis.track_id.clone(),
            position: (i + 1) as u32,
            role,
            role_locked: false,
            arc_lufs_offset_db: offsets.get(i).copied().unwrap_or(0.0),
            intensity_scale: 1.0,
        });
    }
    let mut transitions: Vec<TransitionSpec> = Vec::with_capacity(n.saturating_sub(1));
    for i in 0..n.saturating_sub(1) {
        transitions.push(default_transition_for(&tracks[i], &tracks[i + 1]));
    }
    AlbumPlan {
        title,
        arc,
        tracks,
        transitions,
        intensity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_analysis(
        id: &str,
        inferred_role: Option<TrackRole>,
        character: Option<TrackCharacter>,
        energy_density: Option<f32>,
        transient_flux: Option<f32>,
    ) -> AnalysisResult {
        AnalysisResult {
            track_id: TrackId(id.to_string()),
            lufs_integrated: -14.0,
            lufs_short_term_max: -10.0,
            true_peak_dbtp: -1.0,
            dynamic_range_lu: 8.0,
            spectral_balance: SpectralBalance {
                low: 0.33,
                mid: 0.34,
                high: 0.33,
            },
            transient_density: transient_flux.unwrap_or(0.5),
            stereo_width: 0.5,
            recommended_universal: default_master_settings(),
            measured_at_iso: ISO_PLACEHOLDER.to_string(),
            inferred_role,
            role_confidence: Some(InferenceConfidence::Moderate),
            inferred_character: character,
            character_confidence: character.map(|_| InferenceConfidence::Moderate),
            spectral_balance_6band: None,
            transient_flux,
            stereo_correlation: None,
            dynamic_range_p95_p10_db: None,
            lufs_short_term_max_3s: None,
            energy_density_score: energy_density,
        }
    }

    fn default_master_settings() -> MasteringSettings {
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
            delivery_profile: DeliveryProfile::Custom,
            album: None,
            advanced: AdvancedSettings::default(),
        }
    }

    /// Resample-to-N=6 must return the exact input curve unchanged.
    #[test]
    fn resample_arc_curve_identity_when_n_equals_6() {
        let curve = AlbumArcKind::Cinematic.curve();
        let resampled = resample_arc_curve(curve, 6);
        for (i, &v) in curve.iter().enumerate() {
            assert!(
                (resampled[i] - v).abs() < 1.0e-5,
                "index {} expected {} got {}",
                i,
                v,
                resampled[i]
            );
        }
    }

    /// Resample to smaller N preserves endpoints.
    #[test]
    fn resample_arc_curve_preserves_endpoints() {
        let curve = AlbumArcKind::Cinematic.curve(); // [0.32, ..., 0.46]
        let resampled = resample_arc_curve(curve, 3);
        assert_eq!(resampled.len(), 3);
        assert!((resampled[0] - 0.32).abs() < 1.0e-5);
        assert!((resampled[2] - 0.46).abs() < 1.0e-5);
    }

    /// Resample to larger N is monotonically interpolated where the
    /// underlying curve is monotonic. Cinematic rises 0.32 → 1.00 across
    /// indices 0..3 — the resampled values should rise too.
    #[test]
    fn resample_arc_curve_monotonic_in_rising_segment() {
        let curve = AlbumArcKind::Cinematic.curve();
        let resampled = resample_arc_curve(curve, 12);
        // First 7 samples cover indices 0..3 of the source (rising segment).
        for i in 1..7 {
            assert!(
                resampled[i] >= resampled[i - 1] - 1.0e-5,
                "expected non-decreasing in rising segment at i={}: {} -> {}",
                i,
                resampled[i - 1],
                resampled[i]
            );
        }
    }

    /// N = 1 returns the first point only.
    #[test]
    fn resample_arc_curve_single_track() {
        let curve = AlbumArcKind::Afterhours.curve();
        let resampled = resample_arc_curve(curve, 1);
        assert_eq!(resampled.len(), 1);
        assert!((resampled[0] - 0.78).abs() < 1.0e-5);
    }

    /// Character offset table from Codex's arc.py:287-299, mapped onto
    /// our TrackCharacter enum.
    #[test]
    fn character_loudness_offset_table() {
        // Sparse (≈ acoustic_folk): -0.72 base.
        assert!(
            (character_loudness_offset(Some(TrackCharacter::Sparse), 1, 4) - (-0.72))
                .abs()
                < 1.0e-5
        );
        // Sparse at first track: extra -0.25.
        assert!(
            (character_loudness_offset(Some(TrackCharacter::Sparse), 0, 4) - (-0.97))
                .abs()
                < 1.0e-5
        );
        // Sparse at last track: extra -0.20.
        assert!(
            (character_loudness_offset(Some(TrackCharacter::Sparse), 3, 4) - (-0.92))
                .abs()
                < 1.0e-5
        );
        // Dense (≈ heavy_djent): +0.82.
        assert!(
            (character_loudness_offset(Some(TrackCharacter::Dense), 1, 4) - 0.82).abs()
                < 1.0e-5
        );
        // Bright / Dark / Balanced / None: 0.
        for c in [
            Some(TrackCharacter::Bright),
            Some(TrackCharacter::Dark),
            Some(TrackCharacter::Balanced),
            None,
        ] {
            assert!(character_loudness_offset(c, 1, 4).abs() < 1.0e-5);
        }
    }

    /// `track_loudness_offset` composes arc + source_comp + char.
    #[test]
    fn track_loudness_offset_composition() {
        // At curve=0.5 (neutral), no source_comp, no character offset →
        // arc_offset = 0, total = 0.
        let v = track_loudness_offset(0.5, Some(0.5), None, 1.0, 1, 4);
        assert!(v.abs() < 1.0e-5);
        // At curve=1.0, intensity=1.0: arc_offset = (1.0 - 0.5) * 3.2 = +1.6.
        // No source comp, no char offset.
        let v = track_loudness_offset(1.0, Some(0.5), None, 1.0, 1, 4);
        assert!((v - 1.6).abs() < 1.0e-5);
        // At curve=0.5, energy=0.0: source_comp = (0.5 - 0.0) * 0.45 = +0.225.
        let v = track_loudness_offset(0.5, Some(0.0), None, 1.0, 1, 4);
        assert!((v - 0.225).abs() < 1.0e-5);
    }

    /// Role inference: first → Opener, last → Closer, short+sparse → Interlude.
    #[test]
    fn role_at_position_basic() {
        let opener = fake_analysis("a", Some(TrackRole::Single), None, None, Some(0.5));
        let closer = fake_analysis("b", Some(TrackRole::Single), None, None, Some(0.5));
        let middle =
            fake_analysis("m", Some(TrackRole::AlbumTrack), None, None, Some(0.5));
        let interlude = fake_analysis("i", None, None, None, Some(0.1));

        assert_eq!(role_at_position(&opener, 0, 4, 180.0), TrackRole::Opener);
        assert_eq!(role_at_position(&closer, 3, 4, 180.0), TrackRole::Closer);
        assert_eq!(
            role_at_position(&middle, 1, 4, 180.0),
            TrackRole::AlbumTrack
        );
        // Short + low transient → Interlude (when not first/last).
        assert_eq!(
            role_at_position(&interlude, 1, 4, 60.0),
            TrackRole::Interlude
        );
    }

    /// build_album_plan end-to-end on 3 tracks. Verifies the plan has
    /// the right shape (3 tracks, 2 transitions), Opener/Closer assignment,
    /// and that the arc_lufs_offset_db field is populated.
    #[test]
    fn build_album_plan_three_tracks_cinematic() {
        let analyses = [
            fake_analysis(
                "t1",
                Some(TrackRole::AlbumTrack),
                Some(TrackCharacter::Sparse),
                Some(0.4),
                Some(0.5),
            ),
            fake_analysis(
                "t2",
                Some(TrackRole::Single),
                Some(TrackCharacter::Dense),
                Some(0.75),
                Some(0.8),
            ),
            fake_analysis(
                "t3",
                Some(TrackRole::Ballad),
                Some(TrackCharacter::Sparse),
                Some(0.35),
                Some(0.4),
            ),
        ];
        let refs: Vec<&AnalysisResult> = analyses.iter().collect();
        let durations = [180.0, 220.0, 260.0];
        let plan = build_album_plan(
            "Test Album".to_string(),
            &refs,
            &durations,
            AlbumArc::Preset {
                preset: AlbumArcKind::Cinematic,
            },
            1.0,
        );
        assert_eq!(plan.tracks.len(), 3);
        assert_eq!(plan.transitions.len(), 2);
        assert_eq!(plan.tracks[0].role, TrackRole::Opener);
        assert_eq!(plan.tracks[2].role, TrackRole::Closer);
        // Cinematic curve at i=1 (the peak segment, value ≈ 1.0) should
        // push the second track louder than neutral (positive offset).
        assert!(
            plan.tracks[1].arc_lufs_offset_db > 0.0,
            "Cinematic peak track expected positive arc offset, got {}",
            plan.tracks[1].arc_lufs_offset_db
        );
    }

    /// AlbumArc::Custom uses the user-provided per-track offsets verbatim,
    /// bypassing arc / source / character math.
    #[test]
    fn build_album_plan_custom_arc_uses_explicit_offsets() {
        let analyses = [
            fake_analysis("t1", None, None, None, None),
            fake_analysis("t2", None, None, None, None),
        ];
        let refs: Vec<&AnalysisResult> = analyses.iter().collect();
        let durations = [180.0, 180.0];
        let plan = build_album_plan(
            "Test".to_string(),
            &refs,
            &durations,
            AlbumArc::Custom {
                lufs_offsets: vec![-1.5, 2.5],
            },
            1.0,
        );
        assert_eq!(plan.tracks[0].arc_lufs_offset_db, -1.5);
        assert_eq!(plan.tracks[1].arc_lufs_offset_db, 2.5);
    }

    /// Empty plan: 0 tracks, 0 transitions, intensity preserved.
    #[test]
    fn build_album_plan_empty() {
        let plan = build_album_plan(
            "Empty".to_string(),
            &[],
            &[],
            AlbumArc::default(),
            1.0,
        );
        assert_eq!(plan.tracks.len(), 0);
        assert_eq!(plan.transitions.len(), 0);
        assert_eq!(plan.intensity, 1.0);
    }
}

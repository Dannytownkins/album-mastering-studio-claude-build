use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct TrackId(pub String);

impl TrackId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TrackId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportedTrack {
    pub id: TrackId,
    pub path: String,
    pub display_name: String,
    pub source_format: String,
    pub duration_seconds: Option<f64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SpectralBalance {
    pub low: f32,
    pub mid: f32,
    pub high: f32,
}

/// Phase A5 — finer-grained spectral split via FFT. Bands (approx.):
///   sub      20–80 Hz
///   low      80–250 Hz
///   low_mid  250–800 Hz
///   mid      800–2500 Hz
///   presence 2500–6500 Hz
///   air      6500–min(sr/2, 16000) Hz
/// Fractional values sum to ~1.0. None when the signal is too short or
/// silent for a meaningful FFT.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SpectralBalance6 {
    pub sub: f32,
    pub low: f32,
    pub low_mid: f32,
    pub mid: f32,
    pub presence: f32,
    pub air: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnalysisResult {
    pub track_id: TrackId,
    pub lufs_integrated: f32,
    pub lufs_short_term_max: f32,
    pub true_peak_dbtp: f32,
    pub dynamic_range_lu: f32,
    pub spectral_balance: SpectralBalance,
    pub transient_density: f32,
    pub stereo_width: f32,
    pub recommended_universal: MasteringSettings,
    pub measured_at_iso: String,
    // Phase 9: heuristic role + character inference. Optional so older callers
    // / serialized analyses still parse cleanly.
    #[serde(default)]
    pub inferred_role: Option<TrackRole>,
    #[serde(default)]
    pub role_confidence: Option<InferenceConfidence>,
    #[serde(default)]
    pub inferred_character: Option<TrackCharacter>,
    #[serde(default)]
    pub character_confidence: Option<InferenceConfidence>,
    // Phase A5: richer analysis measurements ported from Codex's
    // analysis.py. All optional so older serialized analyses still load.
    /// 6-band spectral balance via FFT (Hann-windowed, up to 30 s of mono).
    #[serde(default)]
    pub spectral_balance_6band: Option<SpectralBalance6>,
    /// Spectral-flux-based transient density. Higher = more percussive.
    /// 40 ms windows, 10 ms hop, positive flux normalized to mean RMS.
    #[serde(default)]
    pub transient_flux: Option<f32>,
    /// Pearson correlation between L and R channels. `[-1.0, +1.0]`.
    /// `None` for mono input.
    #[serde(default)]
    pub stereo_correlation: Option<f32>,
    /// Dynamic range as P95 minus P10 of RMS-block dB values. Better
    /// "how dynamic does this track feel" than crest factor. 100 ms
    /// windows at 50 ms hop.
    #[serde(default)]
    pub dynamic_range_p95_p10_db: Option<f32>,
    /// Maximum short-term (3 s sliding) LUFS via ebur128 Mode::S. True
    /// measurement, replaces the integrated+LRA*0.5 estimate.
    #[serde(default)]
    pub lufs_short_term_max_3s: Option<f32>,
    /// Composite "how hot does this mix feel" score in `[0, 1]`. Weighted
    /// combination of loudness, brightness, density, transient flux per
    /// Codex's analysis.py formula.
    #[serde(default)]
    pub energy_density_score: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackRole {
    Opener,
    Closer,
    Single,
    Ballad,
    Interlude,
    AlbumTrack,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackCharacter {
    Bright,
    Dark,
    Dense,
    Sparse,
    Balanced,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InferenceConfidence {
    Strong,
    Moderate,
    Unsure,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Preset {
    Universal,
    Clarity,
    Tape,
    Spatial,
    Oomph,
    Warmth,
    Punch,
    Loud,
    Custom { id: String },
}

/// Phase A3: Delivery profile presets ported from
/// `../album-mastering-studio/src/album_mastering_studio/standards.py`.
///
/// Each non-`Custom` variant carries a complete (target LUFS, ceiling,
/// sample-rate hint, bit-depth) bundle for a specific delivery target.
/// At render time, when `delivery_profile != Custom`, the profile's
/// values shadow the corresponding fields in `AdvancedSettings`. `Custom`
/// means "use the user's explicit `lufs_offset_db` / `ceiling_dbtp` /
/// `bit_depth` / `target_sample_rate` exactly as set."
///
/// Sample rate is captured per profile but resampling is deferred to a
/// later phase — A3 honors `bit_depth`, `target_lufs`, and `ceiling_dbtp`
/// but writes WAVs at the source sample rate regardless. The captured
/// rate is exposed via `output_sample_rate()` for future use.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryProfile {
    /// -14 LUFS, -1 dBTP, 48 kHz, 24-bit. Spotify / YouTube / Tidal / Amazon.
    StreamingUniversal,
    /// -16 LUFS, -1 dBTP, 48 kHz, 24-bit. Apple Music's tighter target.
    AppleMusic,
    /// -14 LUFS, -1 dBTP, 44.1 kHz, 16-bit. Red Book CD.
    Cd,
    /// -18 LUFS, -3 dBTP, 48 kHz, 24-bit. Generous headroom for the
    /// cutting engineer / RIAA pre-emphasis.
    VinylPremaster,
    /// -10.5 LUFS, -1 dBTP, 48 kHz, 24-bit. Rock / metal masters that
    /// don't translate well at the streaming -14 target.
    LoudRock,
    /// -23 LUFS, -1 dBTP, 48 kHz, 24-bit. EBU R128 broadcast.
    BroadcastEu,
    /// -24 LUFS, -2 dBTP, 48 kHz, 24-bit. ATSC A/85 broadcast.
    BroadcastUs,
    /// No shadowing — render uses the user's explicit `lufs_offset_db`,
    /// `ceiling_dbtp`, `bit_depth`, and `target_sample_rate` fields
    /// from `AdvancedSettings` verbatim.
    Custom,
}

impl Default for DeliveryProfile {
    fn default() -> Self {
        Self::StreamingUniversal
    }
}

impl DeliveryProfile {
    /// Target integrated LUFS for non-Custom profiles. `None` when
    /// `Custom` (engine falls back to `AdvancedSettings::lufs_offset_db`).
    pub fn target_lufs(&self) -> Option<f32> {
        match self {
            Self::StreamingUniversal => Some(-14.0),
            Self::AppleMusic => Some(-16.0),
            Self::Cd => Some(-14.0),
            Self::VinylPremaster => Some(-18.0),
            Self::LoudRock => Some(-10.5),
            Self::BroadcastEu => Some(-23.0),
            Self::BroadcastUs => Some(-24.0),
            Self::Custom => None,
        }
    }

    /// True-peak ceiling in dBTP for non-Custom profiles. `None` for
    /// `Custom`.
    pub fn ceiling_dbtp(&self) -> Option<f32> {
        match self {
            Self::StreamingUniversal
            | Self::AppleMusic
            | Self::Cd
            | Self::LoudRock
            | Self::BroadcastEu => Some(-1.0),
            Self::VinylPremaster => Some(-3.0),
            Self::BroadcastUs => Some(-2.0),
            Self::Custom => None,
        }
    }

    /// Recommended output sample rate. Captured for future resampling
    /// support — A3 does NOT resample; the renderer writes at the
    /// source's sample rate regardless of this value.
    pub fn output_sample_rate(&self) -> Option<u32> {
        match self {
            Self::Cd => Some(44_100),
            Self::StreamingUniversal
            | Self::AppleMusic
            | Self::VinylPremaster
            | Self::LoudRock
            | Self::BroadcastEu
            | Self::BroadcastUs => Some(48_000),
            Self::Custom => None,
        }
    }

    /// Output bit depth for non-Custom profiles. Honored by the WAV writer.
    pub fn output_bit_depth(&self) -> Option<u16> {
        match self {
            Self::Cd => Some(16),
            Self::StreamingUniversal
            | Self::AppleMusic
            | Self::VinylPremaster
            | Self::LoudRock
            | Self::BroadcastEu
            | Self::BroadcastUs => Some(24),
            Self::Custom => None,
        }
    }

    pub fn is_custom(&self) -> bool {
        matches!(self, Self::Custom)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::StreamingUniversal => "Streaming (Spotify / YouTube / Tidal / Amazon)",
            Self::AppleMusic => "Apple Music",
            Self::Cd => "CD (16-bit)",
            Self::VinylPremaster => "Vinyl Premaster",
            Self::LoudRock => "Loud Rock / Aggressive",
            Self::BroadcastEu => "Broadcast EU (EBU R128)",
            Self::BroadcastUs => "Broadcast US (ATSC A/85)",
            Self::Custom => "Custom",
        }
    }
}

// ============================================================================
// Phase B — Album Master mode types. See docs/ALBUM_MASTER_PLAN.md for the
// full spec. Each non-`Custom` AlbumArc variant carries one of the four
// 6-point intensity curves ported from Codex's `arc.py::ARC_PRESETS`. The
// runtime cosine-eased resample to actual track count lives in
// `engine.rs::arc_planner`.
// ============================================================================

/// Phase B+ — position-aware album character labels ported from Codex's
/// `character.py`. Distinct from `TrackCharacter` which is intrinsic
/// (Bright/Dark/Dense/Sparse/Balanced); these are inferred per-track in
/// the context of the WHOLE album: a track may be `AcousticFolk` on
/// its own but `ReturnAcoustic` if it sits in the back half AFTER a
/// `HeavyDjent` track in the same album.
///
/// Used by the arc planner to apply per-character LUFS offsets and the
/// per-character `mastering_bias` EQ moves. Optional — older serialized
/// plans / analyses load with None.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum AlbumCharacter {
    AcousticFolk,
    Transition,
    HeavyDjent,
    /// `AcousticFolk` whose album-position falls AFTER a `HeavyDjent`
    /// track and in the back half. Codex's listening sessions found
    /// these tracks needed a different (deeper) LUFS pull than a
    /// front-half acoustic track.
    ReturnAcoustic,
}

impl AlbumCharacter {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::AcousticFolk => "Acoustic / Folk",
            Self::Transition => "Transition",
            Self::HeavyDjent => "Heavy / Djent",
            Self::ReturnAcoustic => "Return / Acoustic",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AlbumArcKind {
    /// Codex curve (0.32, 0.52, 0.78, 1.00, 0.70, 0.46). Invitation →
    /// climb → peak → release → afterglow.
    Cinematic,
    /// Codex curve (0.78, 0.66, 0.55, 0.43, 0.34, 0.28). Bright → dim →
    /// private.
    Afterhours,
    /// Codex curve (0.46, 0.62, 0.78, 0.96, 1.00, 0.74). DJ-set energy ramp.
    ClubPeak,
    /// Codex curve (0.58, 0.34, 0.86, 0.48, 1.00, 0.39). Deliberately
    /// unstable.
    FeverDream,
}

impl AlbumArcKind {
    /// The 6-point intensity curve for this arc — values in roughly
    /// `[0.2, 1.0]`. Ported verbatim from Codex's
    /// `arc.py::ARC_PRESETS`.
    pub fn curve(&self) -> [f32; 6] {
        match self {
            Self::Cinematic => [0.32, 0.52, 0.78, 1.00, 0.70, 0.46],
            Self::Afterhours => [0.78, 0.66, 0.55, 0.43, 0.34, 0.28],
            Self::ClubPeak => [0.46, 0.62, 0.78, 0.96, 1.00, 0.74],
            Self::FeverDream => [0.58, 0.34, 0.86, 0.48, 1.00, 0.39],
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Cinematic => "Cinematic",
            Self::Afterhours => "Afterhours",
            Self::ClubPeak => "Club Peak",
            Self::FeverDream => "Fever Dream",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AlbumArc {
    /// One of the four named Codex arcs.
    Preset { preset: AlbumArcKind },
    /// Manual per-track LUFS offsets — one entry per track in playback
    /// order. Lets the user override the arc entirely.
    Custom { lufs_offsets: Vec<f32> },
}

impl Default for AlbumArc {
    fn default() -> Self {
        Self::Preset {
            preset: AlbumArcKind::Cinematic,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TransitionKind {
    /// Sample-accurate butt-splice. No silence between tracks.
    Direct,
    /// `duration_seconds` of digital silence between tracks.
    Gap,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct TransitionSpec {
    pub kind: TransitionKind,
    /// Clamped to `[0.0, 5.0]` at the planner / render layer. Ignored
    /// when `kind = Direct`.
    pub duration_seconds: f32,
}

impl TransitionSpec {
    pub const fn direct() -> Self {
        Self {
            kind: TransitionKind::Direct,
            duration_seconds: 0.0,
        }
    }
    pub const fn gap(seconds: f32) -> Self {
        Self {
            kind: TransitionKind::Gap,
            duration_seconds: seconds,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlbumTrackEntry {
    pub track_id: TrackId,
    /// 1-indexed playback position. The Vec position in
    /// `AlbumPlan::tracks` is the canonical order; this field is for
    /// the manifest and the per-track file-name prefix.
    pub position: u32,
    pub role: TrackRole,
    /// `true` once the user manually overrides the role so re-planning
    /// doesn't clobber the choice.
    #[serde(default)]
    pub role_locked: bool,
    /// Per-track LUFS shift applied by the arc planner. Added on top of
    /// the per-track `MasteringSettings::effective_target_lufs()` at
    /// render time. Negative = quieter than the album-intent target.
    pub arc_lufs_offset_db: f32,
    /// Per-track intensity multiplier. `1.0` = the album-intent intensity;
    /// >1.0 pushes harder for this track; <1.0 softens.
    pub intensity_scale: f32,
    /// Phase B+ — position-aware character label. Drives the
    /// per-character LUFS offset (built into arc_lufs_offset_db) and
    /// the per-character mastering_bias EQ moves at render time.
    /// None means "no album-character signal — use intrinsic character
    /// only / treat as a default track."
    #[serde(default)]
    pub album_character: Option<AlbumCharacter>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlbumPlan {
    pub title: String,
    #[serde(default)]
    pub arc: AlbumArc,
    /// Tracks in playback order. Vec position is authoritative; the
    /// `position` field on each entry is derived for display / manifest.
    pub tracks: Vec<AlbumTrackEntry>,
    /// `tracks.len() - 1` entries (or 0 for single-track albums).
    /// `transitions[i]` is the join between `tracks[i]` and
    /// `tracks[i + 1]`.
    pub transitions: Vec<TransitionSpec>,
    /// Album-level intensity multiplier — feeds into the arc resample
    /// and per-track DSP. Clamped `[0.0, 2.0]`.
    pub intensity: f32,
}

impl Default for AlbumPlan {
    fn default() -> Self {
        Self {
            title: String::new(),
            arc: AlbumArc::default(),
            tracks: Vec::new(),
            transitions: Vec::new(),
            intensity: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MasteringSettings {
    pub preset: Preset,
    pub intensity: f32,
    /// Phase B.1: 80 Hz peaking EQ. User offset on top of preset baseline.
    /// `#[serde(default)]` keeps older project files loading at 0.0.
    #[serde(default)]
    pub eq_sub_db: f32,
    pub eq_low_db: f32,
    /// Phase A2: low-mid peaking EQ (400 Hz, Q=0.9). User offset on top of
    /// the preset's baseline `low_mid_db`. `#[serde(default)]` so projects
    /// saved before this field existed load with a 0.0 value (matching the
    /// chain's identity behavior at neutral gain).
    #[serde(default)]
    pub eq_low_mid_db: f32,
    pub eq_mid_db: f32,
    /// Phase B.1: 3.5 kHz peaking EQ. User offset on top of preset baseline.
    /// `#[serde(default)]` keeps older project files loading at 0.0.
    #[serde(default)]
    pub eq_high_mid_db: f32,
    pub eq_high_db: f32,
    /// Phase B.1: 12 kHz high-shelf EQ. User offset on top of preset baseline.
    /// `#[serde(default)]` keeps older project files loading at 0.0.
    #[serde(default)]
    pub eq_sparkle_db: f32,
    pub volume_match: bool,
    /// Source-track integrated LUFS, injected by the playback driver
    /// before each `updateChain` so the chain can compute a proper
    /// Volume Match attenuation (target_lufs - source_lufs) instead of
    /// merely undoing the input-gain stage. `None` falls back to the
    /// legacy "undo input gain" behavior. Not user-facing — set by the
    /// frontend from the current track's `AnalysisResult.lufs_integrated`.
    #[serde(default)]
    pub source_lufs_integrated: Option<f32>,
    /// Pre-chain gain. Negative values back off the source before the preset
    /// EQ/saturation/limiter sees it — useful for already-mastered material
    /// that would otherwise clip when the preset adds its baseline gain push.
    /// Default 0 dB. Phase 12.1 Dan feedback.
    #[serde(default)]
    pub input_gain_db: f32,
    /// Post-limiter trim. Applied after the chain's limiter and volume-match
    /// stages. Default 0 dB. Boosting above 0 here can re-introduce peaks
    /// above the ceiling, which is intentionally allowed so a user can match
    /// reference loudness — but the export receipt's true-peak check will
    /// catch the result.
    #[serde(default)]
    pub output_gain_db: f32,
    /// Phase A3 — delivery profile preset. When non-`Custom`, shadows
    /// `lufs_offset_db`, `ceiling_dbtp`, and `bit_depth` at render time
    /// with the profile's values. `Custom` means "use the explicit
    /// advanced fields as-is." `#[serde(default)]` so older `.ams.json`
    /// projects load with the streaming-universal default.
    #[serde(default)]
    pub delivery_profile: DeliveryProfile,
    /// Phase B — Album Master mode. `None` for Track Master mode. When
    /// `Some`, the render pipeline reads per-track arc offsets / intensity
    /// scales from the plan and shadows the per-track settings accordingly.
    #[serde(default)]
    pub album: Option<AlbumPlan>,
    pub advanced: AdvancedSettings,
}

impl MasteringSettings {
    /// Phase A3 — effective target LUFS for the post-chain landing stage.
    /// When `delivery_profile` is non-`Custom`, the profile's target wins;
    /// `Custom` falls back to `advanced.lufs_offset_db`. Used by the render
    /// pipeline in `engine.rs`.
    pub fn effective_target_lufs(&self) -> Option<f32> {
        self.delivery_profile
            .target_lufs()
            .or(self.advanced.lufs_offset_db)
    }

    /// Phase A3 — effective true-peak ceiling in dBTP. Profile shadows the
    /// user-set value when non-`Custom`. Falls back to `-1.0` when no
    /// value is set anywhere (matches the prior default).
    pub fn effective_ceiling_dbtp(&self) -> f32 {
        self.delivery_profile
            .ceiling_dbtp()
            .or(self.advanced.ceiling_dbtp)
            .unwrap_or(-1.0)
    }

    /// Phase A3 — effective output bit depth. Profile shadows the user-
    /// set value when non-`Custom`. Falls back to `24` when no value is
    /// set anywhere (matches the prior default).
    pub fn effective_bit_depth(&self) -> u16 {
        self.delivery_profile
            .output_bit_depth()
            .or(self.advanced.bit_depth)
            .unwrap_or(24)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AdvancedSettings {
    pub lufs_offset_db: Option<f32>,
    pub ceiling_dbtp: Option<f32>,
    pub width: Option<f32>,
    pub warmth: Option<f32>,
    pub presence_air: Option<f32>,
    pub compression_density: Option<f32>,
    // Phase 12.2 — per-band compressor overrides. `None` => the macro slider
    // (compression_density) drives that band's threshold; per-band ratio /
    // attack / release fall back to fixed musical defaults (see
    // `ChainCoeffs::from_settings`). `Some(v)` => override the macro for this
    // band/parameter only. All `#[serde(default)]` so older sessions and
    // older frontends parse cleanly.
    #[serde(default)]
    pub compression_low_threshold_db: Option<f32>,
    #[serde(default)]
    pub compression_low_ratio: Option<f32>,
    #[serde(default)]
    pub compression_low_attack_ms: Option<f32>,
    #[serde(default)]
    pub compression_low_release_ms: Option<f32>,
    #[serde(default)]
    pub compression_mid_threshold_db: Option<f32>,
    #[serde(default)]
    pub compression_mid_ratio: Option<f32>,
    #[serde(default)]
    pub compression_mid_attack_ms: Option<f32>,
    #[serde(default)]
    pub compression_mid_release_ms: Option<f32>,
    #[serde(default)]
    pub compression_high_threshold_db: Option<f32>,
    #[serde(default)]
    pub compression_high_ratio: Option<f32>,
    #[serde(default)]
    pub compression_high_attack_ms: Option<f32>,
    #[serde(default)]
    pub compression_high_release_ms: Option<f32>,
    /// Phase 12.2 — when `Some(false)`, the multiband compressor runs
    /// independent L/R envelope followers per band. Default (`None` or
    /// `Some(true)`) links stereo: a single max-of-|L|,|R| envelope drives the
    /// same gain reduction on both channels, the standard mastering choice.
    #[serde(default)]
    pub compression_link_stereo: Option<bool>,
    pub bit_depth: Option<u16>,
    pub target_sample_rate: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WaveformPeaks {
    pub track_id: TrackId,
    pub channels: Vec<Vec<f32>>,
    pub samples_per_pixel: u32,
    pub total_samples: u64,
    pub sample_rate: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RenderKind {
    Preview,
    Master,
    Album,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Done,
    Failed { reason: String },
    Cancelled,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RenderJob {
    pub id: String,
    pub kind: RenderKind,
    pub target_tracks: Vec<TrackId>,
    pub status: JobStatus,
    pub progress: f32,
    pub started_at_iso: String,
    pub output_paths: Vec<String>,
    // Phase B+ Step 8 prep / Codex audit 2026-05-13 P0: measurements taken on
    // the rendered output samples, not on the source. `None` for render paths
    // that don't measure yet (album master). Single-track preview and master
    // populate this so the export receipt can stop quoting source analysis.
    #[serde(default)]
    pub measurements: Option<RenderedMeasurements>,
}

/// Post-render measurements taken on the final f32 sample buffer immediately
/// before WAV write. The receipt these feed shows the user what was actually
/// produced, not what the source looked like before the chain ran.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RenderedMeasurements {
    pub lufs_integrated: f32,
    pub true_peak_dbtp: f32,
    pub dynamic_range_lu: f32,
    pub sample_rate: u32,
    pub bit_depth: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum QualityLevel {
    Info,
    Warning,
    Critical,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QualityCheck {
    pub level: QualityLevel,
    pub code: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportReport {
    pub track_id: TrackId,
    pub output_path: String,
    pub measured_lufs: f32,
    pub measured_true_peak_dbtp: f32,
    pub measured_dynamic_range_lu: f32,
    pub source_format: String,
    pub destination_format: String,
    pub sample_rate: u32,
    pub bit_depth: u16,
    pub checks: Vec<QualityCheck>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ProjectMode {
    Track,
    Album,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectState {
    pub schema_version: u32,
    pub mode: ProjectMode,
    pub tracks: Vec<ImportedTrack>,
    pub track_order: Vec<TrackId>,
    pub track_settings: HashMap<String, MasteringSettings>,
    pub album_intent: Option<MasteringSettings>,
    /// Set of track IDs whose per-track `track_settings` should override the
    /// shared `album_intent` during album rendering. Defaulted so older
    /// sessions (without this field) deserialize cleanly as "no overrides."
    #[serde(default)]
    pub track_override_album: Vec<TrackId>,
    pub last_saved_iso: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PresetKind {
    Track,
    Album,
    Shared,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserPreset {
    pub id: String,
    pub name: String,
    pub kind: PresetKind,
    pub settings: MasteringSettings,
    pub created_at_iso: String,
}

#[derive(Debug, Error, Clone)]
pub enum CommandError {
    #[error("not implemented: {0}")]
    NotImplemented(String),
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("decode error: {0}")]
    Decode(String),
    #[error("render error: {0}")]
    Render(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("{0}")]
    Other(String),
}

impl Serialize for CommandError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type CommandResult<T> = Result<T, CommandError>;

/// Deterministic ISO 8601 string used as a frozen timestamp in TEST
/// fixtures that construct `AnalysisResult` / `RenderJob` / `UserPreset`
/// by hand. Production code paths use `now_iso()` instead so reports
/// and manifests record real current timestamps.
pub const ISO_PLACEHOLDER: &str = "2026-05-11T12:00:00Z";

/// Current UTC timestamp formatted as RFC 3339 / ISO 8601. Used by every
/// production call site that needs a "when did this happen" string —
/// analysis `measured_at_iso`, render `started_at_iso` /
/// `rendered_at_iso`, preset `created_at_iso`, album manifest fields.
/// Prior to B4 these were all hardcoded to `ISO_PLACEHOLDER` (a frozen
/// `2026-05-11T12:00:00Z`), so reports couldn't distinguish between
/// two renders by timestamp. Tests still use `ISO_PLACEHOLDER` for
/// deterministic fixture construction.
pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlaybackTick {
    pub track_id: Option<TrackId>,
    pub position_sec: f64,
    pub is_playing: bool,
    pub is_loaded: bool,
    /// Post-output-gain peak across all channels since the last tick, in dBFS.
    /// `-120.0` is the silence sentinel (no signal in the window). Values
    /// above `-0.1` indicate clipping risk; values above `0.0` are clipping.
    /// Defaulted so older sessions/frontends parse cleanly as "no info."
    #[serde(default = "default_silence_dbfs")]
    pub peak_dbfs: f32,
    /// Phase 12.2 — gain reduction (in dB, negative) from the low band of the
    /// multiband compressor, captured as the maximum reduction seen in the
    /// last snapshot window. `-120.0` is the silence sentinel (no signal or
    /// compressor inactive in the window). Defaulted so older sessions and
    /// older frontends parse cleanly.
    #[serde(default = "default_silence_dbfs")]
    pub gr_low_db: f32,
    #[serde(default = "default_silence_dbfs")]
    pub gr_mid_db: f32,
    #[serde(default = "default_silence_dbfs")]
    pub gr_high_db: f32,
    /// Phase 12.2 P3 — live BS.1770 momentary LUFS (400 ms K-weighted
    /// sliding window). `-120.0` is the silence sentinel.
    #[serde(default = "default_silence_dbfs")]
    pub lufs_momentary: f32,
    /// Phase 12.2 P3+ — live BS.1770-4 integrated LUFS over the current
    /// playback session. Updates every 100 ms as new 400 ms blocks complete
    /// and pass the absolute (-70 LUFS) and relative (-10 LU) gates. Resets
    /// when a new playback starts. `-120.0` is the silence sentinel.
    /// Defaulted so older sessions/frontends parse cleanly as "no info."
    #[serde(default = "default_silence_dbfs")]
    pub lufs_integrated: f32,
    /// L4b — live FFT spectrum, log-binned to ~32 dB values. The
    /// frontend EQ panel renders these as a filled area under the
    /// response curve. Defaulted to an empty vec for back-compat;
    /// the frontend treats empty as "no spectrum data yet."
    #[serde(default)]
    pub spectrum_db: Vec<f32>,
}

fn default_silence_dbfs() -> f32 {
    -120.0
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct LoopRegion {
    pub start_sec: f64,
    pub end_sec: f64,
}

#[cfg(test)]
mod effective_settings_tests {
    //! Direct unit tests for `MasteringSettings::effective_*` accessors —
    //! the foundation behavior the live audio chain and the frontend
    //! display logic both depend on. Pre-existing tests cover the
    //! accessors indirectly through `delivery_profile_render.rs` (which
    //! renders and measures landing values), but no direct gate sat on
    //! the shadowing rule itself. These tests are that gate: if
    //! `effective_target_lufs` ever stops honoring the profile-over-
    //! advanced precedence, every downstream caller — landing block,
    //! ceiling-bounded math, LoudnessTarget readout — would drift in
    //! sync, and only render-time tests would catch it.
    use super::*;

    fn settings_with_profile_and_advanced(
        profile: DeliveryProfile,
        advanced: AdvancedSettings,
    ) -> MasteringSettings {
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
            advanced,
        }
    }

    // ----- effective_target_lufs ---------------------------------------

    /// Non-Custom profile wins over advanced.lufs_offset_db. This is the
    /// load-bearing rule that the LoudnessTarget UI readout must mirror —
    /// when the user picks a profile, the chain targets the profile's
    /// value regardless of any leftover advanced.lufs_offset_db.
    #[test]
    fn effective_target_lufs_profile_overrides_advanced() {
        let advanced = AdvancedSettings {
            lufs_offset_db: Some(-9.0), // user typed -9 but...
            ..Default::default()
        };
        let s = settings_with_profile_and_advanced(
            DeliveryProfile::StreamingUniversal, // ...profile says -14
            advanced,
        );
        assert_eq!(
            s.effective_target_lufs(),
            Some(-14.0),
            "non-Custom profile must shadow advanced.lufs_offset_db"
        );
    }

    /// Custom profile defers to advanced.lufs_offset_db. The fall-through
    /// path is what makes a manual LUFS target actually apply.
    #[test]
    fn effective_target_lufs_custom_uses_advanced_value() {
        let advanced = AdvancedSettings {
            lufs_offset_db: Some(-9.0),
            ..Default::default()
        };
        let s = settings_with_profile_and_advanced(DeliveryProfile::Custom, advanced);
        assert_eq!(
            s.effective_target_lufs(),
            Some(-9.0),
            "Custom profile must fall through to advanced.lufs_offset_db"
        );
    }

    /// Custom + None advanced → no target. Verifies the "no landing at
    /// all" path (the chain will skip the landing block entirely).
    #[test]
    fn effective_target_lufs_custom_with_none_advanced_returns_none() {
        let s = settings_with_profile_and_advanced(
            DeliveryProfile::Custom,
            AdvancedSettings::default(),
        );
        assert_eq!(s.effective_target_lufs(), None);
    }

    /// Every non-Custom profile reports a concrete target. Discriminator
    /// test that confirms each variant's `target_lufs()` is wired into
    /// the accessor.
    #[test]
    fn effective_target_lufs_known_for_every_non_custom_profile() {
        let cases: &[(DeliveryProfile, f32)] = &[
            (DeliveryProfile::StreamingUniversal, -14.0),
            (DeliveryProfile::AppleMusic, -16.0),
            (DeliveryProfile::Cd, -14.0),
            (DeliveryProfile::VinylPremaster, -18.0),
            (DeliveryProfile::LoudRock, -10.5),
            (DeliveryProfile::BroadcastEu, -23.0),
            (DeliveryProfile::BroadcastUs, -24.0),
        ];
        for (profile, expected) in cases {
            let s =
                settings_with_profile_and_advanced(profile.clone(), AdvancedSettings::default());
            assert_eq!(
                s.effective_target_lufs(),
                Some(*expected),
                "profile {:?} must report {} LUFS",
                profile,
                expected
            );
        }
    }

    // ----- effective_ceiling_dbtp --------------------------------------

    /// Profile ceiling shadows advanced.ceiling_dbtp.
    #[test]
    fn effective_ceiling_dbtp_profile_overrides_advanced() {
        let advanced = AdvancedSettings {
            ceiling_dbtp: Some(-0.5), // user typed -0.5 but...
            ..Default::default()
        };
        let s = settings_with_profile_and_advanced(
            DeliveryProfile::VinylPremaster, // ...profile says -3.0
            advanced,
        );
        assert!(
            (s.effective_ceiling_dbtp() - -3.0).abs() < 1.0e-6,
            "VinylPremaster ceiling -3.0 must shadow advanced -0.5; got {}",
            s.effective_ceiling_dbtp()
        );
    }

    /// Custom defers to advanced.ceiling_dbtp.
    #[test]
    fn effective_ceiling_dbtp_custom_uses_advanced_value() {
        let advanced = AdvancedSettings {
            ceiling_dbtp: Some(-0.5),
            ..Default::default()
        };
        let s = settings_with_profile_and_advanced(DeliveryProfile::Custom, advanced);
        assert!(
            (s.effective_ceiling_dbtp() - -0.5).abs() < 1.0e-6,
            "Custom ceiling falls through to advanced -0.5; got {}",
            s.effective_ceiling_dbtp()
        );
    }

    /// Custom + None advanced → -1.0 dBTP default. The unwrap_or fallback
    /// is what the chain assumes when nothing else is set; verifying it
    /// here keeps the default from silently shifting.
    #[test]
    fn effective_ceiling_dbtp_custom_with_none_advanced_defaults_to_minus_one() {
        let s = settings_with_profile_and_advanced(
            DeliveryProfile::Custom,
            AdvancedSettings::default(),
        );
        assert!(
            (s.effective_ceiling_dbtp() - -1.0).abs() < 1.0e-6,
            "default ceiling must be -1.0 dBTP; got {}",
            s.effective_ceiling_dbtp()
        );
    }

    // ----- effective_bit_depth -----------------------------------------

    /// Profile bit depth shadows advanced.bit_depth.
    #[test]
    fn effective_bit_depth_profile_overrides_advanced() {
        let advanced = AdvancedSettings {
            bit_depth: Some(32), // user picked 32 but...
            ..Default::default()
        };
        let s = settings_with_profile_and_advanced(
            DeliveryProfile::Cd, // ...CD profile says 16
            advanced,
        );
        assert_eq!(
            s.effective_bit_depth(),
            16,
            "CD profile must shadow advanced 32-bit choice"
        );
    }

    /// Custom defers to advanced.bit_depth.
    #[test]
    fn effective_bit_depth_custom_uses_advanced_value() {
        let advanced = AdvancedSettings {
            bit_depth: Some(32),
            ..Default::default()
        };
        let s = settings_with_profile_and_advanced(DeliveryProfile::Custom, advanced);
        assert_eq!(s.effective_bit_depth(), 32);
    }

    /// Custom + None advanced → 24-bit default.
    #[test]
    fn effective_bit_depth_custom_with_none_advanced_defaults_to_24() {
        let s = settings_with_profile_and_advanced(
            DeliveryProfile::Custom,
            AdvancedSettings::default(),
        );
        assert_eq!(s.effective_bit_depth(), 24);
    }
}

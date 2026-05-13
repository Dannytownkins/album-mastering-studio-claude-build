// Hand-written TS types matching src-tauri/src/types.rs.
// Phase 1.2 will replace this file with auto-generated bindings via tauri-specta.

export type TrackId = string;

export interface ImportedTrack {
  id: TrackId;
  path: string;
  display_name: string;
  source_format: string;
  duration_seconds: number | null;
  sample_rate: number | null;
  channels: number | null;
}

export interface SpectralBalance {
  low: number;
  mid: number;
  high: number;
}

export type Preset =
  | { kind: "universal" }
  | { kind: "clarity" }
  | { kind: "tape" }
  | { kind: "spatial" }
  | { kind: "oomph" }
  | { kind: "warmth" }
  | { kind: "punch" }
  | { kind: "loud" }
  | { kind: "custom"; id: string };

export interface AdvancedSettings {
  lufs_offset_db: number | null;
  ceiling_dbtp: number | null;
  width: number | null;
  warmth: number | null;
  presence_air: number | null;
  compression_density: number | null;
  // Phase 12.2 per-band compressor overrides. `null` = let the macro
  // (compression_density) drive that band's threshold; per-band ratio/
  // attack/release fall back to fixed musical defaults in the backend.
  compression_low_threshold_db: number | null;
  compression_low_ratio: number | null;
  compression_low_attack_ms: number | null;
  compression_low_release_ms: number | null;
  compression_mid_threshold_db: number | null;
  compression_mid_ratio: number | null;
  compression_mid_attack_ms: number | null;
  compression_mid_release_ms: number | null;
  compression_high_threshold_db: number | null;
  compression_high_ratio: number | null;
  compression_high_attack_ms: number | null;
  compression_high_release_ms: number | null;
  /// `null` or `true` = linked stereo (max-of-|L|,|R| envelope per band).
  /// `false` = independent L/R envelopes per band.
  compression_link_stereo: boolean | null;
  bit_depth: number | null;
  target_sample_rate: number | null;
}

/// Phase A3 — delivery profile presets. Mirrors `types::DeliveryProfile`
/// in the Rust crate. Serializes as kebab-case so it round-trips through
/// the .ams.json schema.
export type DeliveryProfile =
  | "streaming-universal"
  | "apple-music"
  | "cd"
  | "vinyl-premaster"
  | "loud-rock"
  | "broadcast-eu"
  | "broadcast-us"
  | "custom";

export const DELIVERY_PROFILE_DISPLAY: Record<DeliveryProfile, string> = {
  "streaming-universal":
    "Streaming (Spotify / YouTube / Tidal / Amazon)",
  "apple-music": "Apple Music",
  "cd": "CD (16-bit)",
  "vinyl-premaster": "Vinyl Premaster",
  "loud-rock": "Loud Rock / Aggressive",
  "broadcast-eu": "Broadcast EU (EBU R128)",
  "broadcast-us": "Broadcast US (ATSC A/85)",
  "custom": "Custom",
};

/// Target LUFS for each non-`custom` profile. Used by the UI to display
/// the implied value next to the profile name, and to detect "user
/// deviated from profile, flip to Custom" in the settings editor.
export const DELIVERY_PROFILE_TARGET_LUFS: Record<DeliveryProfile, number | null> = {
  "streaming-universal": -14.0,
  "apple-music": -16.0,
  "cd": -14.0,
  "vinyl-premaster": -18.0,
  "loud-rock": -10.5,
  "broadcast-eu": -23.0,
  "broadcast-us": -24.0,
  "custom": null,
};

/// Phase B — Album Master mode types. Mirror of the Rust types in
/// types.rs; the four AlbumArcKind values map to the same 6-point
/// curves the Rust runtime cosine-resamples to actual track count.
export type AlbumArcKind =
  | "cinematic"
  | "afterhours"
  | "club-peak"
  | "fever-dream";

export const ALBUM_ARC_DISPLAY: Record<AlbumArcKind, string> = {
  "cinematic": "Cinematic",
  "afterhours": "Afterhours",
  "club-peak": "Club Peak",
  "fever-dream": "Fever Dream",
};

export type AlbumArc =
  | { kind: "preset"; preset: AlbumArcKind }
  | { kind: "custom"; lufs_offsets: number[] };

export type TransitionKind = "direct" | "gap";

export interface TransitionSpec {
  kind: TransitionKind;
  /// Clamped to [0, 5] seconds at the planner / render layer. Ignored
  /// for kind = "direct".
  duration_seconds: number;
}

export interface AlbumTrackEntry {
  track_id: TrackId;
  position: number;
  role: TrackRole;
  role_locked?: boolean;
  arc_lufs_offset_db: number;
  intensity_scale: number;
}

export interface AlbumPlan {
  title: string;
  arc: AlbumArc;
  tracks: AlbumTrackEntry[];
  transitions: TransitionSpec[];
  intensity: number;
}

export interface MasteringSettings {
  preset: Preset;
  intensity: number;
  eq_low_db: number;
  /// Phase A2 — user offset on top of the preset's low-mid baseline
  /// (400 Hz peaking @ Q=0.9). 0 = use preset value as-is.
  eq_low_mid_db: number;
  eq_mid_db: number;
  eq_high_db: number;
  volume_match: boolean;
  /// Pre-chain gain in dB. Negative reduces the source level before the
  /// preset / EQ / limiter sees it. Default 0.
  input_gain_db: number;
  /// Post-limiter output trim in dB. Default 0. Boosting may reintroduce
  /// peaks above the ceiling.
  output_gain_db: number;
  /// Phase A3 — delivery profile preset. Shadows lufs_offset_db /
  /// ceiling_dbtp / bit_depth at render time when non-`custom`.
  delivery_profile: DeliveryProfile;
  /// Phase B — album-mode plan. `null` for Track Master mode.
  album?: AlbumPlan | null;
  advanced: AdvancedSettings;
}

export type TrackRole =
  | "opener"
  | "closer"
  | "single"
  | "ballad"
  | "interlude"
  | "album_track";

export type TrackCharacter =
  | "bright"
  | "dark"
  | "dense"
  | "sparse"
  | "balanced";

export type InferenceConfidence = "strong" | "moderate" | "unsure";

export interface SpectralBalance6 {
  sub: number;
  low: number;
  low_mid: number;
  mid: number;
  presence: number;
  air: number;
}

export interface AnalysisResult {
  track_id: TrackId;
  lufs_integrated: number;
  lufs_short_term_max: number;
  true_peak_dbtp: number;
  dynamic_range_lu: number;
  spectral_balance: SpectralBalance;
  transient_density: number;
  stereo_width: number;
  recommended_universal: MasteringSettings;
  measured_at_iso: string;
  inferred_role?: TrackRole | null;
  role_confidence?: InferenceConfidence | null;
  inferred_character?: TrackCharacter | null;
  character_confidence?: InferenceConfidence | null;
  /// Phase A5: richer analysis measurements (optional — older
  /// serialized analyses lack them and `null` falls through).
  spectral_balance_6band?: SpectralBalance6 | null;
  transient_flux?: number | null;
  stereo_correlation?: number | null;
  dynamic_range_p95_p10_db?: number | null;
  lufs_short_term_max_3s?: number | null;
  energy_density_score?: number | null;
}

export interface WaveformPeaks {
  track_id: TrackId;
  channels: number[][];
  samples_per_pixel: number;
  total_samples: number;
  sample_rate: number;
}

export type PlaybackKind = "source" | "master";

export interface PlaybackHandle {
  id: string;
  track_id: TrackId;
  kind: PlaybackKind;
  duration_seconds: number;
}

export interface AbPreview {
  track_id: TrackId;
  source_handle: PlaybackHandle;
  master_handle: PlaybackHandle;
  volume_match_offset_db: number;
}

export type RenderKind = "preview" | "master" | "album";

export type JobStatus =
  | { status: "pending" }
  | { status: "running" }
  | { status: "done" }
  | { status: "failed"; reason: string }
  | { status: "cancelled" };

export interface RenderJob {
  id: string;
  kind: RenderKind;
  target_tracks: TrackId[];
  status: JobStatus;
  progress: number;
  started_at_iso: string;
  output_paths: string[];
}

export type QualityLevel = "info" | "warning" | "critical";

export interface QualityCheck {
  level: QualityLevel;
  code: string;
  message: string;
}

export interface ExportReport {
  track_id: TrackId;
  output_path: string;
  measured_lufs: number;
  measured_true_peak_dbtp: number;
  measured_dynamic_range_lu: number;
  source_format: string;
  destination_format: string;
  sample_rate: number;
  bit_depth: number;
  checks: QualityCheck[];
}

export type ProjectMode = "track" | "album";

export interface ProjectState {
  schema_version: number;
  mode: ProjectMode;
  tracks: ImportedTrack[];
  track_order: TrackId[];
  track_settings: Record<string, MasteringSettings>;
  album_intent: MasteringSettings | null;
  track_override_album?: TrackId[];
  last_saved_iso: string | null;
}

export type PresetKind = "track" | "album" | "shared";

export interface UserPreset {
  id: string;
  name: string;
  kind: PresetKind;
  settings: MasteringSettings;
  created_at_iso: string;
}

// Rust's CommandError is serialized to a string via Display, so on the JS side it arrives as a string.
export type CommandError = string;

export interface PlaybackTick {
  track_id: TrackId | null;
  position_sec: number;
  is_playing: boolean;
  is_loaded: boolean;
  /// Post-output-gain peak across all channels since the last tick, in dBFS.
  /// `-120` is the silence sentinel (no signal seen in the window). Values
  /// above `-0.1` indicate clipping risk; values above `0` are clipping.
  peak_dbfs: number;
  /// Phase 12.2 — per-band compressor gain reduction in dB (negative).
  /// `-120` is the silence sentinel; values like -2.3 mean 2.3 dB of GR.
  gr_low_db: number;
  gr_mid_db: number;
  gr_high_db: number;
  /// Phase 12.2 P3 — live BS.1770 momentary LUFS (400 ms K-weighted
  /// sliding window). `-120` is the silence sentinel.
  lufs_momentary: number;
  /// Phase 12.2 P3+ — live BS.1770-4 integrated LUFS over the current
  /// playback session. Updates every 100 ms; resets when a new playback
  /// starts. `-120` is the silence sentinel.
  lufs_integrated: number;
}

export interface LoopRegion {
  start_sec: number;
  end_sec: number;
}

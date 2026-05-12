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
  bit_depth: number | null;
  target_sample_rate: number | null;
}

export interface MasteringSettings {
  preset: Preset;
  intensity: number;
  eq_low_db: number;
  eq_mid_db: number;
  eq_high_db: number;
  volume_match: boolean;
  advanced: AdvancedSettings;
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
}

export interface LoopRegion {
  start_sec: number;
  end_sec: number;
}

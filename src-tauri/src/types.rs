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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MasteringSettings {
    pub preset: Preset,
    pub intensity: f32,
    pub eq_low_db: f32,
    pub eq_mid_db: f32,
    pub eq_high_db: f32,
    pub volume_match: bool,
    pub advanced: AdvancedSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AdvancedSettings {
    pub lufs_offset_db: Option<f32>,
    pub ceiling_dbtp: Option<f32>,
    pub width: Option<f32>,
    pub warmth: Option<f32>,
    pub presence_air: Option<f32>,
    pub compression_density: Option<f32>,
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
pub enum PlaybackKind {
    Source,
    Master,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlaybackHandle {
    pub id: String,
    pub track_id: TrackId,
    pub kind: PlaybackKind,
    pub duration_seconds: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AbPreview {
    pub track_id: TrackId,
    pub source_handle: PlaybackHandle,
    pub master_handle: PlaybackHandle,
    pub volume_match_offset_db: f32,
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

pub const ISO_PLACEHOLDER: &str = "2026-05-11T12:00:00Z";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlaybackTick {
    pub track_id: Option<TrackId>,
    pub position_sec: f64,
    pub is_playing: bool,
    pub is_loaded: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct LoopRegion {
    pub start_sec: f64,
    pub end_sec: f64,
}

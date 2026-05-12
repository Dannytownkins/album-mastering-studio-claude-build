import { invoke } from "@tauri-apps/api/core";
import type {
  AbPreview,
  AnalysisResult,
  ExportReport,
  ImportedTrack,
  MasteringSettings,
  PlaybackHandle,
  PresetKind,
  ProjectState,
  QualityCheck,
  RenderJob,
  TrackId,
  UserPreset,
  WaveformPeaks,
} from "../bindings";

export const api = {
  importTracks: (paths: string[]) =>
    invoke<ImportedTrack[]>("import_tracks", { paths }),

  analyzeTracks: (trackIds: TrackId[]) =>
    invoke<AnalysisResult[]>("analyze_tracks", { track_ids: trackIds }),

  renderTrackPreview: (trackId: TrackId, settings: MasteringSettings) =>
    invoke<RenderJob>("render_track_preview", {
      track_id: trackId,
      settings,
    }),

  renderTrackMaster: (trackId: TrackId, settings: MasteringSettings) =>
    invoke<RenderJob>("render_track_master", {
      track_id: trackId,
      settings,
    }),

  renderAlbumMaster: (
    trackIds: TrackId[],
    albumIntent: MasteringSettings,
    perTrackOverrides?: Record<string, MasteringSettings>,
  ) =>
    invoke<RenderJob>("render_album_master", {
      track_ids: trackIds,
      album_intent: albumIntent,
      per_track_overrides: perTrackOverrides ?? null,
    }),

  prepareSourcePlayback: (trackId: TrackId, trackPath: string) =>
    invoke<PlaybackHandle>("prepare_source_playback", {
      track_id: trackId,
      track_path: trackPath,
    }),

  prepareMasterPlayback: (
    trackId: TrackId,
    trackPath: string,
    settings: MasteringSettings,
  ) =>
    invoke<PlaybackHandle>("prepare_master_playback", {
      track_id: trackId,
      track_path: trackPath,
      settings,
    }),

  prepareAbPreview: (
    trackId: TrackId,
    trackPath: string,
    settings: MasteringSettings,
    volumeMatch: boolean,
  ) =>
    invoke<AbPreview>("prepare_ab_preview", {
      track_id: trackId,
      track_path: trackPath,
      settings,
      volume_match: volumeMatch,
    }),

  prepareWaveform: (
    trackId: TrackId,
    trackPath: string,
    targetPixels?: number,
  ) =>
    invoke<WaveformPeaks>("prepare_waveform", {
      track_id: trackId,
      track_path: trackPath,
      target_pixels: targetPixels ?? null,
    }),

  runExportChecks: (report: ExportReport) =>
    invoke<QualityCheck[]>("run_export_checks", { report }),

  openOutput: (outputPath: string) =>
    invoke<null>("open_output", { output_path: outputPath }),

  saveProject: (path: string, state: ProjectState) =>
    invoke<null>("save_project", { path, state }),

  autosaveSession: (state: ProjectState) =>
    invoke<null>("autosave_session", { state }),

  loadRecentSession: () =>
    invoke<ProjectState | null>("load_recent_session"),

  saveUserPreset: (
    name: string,
    kind: PresetKind,
    settings: MasteringSettings,
  ) =>
    invoke<UserPreset>("save_user_preset", { name, kind, settings }),

  listUserPresets: () => invoke<UserPreset[]>("list_user_presets"),
};

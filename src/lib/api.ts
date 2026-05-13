import { invoke, listen } from "./tauri-runtime";
import type { UnlistenFn } from "@tauri-apps/api/event";
import type {
  AbPreview,
  AnalysisResult,
  ExportReport,
  ImportedTrack,
  LoopRegion,
  MasteringSettings,
  PlaybackHandle,
  PlaybackTick,
  PresetKind,
  ProjectState,
  QualityCheck,
  RenderJob,
  TrackId,
  UserPreset,
  WaveformPeaks,
} from "../bindings";

// Tauri 2 auto-converts camelCase invoke arg keys to snake_case Rust parameter
// names. So `trackId` here lands as `track_id` in the Rust handler signature.
// Sending snake_case keys directly does NOT work — Tauri's command arg parser
// rejects them with "missing required key <camelCaseName>". Phase 11.3 fix.

export const api = {
  importTracks: (paths: string[]) =>
    invoke<ImportedTrack[]>("import_tracks", { paths }),

  analyzeTracks: (tracks: Array<{ id: TrackId; path: string }>) =>
    invoke<AnalysisResult[]>("analyze_tracks", { tracks }),

  renderTrackPreview: (
    trackId: TrackId,
    trackPath: string,
    settings: MasteringSettings,
  ) =>
    invoke<RenderJob>("render_track_preview", {
      trackId,
      trackPath,
      settings,
    }),

  renderTrackMaster: (
    trackId: TrackId,
    trackPath: string,
    settings: MasteringSettings,
  ) =>
    invoke<RenderJob>("render_track_master", {
      trackId,
      trackPath,
      settings,
    }),

  renderAlbumMaster: (
    tracks: Array<{ id: TrackId; path: string }>,
    albumIntent: MasteringSettings,
    perTrackOverrides?: Record<string, MasteringSettings>,
  ) =>
    invoke<RenderJob>("render_album_master", {
      request: {
        tracks,
        album_intent: albumIntent,
        per_track_overrides: perTrackOverrides ?? null,
      },
    }),

  prepareSourcePlayback: (trackId: TrackId, trackPath: string) =>
    invoke<PlaybackHandle>("prepare_source_playback", {
      trackId,
      trackPath,
    }),

  prepareMasterPlayback: (
    trackId: TrackId,
    trackPath: string,
    settings: MasteringSettings,
  ) =>
    invoke<PlaybackHandle>("prepare_master_playback", {
      trackId,
      trackPath,
      settings,
    }),

  prepareAbPreview: (
    trackId: TrackId,
    trackPath: string,
    settings: MasteringSettings,
    volumeMatch: boolean,
  ) =>
    invoke<AbPreview>("prepare_ab_preview", {
      trackId,
      trackPath,
      settings,
      volumeMatch,
    }),

  prepareWaveform: (
    trackId: TrackId,
    trackPath: string,
    targetPixels?: number,
  ) =>
    invoke<WaveformPeaks>("prepare_waveform", {
      trackId,
      trackPath,
      targetPixels: targetPixels ?? null,
    }),

  runExportChecks: (
    report: ExportReport,
    sourceAnalysis?: AnalysisResult | null,
    settings?: MasteringSettings | null,
  ) =>
    invoke<QualityCheck[]>("run_export_checks", {
      report,
      sourceAnalysis: sourceAnalysis ?? null,
      settings: settings ?? null,
    }),

  openOutput: (outputPath: string) =>
    invoke<null>("open_output", { outputPath }),

  saveProject: (path: string, state: ProjectState) =>
    invoke<null>("save_project", { path, state }),

  autosaveSession: (state: ProjectState) =>
    invoke<null>("autosave_session", { state }),

  loadRecentSession: () =>
    invoke<ProjectState | null>("load_recent_session"),

  loadProject: (path: string) =>
    invoke<ProjectState>("load_project", { path }),

  saveUserPreset: (
    name: string,
    kind: PresetKind,
    settings: MasteringSettings,
  ) =>
    invoke<UserPreset>("save_user_preset", { name, kind, settings }),

  listUserPresets: () => invoke<UserPreset[]>("list_user_presets"),

  deleteUserPreset: (id: string) =>
    invoke<null>("delete_user_preset", { id }),

  playTrack: (
    trackId: TrackId,
    trackPath: string,
    startPositionSec?: number,
  ) =>
    invoke<null>("play_track", {
      trackId,
      trackPath,
      startPositionSec: startPositionSec ?? null,
    }),

  playMaster: (
    trackId: TrackId,
    trackPath: string,
    settings: MasteringSettings,
    startPositionSec?: number,
  ) =>
    invoke<null>("play_master", {
      trackId,
      trackPath,
      settings,
      startPositionSec: startPositionSec ?? null,
    }),

  updateChain: (settings: MasteringSettings) =>
    invoke<null>("update_chain", { settings }),

  pausePlayback: () => invoke<null>("pause_playback"),
  resumePlayback: () => invoke<null>("resume_playback"),
  stopPlayback: () => invoke<null>("stop_playback"),
  seekPlayback: (positionSec: number) =>
    invoke<null>("seek_playback", { positionSec }),
  setLoopRegion: (region: LoopRegion | null) =>
    invoke<null>("set_loop_region", { region }),
};

export function onPlaybackTick(
  handler: (tick: PlaybackTick) => void,
): Promise<UnlistenFn> {
  return listen<PlaybackTick>("playback:tick", (event) => handler(event.payload));
}

export interface RenderProgressEvent {
  track_id: TrackId;
  kind: "preview" | "master" | "album";
  fraction: number;
}

export function onRenderProgress(
  handler: (event: RenderProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<RenderProgressEvent>("render:progress", (event) =>
    handler(event.payload),
  );
}

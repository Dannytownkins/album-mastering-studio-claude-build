import { invoke, listen } from "./tauri-runtime";
import type { UnlistenFn } from "@tauri-apps/api/event";
import type {
  AlbumArc,
  AlbumPlan,
  AnalysisResult,
  ExportReport,
  ImportedTrack,
  LoopRegion,
  MasteringSettings,
  PlaybackTick,
  PresetKind,
  ProjectState,
  QualityCheck,
  RenderJob,
  TrackId,
  UserPreset,
  WaveformPeaks,
} from "../bindings";

/// Phase B: render_album_plan return shape.
export interface AlbumTrackRenderRecord {
  track_id: TrackId;
  position: number;
  output_path: string;
  measured_lufs: number;
}

export interface AlbumRenderReport {
  album_wav_path: string;
  manifest_path: string;
  tracks: AlbumTrackRenderRecord[];
}

export interface AlbumTrackRenderInput {
  track_id: TrackId;
  source_path: string;
  settings: MasteringSettings;
}

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
    outputPath?: string,
  ) =>
    invoke<RenderJob>("render_track_master", {
      trackId,
      trackPath,
      settings,
      outputPath: outputPath ?? null,
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
    previewLufsLanding = true,
  ) =>
    invoke<null>("play_master", {
      trackId,
      trackPath,
      settings,
      startPositionSec: startPositionSec ?? null,
      previewLufsLanding,
    }),

  updateChain: (settings: MasteringSettings, previewLufsLanding = true) =>
    invoke<null>("update_chain", { settings, previewLufsLanding }),

  /// Prewarm the backend decode cache for a track. Fire-and-forget
  /// from track-select / track-import handlers so the PCM is ready
  /// by the time the user clicks Mastered (eliminates the 1-2 s
  /// freeze on first click for long WAVs). Idempotent; safe to
  /// call repeatedly on the same track.
  prewarmDecode: (trackPath: string) =>
    invoke<null>("prewarm_decode", { trackPath }),

  pausePlayback: () => invoke<null>("pause_playback"),
  resumePlayback: () => invoke<null>("resume_playback"),
  stopPlayback: () => invoke<null>("stop_playback"),
  seekPlayback: (positionSec: number) =>
    invoke<null>("seek_playback", { positionSec }),
  setLoopRegion: (region: LoopRegion | null) =>
    invoke<null>("set_loop_region", { region }),

  // Phase B — album-mode planning + render.
  planAlbum: (
    title: string,
    analyses: AnalysisResult[],
    durations: number[],
    arc: AlbumArc,
    intensity: number,
  ) =>
    invoke<AlbumPlan>("plan_album", {
      request: { title, analyses, durations, arc, intensity },
    }),

  renderAlbumPlan: (
    plan: AlbumPlan,
    tracks: AlbumTrackRenderInput[],
    outputDir?: string,
  ) =>
    invoke<AlbumRenderReport>("render_album_plan", {
      request: { plan, tracks },
      outputDir: outputDir ?? null,
    }),
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

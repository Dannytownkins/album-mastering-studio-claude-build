// Browser-preview mocks for the Tauri backend.
//
// Only loaded by `tauri-runtime.ts` when `window.__TAURI_INTERNALS__` is
// missing — i.e. when the app is being viewed in a plain browser (Vite
// dev server at localhost:1420, agent-browser screenshots, etc).
// In the real Tauri WebView these mocks are never imported.
//
// The mock seeds a "preview project" with one realistic-looking track so
// the loaded-track UI renders immediately on boot. Playback ticks emit at
// ~50 Hz with bouncing meter values so the live readouts animate.

import type {
  AbPreview,
  AnalysisResult,
  ImportedTrack,
  MasteringSettings,
  PlaybackHandle,
  PlaybackTick,
  ProjectState,
  TrackId,
  UserPreset,
  WaveformPeaks,
} from "../bindings";
import type { UnlistenFn } from "@tauri-apps/api/event";

const PREVIEW_TRACK_ID = "preview-track-1";
const PREVIEW_DURATION = 245;

const DEFAULT_ADVANCED: MasteringSettings["advanced"] = {
  lufs_offset_db: null,
  ceiling_dbtp: null,
  width: null,
  warmth: null,
  presence_air: null,
  compression_density: null,
  compression_low_threshold_db: null,
  compression_low_ratio: null,
  compression_low_attack_ms: null,
  compression_low_release_ms: null,
  compression_mid_threshold_db: null,
  compression_mid_ratio: null,
  compression_mid_attack_ms: null,
  compression_mid_release_ms: null,
  compression_high_threshold_db: null,
  compression_high_ratio: null,
  compression_high_attack_ms: null,
  compression_high_release_ms: null,
  compression_link_stereo: null,
  bit_depth: null,
  target_sample_rate: null,
};

const DEFAULT_SETTINGS: MasteringSettings = {
  preset: { kind: "universal" },
  intensity: 0.5,
  eq_low_db: 0,
  eq_low_mid_db: 0,
  eq_mid_db: 0,
  eq_high_db: 0,
  volume_match: false,
  input_gain_db: 0,
  output_gain_db: 0,
  delivery_profile: "streaming-universal",
  advanced: DEFAULT_ADVANCED,
};

const PREVIEW_TRACK: ImportedTrack = {
  id: PREVIEW_TRACK_ID,
  path: "/preview/sample.wav",
  display_name: "Preview Track (browser preview)",
  source_format: "wav",
  duration_seconds: PREVIEW_DURATION,
  sample_rate: 48000,
  channels: 2,
};

const PREVIEW_ANALYSIS: AnalysisResult = {
  track_id: PREVIEW_TRACK_ID,
  lufs_integrated: -14.6,
  lufs_short_term_max: -10.2,
  true_peak_dbtp: -4.0,
  dynamic_range_lu: 5.2,
  spectral_balance: { low: 0.32, mid: 0.42, high: 0.26 },
  transient_density: 0.55,
  stereo_width: 1.0,
  recommended_universal: DEFAULT_SETTINGS,
  measured_at_iso: new Date().toISOString(),
  inferred_role: null,
  role_confidence: null,
  inferred_character: null,
  character_confidence: null,
};

const PREVIEW_PROJECT: ProjectState = {
  schema_version: 1,
  mode: "track",
  tracks: [PREVIEW_TRACK],
  track_order: [PREVIEW_TRACK_ID],
  track_settings: { [PREVIEW_TRACK_ID]: DEFAULT_SETTINGS },
  album_intent: null,
  track_override_album: [],
  last_saved_iso: null,
};

// Synthesize a stereo waveform that visually reads like a music track —
// a couple of dynamic envelope swells over the duration, with stereo
// asymmetry so the two channels don't look identical.
function syntheticWaveform(targetPixels: number): WaveformPeaks {
  const px = Math.max(64, Math.min(4096, targetPixels));
  const peaks: number[][] = [[], []];
  for (let i = 0; i < px; i++) {
    const t = i / px;
    // Envelope: slow swell over the track + a quieter intro + a chorus bump.
    const envelope =
      0.55 +
      0.25 * Math.sin(t * Math.PI * 2 - Math.PI / 2) +
      0.15 * Math.sin(t * Math.PI * 6) +
      0.05 * Math.sin(t * Math.PI * 27);
    const ampL = Math.max(0.05, envelope) * (0.9 + 0.1 * Math.sin(t * 41));
    const ampR = Math.max(0.05, envelope) * (0.9 + 0.1 * Math.cos(t * 37));
    peaks[0].push(Math.min(0.98, ampL));
    peaks[1].push(Math.min(0.98, ampR));
  }
  return {
    track_id: PREVIEW_TRACK_ID,
    channels: peaks,
    samples_per_pixel: Math.floor((PREVIEW_DURATION * 48000) / px),
    total_samples: PREVIEW_DURATION * 48000,
    sample_rate: 48000,
  };
}

// Mock playback ticker state. The browser-preview meters animate when the
// user "plays" so the LIVE pill, gradient bars, and live integrated LUFS
// have something visually interesting to do.
let mockPlaying = false;
let mockPosition = 0;
const TICK_HZ = 20;
const TICK_INTERVAL_MS = Math.floor(1000 / TICK_HZ);

export async function mockInvoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  switch (cmd) {
    case "load_recent_session":
      return PREVIEW_PROJECT as unknown as T;
    case "load_project":
      return PREVIEW_PROJECT as unknown as T;
    case "autosave_session":
    case "save_project":
      return null as unknown as T;

    case "import_tracks": {
      const paths = (args?.paths as string[]) ?? [];
      const imported: ImportedTrack[] = paths.map((p, i) => ({
        ...PREVIEW_TRACK,
        id: `${PREVIEW_TRACK_ID}-${i}`,
        path: p,
        display_name: p.split(/[\\/]/).pop() ?? `Track ${i + 1}`,
      }));
      return imported as unknown as T;
    }

    case "analyze_tracks": {
      const tracks =
        (args?.tracks as Array<{ id: TrackId; path: string }>) ?? [];
      const results: AnalysisResult[] = tracks.map((t) => ({
        ...PREVIEW_ANALYSIS,
        track_id: t.id,
      }));
      return results as unknown as T;
    }

    case "prepare_waveform": {
      const pixels = (args?.targetPixels as number | null) ?? 1600;
      return syntheticWaveform(pixels) as unknown as T;
    }

    case "prepare_source_playback":
    case "prepare_master_playback": {
      const handle: PlaybackHandle = {
        id: `mock-${Date.now()}`,
        track_id: (args?.trackId as TrackId) ?? PREVIEW_TRACK_ID,
        kind: cmd === "prepare_master_playback" ? "master" : "source",
        duration_seconds: PREVIEW_DURATION,
      };
      return handle as unknown as T;
    }

    case "prepare_ab_preview": {
      const preview: AbPreview = {
        track_id: (args?.trackId as TrackId) ?? PREVIEW_TRACK_ID,
        source_handle: {
          id: `mock-source-${Date.now()}`,
          track_id: (args?.trackId as TrackId) ?? PREVIEW_TRACK_ID,
          kind: "source",
          duration_seconds: PREVIEW_DURATION,
        },
        master_handle: {
          id: `mock-master-${Date.now()}`,
          track_id: (args?.trackId as TrackId) ?? PREVIEW_TRACK_ID,
          kind: "master",
          duration_seconds: PREVIEW_DURATION,
        },
        volume_match_offset_db: 0,
      };
      return preview as unknown as T;
    }

    case "play_track":
    case "play_master":
    case "resume_playback":
      mockPlaying = true;
      return null as unknown as T;

    case "pause_playback":
      mockPlaying = false;
      return null as unknown as T;

    case "stop_playback":
      mockPlaying = false;
      mockPosition = 0;
      return null as unknown as T;

    case "seek_playback":
      mockPosition = (args?.positionSec as number) ?? 0;
      return null as unknown as T;

    case "set_loop_region":
    case "update_chain":
    case "open_output":
    case "delete_user_preset":
      return null as unknown as T;

    case "list_user_presets":
      return [] as unknown as T;

    case "save_user_preset": {
      const preset: UserPreset = {
        id: `mock-preset-${Date.now()}`,
        name: (args?.name as string) ?? "Preview Preset",
        kind: (args?.kind as UserPreset["kind"]) ?? "track",
        settings: (args?.settings as MasteringSettings) ?? DEFAULT_SETTINGS,
        created_at_iso: new Date().toISOString(),
      };
      return preset as unknown as T;
    }

    case "plan_album": {
      // Mock: return a single-track plan from whatever the caller passed.
      const req = (args?.request as Record<string, unknown>) ?? {};
      const analyses = (req.analyses as Array<{ track_id: string }> | undefined) ?? [];
      const tracks = analyses.map((a, i) => ({
        track_id: a.track_id,
        position: i + 1,
        role: i === 0
          ? "opener"
          : i === analyses.length - 1
            ? "closer"
            : "album_track",
        role_locked: false,
        arc_lufs_offset_db: 0,
        intensity_scale: 1.0,
      }));
      return {
        title: req.title ?? "Mock Album",
        arc: req.arc ?? { kind: "preset", preset: "cinematic" },
        tracks,
        transitions: Array(Math.max(0, analyses.length - 1)).fill({
          kind: "direct",
          duration_seconds: 0,
        }),
        intensity: req.intensity ?? 1.0,
      } as unknown as T;
    }

    case "render_album_plan":
      return {
        album_wav_path: "/preview/album.wav",
        manifest_path: "/preview/manifest.json",
        tracks: [],
      } as unknown as T;

    case "render_track_preview":
    case "render_track_master":
    case "render_album_master":
      return {
        id: `mock-render-${Date.now()}`,
        kind: "preview",
        target_tracks: [PREVIEW_TRACK_ID],
        status: { status: "done" },
        progress: 1.0,
        started_at_iso: new Date().toISOString(),
        output_paths: ["/preview/output.wav"],
      } as unknown as T;

    case "run_export_checks":
      return [] as unknown as T;

    default:
      console.warn(`[preview-mock] unhandled command: ${cmd}`, args);
      return null as unknown as T;
  }
}

export async function mockListen<T>(
  channel: string,
  handler: (event: { payload: T }) => void,
): Promise<UnlistenFn> {
  if (channel === "playback:tick") {
    const interval = setInterval(() => {
      if (mockPlaying) {
        mockPosition = (mockPosition + 1 / TICK_HZ) % PREVIEW_DURATION;
      }
      // Animate live readouts so the meter has something to do. Sinusoidal
      // bounce around a plausible mastered-track level. When paused, all
      // live signals collapse to the silence sentinel (-120) so the UI
      // renders the same "idle" state the real backend would emit.
      const t = mockPosition;
      const peakDb = mockPlaying ? -3.5 + 2.5 * Math.sin(t * 7) : -120;
      const lufsMomentary = mockPlaying ? -11 + 3 * Math.sin(t * 4) : -120;
      const lufsIntegrated = mockPlaying ? -14.2 + 0.4 * Math.sin(t * 0.6) : -120;
      const tick: PlaybackTick = {
        track_id: PREVIEW_TRACK_ID,
        position_sec: mockPosition,
        is_playing: mockPlaying,
        is_loaded: true,
        peak_dbfs: peakDb,
        gr_low_db: mockPlaying ? -1.2 + 0.8 * Math.sin(t * 3) : -120,
        gr_mid_db: mockPlaying ? -2.3 + 1.1 * Math.sin(t * 5 + 1) : -120,
        gr_high_db: mockPlaying ? -0.6 + 0.4 * Math.sin(t * 9 + 2) : -120,
        lufs_momentary: lufsMomentary,
        lufs_integrated: lufsIntegrated,
      };
      handler({ payload: tick as unknown as T });
    }, TICK_INTERVAL_MS);
    return () => clearInterval(interval);
  }
  // Unknown channel — return a no-op unlisten.
  console.warn(`[preview-mock] unhandled listen channel: ${channel}`);
  return () => {};
}

export async function mockOpen(
  opts?: { multiple?: boolean },
): Promise<string | string[] | null> {
  // Browser-preview can't access the OS filesystem. Returning null mimics
  // "user cancelled the dialog" so error paths render correctly.
  console.info("[preview-mock] open() returned null (cancelled)", opts);
  return null;
}

export async function mockSave(): Promise<string | null> {
  console.info("[preview-mock] save() returned null (cancelled)");
  return null;
}

export function mockWebview(): {
  onDragDropEvent: (
    handler: (event: { payload: unknown }) => void,
  ) => Promise<UnlistenFn>;
} {
  return {
    onDragDropEvent: async () => () => {},
  };
}

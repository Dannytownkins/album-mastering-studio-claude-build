import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  vi,
  type Mock,
} from "vitest";

import type {
  AnalysisResult,
  ImportedTrack,
  MasteringSettings,
  ProjectState,
  RenderJob,
  WaveformPeaks,
} from "../bindings";
import { useTrackMaster } from "./useTrackMaster";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean })
  .IS_REACT_ACT_ENVIRONMENT = true;

const mocks = vi.hoisted(() => {
  const api = {
    importTracks: vi.fn(),
    analyzeTracks: vi.fn(),
    renderTrackPreview: vi.fn(),
    renderTrackMaster: vi.fn(),
    renderAlbumMaster: vi.fn(),
    prepareWaveform: vi.fn(),
    runExportChecks: vi.fn(),
    openOutput: vi.fn(),
    saveProject: vi.fn(),
    autosaveSession: vi.fn(),
    loadRecentSession: vi.fn(),
    loadProject: vi.fn(),
    saveUserPreset: vi.fn(),
    listUserPresets: vi.fn(),
    deleteUserPreset: vi.fn(),
    playTrack: vi.fn(),
    playMaster: vi.fn(),
    updateChain: vi.fn(),
    prewarmDecode: vi.fn(),
    pausePlayback: vi.fn(),
    resumePlayback: vi.fn(),
    stopPlayback: vi.fn(),
    seekPlayback: vi.fn(),
    setLoopRegion: vi.fn(),
    planAlbum: vi.fn(),
    renderAlbumPlan: vi.fn(),
  };
  return {
    api,
    onPlaybackTick: vi.fn(),
    onRenderProgress: vi.fn(),
    open: vi.fn(),
    save: vi.fn(),
    onDragDropEvent: vi.fn(),
  };
});

vi.mock("../lib/api", () => ({
  api: mocks.api,
  onPlaybackTick: mocks.onPlaybackTick,
  onRenderProgress: mocks.onRenderProgress,
}));

vi.mock("../lib/tauri-runtime", () => ({
  open: mocks.open,
  save: mocks.save,
  getCurrentWebview: () => ({
    onDragDropEvent: mocks.onDragDropEvent,
  }),
}));

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
  advanced: {
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
  },
};

function makeTrack(id: string, path: string): ImportedTrack {
  return {
    id,
    path,
    display_name: `${id}.wav`,
    source_format: "wav",
    duration_seconds: 10,
    sample_rate: 44_100,
    channels: 2,
  };
}

function makeProjectState(track: ImportedTrack): ProjectState {
  return {
    schema_version: 1,
    mode: "track",
    tracks: [track],
    track_order: [track.id],
    track_settings: { [track.id]: DEFAULT_SETTINGS },
    album_intent: DEFAULT_SETTINGS,
    track_override_album: [],
    last_saved_iso: "2026-05-17T00:00:00.000Z",
  };
}

function makeWaveform(trackId: string): WaveformPeaks {
  return {
    track_id: trackId,
    channels: [[], []],
    samples_per_pixel: 512,
    total_samples: 0,
    sample_rate: 44_100,
  };
}

function makeAnalysis(trackId: string): AnalysisResult {
  return {
    track_id: trackId,
    lufs_integrated: -14,
    lufs_short_term_max: -10,
    true_peak_dbtp: -1,
    dynamic_range_lu: 8,
    spectral_balance: { low: 0.33, mid: 0.34, high: 0.33 },
    transient_density: 0.5,
    stereo_width: 0.5,
    recommended_universal: DEFAULT_SETTINGS,
    measured_at_iso: "2026-05-17T00:00:00.000Z",
    inferred_role: null,
    role_confidence: null,
    inferred_character: null,
    character_confidence: null,
    spectral_balance_6band: null,
    transient_flux: null,
    stereo_correlation: null,
    dynamic_range_p95_p10_db: null,
    lufs_short_term_max_3s: null,
    energy_density_score: null,
  };
}

function makeRenderJob(path: string): RenderJob {
  return {
    id: "render-1",
    kind: "master",
    target_tracks: ["export-1"],
    status: { status: "done" },
    progress: 1,
    started_at_iso: "2026-05-17T00:00:00.000Z",
    output_paths: [path],
    measurements: {
      lufs_integrated: -14,
      true_peak_dbtp: -1,
      dynamic_range_lu: 8,
      sample_rate: 44_100,
      bit_depth: 24,
    },
  };
}

function HookHarness({
  onRender,
}: {
  onRender: (value: ReturnType<typeof useTrackMaster>) => void;
}) {
  onRender(useTrackMaster());
  return null;
}

async function renderHookHarness(): Promise<{
  current: () => ReturnType<typeof useTrackMaster>;
  root: Root;
  container: HTMLDivElement;
}> {
  const container = document.createElement("div");
  document.body.appendChild(container);
  let current: ReturnType<typeof useTrackMaster> | null = null;
  const root = createRoot(container);
  await act(async () => {
    root.render(<HookHarness onRender={(value) => { current = value; }} />);
  });
  return {
    current: () => {
      if (current === null) throw new Error("hook has not rendered");
      return current;
    },
    root,
    container,
  };
}

async function waitFor(
  assertion: () => void,
  timeoutMs = 1500,
): Promise<void> {
  const startedAt = Date.now();
  let lastError: unknown;
  while (Date.now() - startedAt < timeoutMs) {
    try {
      assertion();
      return;
    } catch (error) {
      lastError = error;
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
      });
    }
  }
  throw lastError;
}

function resetApiMocks() {
  for (const fn of Object.values(mocks.api)) {
    (fn as Mock).mockReset();
    (fn as Mock).mockResolvedValue(null);
  }
  mocks.open.mockReset();
  mocks.save.mockReset();
  mocks.onDragDropEvent.mockReset();
  mocks.onPlaybackTick.mockReset();
  mocks.onRenderProgress.mockReset();

  mocks.api.listUserPresets.mockResolvedValue([]);
  mocks.api.loadRecentSession.mockResolvedValue(null);
  mocks.api.importTracks.mockResolvedValue([]);
  mocks.api.analyzeTracks.mockResolvedValue([]);
  mocks.api.prepareWaveform.mockImplementation((trackId: string) =>
    Promise.resolve(makeWaveform(trackId)),
  );
  mocks.api.prewarmDecode.mockResolvedValue(null);
  mocks.api.setLoopRegion.mockResolvedValue(null);
  mocks.api.stopPlayback.mockResolvedValue(null);
  mocks.api.playMaster.mockResolvedValue(null);
  mocks.api.updateChain.mockResolvedValue(null);
  mocks.onPlaybackTick.mockResolvedValue(() => {});
  mocks.onRenderProgress.mockResolvedValue(() => {});
  mocks.onDragDropEvent.mockResolvedValue(() => {});
}

beforeEach(() => {
  resetApiMocks();
});

afterEach(() => {
  document.body.innerHTML = "";
});

describe("useTrackMaster integration dispatches", () => {
  it("prewarms the auto-selected track when restoring the recent session", async () => {
    const track = makeTrack("restored-1", "C:/audio/restored.wav");
    mocks.api.loadRecentSession.mockResolvedValue(makeProjectState(track));

    const harness = await renderHookHarness();

    await waitFor(() => {
      expect(mocks.api.prewarmDecode).toHaveBeenCalledWith(track.path);
    });
    await act(async () => {
      harness.root.unmount();
    });
  });

  it("prewarms the first imported track when import auto-selects it", async () => {
    const track = makeTrack("imported-1", "C:/audio/imported.wav");
    mocks.api.importTracks.mockResolvedValue([track]);
    const harness = await renderHookHarness();

    await act(async () => {
      await harness.current().importFiles([track.path]);
    });

    expect(mocks.api.prewarmDecode).toHaveBeenCalledWith(track.path);
    await act(async () => {
      harness.root.unmount();
    });
  });

  it("prewarms the first track when opening a project from disk", async () => {
    const track = makeTrack("project-1", "C:/audio/project.wav");
    mocks.open.mockResolvedValue("C:/projects/test.ams.json");
    mocks.api.loadProject.mockResolvedValue(makeProjectState(track));
    const harness = await renderHookHarness();

    await act(async () => {
      await harness.current().openProjectFromDisk();
    });

    expect(mocks.api.prewarmDecode).toHaveBeenCalledWith(track.path);
    await act(async () => {
      harness.root.unmount();
    });
  });

  it("dispatches updateChain with the current export-LUFS preview flag", async () => {
    const track = makeTrack("mastered-1", "C:/audio/mastered.wav");
    mocks.api.importTracks.mockResolvedValue([track]);
    const harness = await renderHookHarness();

    await act(async () => {
      await harness.current().importFiles([track.path]);
    });
    await waitFor(() => {
      expect(harness.current().selectedTrackId).toBe(track.id);
    });

    await act(async () => {
      await harness.current().setPlaybackKind("master");
    });
    await waitFor(() => {
      expect(harness.current().transport.playbackKind).toBe("master");
    });

    await act(async () => {
      await harness.current().togglePlay();
    });
    await waitFor(() => {
      expect(mocks.api.playMaster).toHaveBeenCalled();
    });

    mocks.api.updateChain.mockClear();
    await act(async () => {
      harness.current().setExportLufsPreview(false);
    });

    await waitFor(() => {
      expect(mocks.api.updateChain).toHaveBeenCalledWith(
        expect.objectContaining({ volume_match: false }),
        false,
      );
    });
    await act(async () => {
      harness.root.unmount();
    });
  });

  it("asks where to save a track master and passes that path to render", async () => {
    const track = makeTrack("export-1", "C:/audio/export source.wav");
    mocks.api.importTracks.mockResolvedValue([track]);
    mocks.api.analyzeTracks.mockResolvedValue([makeAnalysis(track.id)]);
    mocks.save.mockResolvedValue("/Users/daniel/Desktop/exported-master");
    mocks.api.renderTrackMaster.mockResolvedValue(
      makeRenderJob("/Users/daniel/Desktop/exported-master.wav"),
    );
    mocks.api.runExportChecks.mockResolvedValue([]);
    const harness = await renderHookHarness();

    await act(async () => {
      await harness.current().importFiles([track.path]);
    });
    await waitFor(() => {
      expect(harness.current().selectedTrackId).toBe(track.id);
    });

    await act(async () => {
      await harness.current().exportMaster();
    });

    expect(mocks.save).toHaveBeenCalledWith({
      defaultPath: "export-1__master.wav",
      filters: [{ name: "WAV audio", extensions: ["wav"] }],
    });
    expect(mocks.api.renderTrackMaster).toHaveBeenCalledWith(
      track.id,
      track.path,
      DEFAULT_SETTINGS,
      "/Users/daniel/Desktop/exported-master.wav",
    );
    expect(harness.current().lastExportReceipt?.outputPath).toBe(
      "/Users/daniel/Desktop/exported-master.wav",
    );
    await act(async () => {
      harness.root.unmount();
    });
  });

  it("does not render when the export save dialog is cancelled", async () => {
    const track = makeTrack("export-cancel", "C:/audio/export cancel.wav");
    mocks.api.importTracks.mockResolvedValue([track]);
    mocks.api.analyzeTracks.mockResolvedValue([makeAnalysis(track.id)]);
    mocks.save.mockResolvedValue(null);
    const harness = await renderHookHarness();

    await act(async () => {
      await harness.current().importFiles([track.path]);
    });
    await waitFor(() => {
      expect(harness.current().selectedTrackId).toBe(track.id);
    });

    await act(async () => {
      await harness.current().exportMaster();
    });

    expect(mocks.save).toHaveBeenCalled();
    expect(mocks.api.renderTrackMaster).not.toHaveBeenCalled();
    await act(async () => {
      harness.root.unmount();
    });
  });
});

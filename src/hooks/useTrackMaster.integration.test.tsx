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
  AlbumPlan,
  ImportedTrack,
  MasteringSettings,
  ProjectState,
  RenderJob,
  WaveformPeaks,
} from "../bindings";
import { lastExportDirectory } from "../lib/export-location";
import { useTrackMaster } from "./useTrackMaster";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean })
  .IS_REACT_ACT_ENVIRONMENT = true;

const mocks = vi.hoisted(() => {
  const api = {
    importTracks: vi.fn(),
    analyzeTracks: vi.fn(),
    renderTrackPreview: vi.fn(),
    renderTrackMaster: vi.fn(),
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
  eq_sub_db: 0,
  eq_low_db: 0,
  eq_low_mid_db: 0,
  eq_mid_db: 0,
  eq_high_mid_db: 0,
  eq_high_db: 0,
  eq_sparkle_db: 0,
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

function makeAlbumPlan(trackIds: string[]): AlbumPlan {
  return {
    title: "Desk Check",
    arc: { kind: "preset", preset: "cinematic" },
    tracks: trackIds.map((trackId, index) => ({
      track_id: trackId,
      position: index,
      role: index === 0 ? "opener" : "closer",
      role_locked: false,
      arc_lufs_offset_db: 0,
      intensity_scale: 1,
    })),
    transitions: trackIds.map(() => ({
      kind: "direct",
      duration_seconds: 0,
    })),
    intensity: 1,
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

function installTestLocalStorage() {
  const values = new Map<string, string>();
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => {
        values.set(key, String(value));
      },
      removeItem: (key: string) => {
        values.delete(key);
      },
      clear: () => {
        values.clear();
      },
    },
  });
}

beforeEach(() => {
  installTestLocalStorage();
  localStorage.clear();
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
    expect(lastExportDirectory(localStorage, "track")).toBe("/Users/daniel/Desktop");
    await act(async () => {
      harness.root.unmount();
    });
  });

  it("accepts an existing track master path returned by the save dialog", async () => {
    const track = makeTrack("export-overwrite", "C:/audio/export overwrite.wav");
    const outputPath = "/Users/daniel/Desktop/existing-master.wav";
    mocks.api.importTracks.mockResolvedValue([track]);
    mocks.api.analyzeTracks.mockResolvedValue([makeAnalysis(track.id)]);
    mocks.save.mockResolvedValue(outputPath);
    mocks.api.renderTrackMaster.mockResolvedValue(makeRenderJob(outputPath));
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

    expect(mocks.api.renderTrackMaster).toHaveBeenCalledWith(
      track.id,
      track.path,
      DEFAULT_SETTINGS,
      outputPath,
    );
    expect(harness.current().lastExportReceipt?.outputPath).toBe(outputPath);
    await act(async () => {
      harness.root.unmount();
    });
  });

  it("passes Windows-style picker paths through to track rendering unchanged", async () => {
    const track = makeTrack("export-windows", "C:\\audio\\export windows.wav");
    const outputPath = "C:\\Users\\Dan\\Desktop\\existing-master.wav";
    mocks.api.importTracks.mockResolvedValue([track]);
    mocks.api.analyzeTracks.mockResolvedValue([makeAnalysis(track.id)]);
    mocks.save.mockResolvedValue(outputPath);
    mocks.api.renderTrackMaster.mockResolvedValue(makeRenderJob(outputPath));
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

    expect(mocks.api.renderTrackMaster).toHaveBeenCalledWith(
      track.id,
      track.path,
      DEFAULT_SETTINGS,
      outputPath,
    );
    expect(lastExportDirectory(localStorage, "track")).toBe("C:\\Users\\Dan\\Desktop");
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
    expect(harness.current().isExporting).toBe(false);
    expect(harness.current().lastExportReceipt).toBeNull();
    expect(lastExportDirectory(localStorage, "track")).toBeNull();
    await act(async () => {
      harness.root.unmount();
    });
  });

  it("uses the last track export folder as the next save default", async () => {
    const track = makeTrack("export-repeat", "C:/audio/export repeat.wav");
    localStorage.setItem("yes-master:last-track-export-dir", "/Users/daniel/Desktop");
    mocks.api.importTracks.mockResolvedValue([track]);
    mocks.api.analyzeTracks.mockResolvedValue([makeAnalysis(track.id)]);
    mocks.save.mockResolvedValue("/Users/daniel/Desktop/repeated-master.wav");
    mocks.api.renderTrackMaster.mockResolvedValue(
      makeRenderJob("/Users/daniel/Desktop/repeated-master.wav"),
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
      defaultPath: "/Users/daniel/Desktop/export-repeat__master.wav",
      filters: [{ name: "WAV audio", extensions: ["wav"] }],
    });
    await act(async () => {
      harness.root.unmount();
    });
  });

  it("asks where to save an album plan export and passes that folder to render", async () => {
    const first = makeTrack("album-1", "C:/audio/album one.wav");
    const second = makeTrack("album-2", "C:/audio/album two.wav");
    const plan = makeAlbumPlan([first.id, second.id]);
    const outputDir = "/Users/daniel/Desktop/Album Masters";
    mocks.api.importTracks.mockResolvedValue([first, second]);
    mocks.api.analyzeTracks.mockResolvedValue([
      makeAnalysis(first.id),
      makeAnalysis(second.id),
    ]);
    mocks.open.mockResolvedValue(outputDir);
    mocks.api.planAlbum.mockResolvedValue(plan);
    mocks.api.renderAlbumPlan.mockResolvedValue({
      album_wav_path: `${outputDir}/album_continuous_1.wav`,
      manifest_path: `${outputDir}/manifest.json`,
      tracks: [
        {
          track_id: first.id,
          position: 0,
          output_path: `${outputDir}/album-1__master.wav`,
          measured_lufs: -14,
        },
        {
          track_id: second.id,
          position: 1,
          output_path: `${outputDir}/album-2__master.wav`,
          measured_lufs: -14,
        },
      ],
    });
    const harness = await renderHookHarness();

    await act(async () => {
      await harness.current().importFiles([first.path, second.path]);
    });
    await waitFor(() => {
      expect(harness.current().tracks).toHaveLength(2);
    });

    await act(async () => {
      await harness.current().exportAlbumPlan();
    });

    expect(mocks.open).toHaveBeenCalledWith({
      directory: true,
      defaultPath: undefined,
      multiple: false,
      title: "Choose album export folder",
    });
    expect(mocks.api.renderAlbumPlan).toHaveBeenCalledWith(
      plan,
      expect.arrayContaining([
        expect.objectContaining({
          track_id: first.id,
          source_path: first.path,
        }),
        expect.objectContaining({
          track_id: second.id,
          source_path: second.path,
        }),
      ]),
      outputDir,
    );
    expect(harness.current().albumExportReport?.album_wav_path).toBe(
      `${outputDir}/album_continuous_1.wav`,
    );
    expect(lastExportDirectory(localStorage, "album")).toBe(outputDir);
    await act(async () => {
      harness.root.unmount();
    });
  });

  it("accepts an existing album export folder selected by the folder picker", async () => {
    const first = makeTrack("album-existing-1", "C:/audio/album existing one.wav");
    const second = makeTrack("album-existing-2", "C:/audio/album existing two.wav");
    const plan = makeAlbumPlan([first.id, second.id]);
    const outputDir = "/Users/daniel/Desktop/Album Masters";
    localStorage.setItem("yes-master:last-album-export-dir", outputDir);
    mocks.api.importTracks.mockResolvedValue([first, second]);
    mocks.api.analyzeTracks.mockResolvedValue([
      makeAnalysis(first.id),
      makeAnalysis(second.id),
    ]);
    mocks.open.mockResolvedValue(outputDir);
    mocks.api.planAlbum.mockResolvedValue(plan);
    mocks.api.renderAlbumPlan.mockResolvedValue({
      album_wav_path: `${outputDir}/album_continuous_1.wav`,
      manifest_path: `${outputDir}/manifest.json`,
      tracks: [],
    });
    const harness = await renderHookHarness();

    await act(async () => {
      await harness.current().importFiles([first.path, second.path]);
    });
    await waitFor(() => {
      expect(harness.current().tracks).toHaveLength(2);
    });

    await act(async () => {
      await harness.current().exportAlbumPlan();
    });

    expect(mocks.open).toHaveBeenCalledWith({
      directory: true,
      defaultPath: outputDir,
      multiple: false,
      title: "Choose album export folder",
    });
    expect(mocks.api.renderAlbumPlan).toHaveBeenCalledWith(
      plan,
      expect.any(Array),
      outputDir,
    );
    await act(async () => {
      harness.root.unmount();
    });
  });

  it("passes Windows-style album folders through to album rendering unchanged", async () => {
    const first = makeTrack("album-windows-1", "C:\\audio\\album one.wav");
    const second = makeTrack("album-windows-2", "C:\\audio\\album two.wav");
    const plan = makeAlbumPlan([first.id, second.id]);
    const outputDir = "C:\\Users\\Dan\\Desktop\\Album Masters";
    mocks.api.importTracks.mockResolvedValue([first, second]);
    mocks.api.analyzeTracks.mockResolvedValue([
      makeAnalysis(first.id),
      makeAnalysis(second.id),
    ]);
    mocks.open.mockResolvedValue(outputDir);
    mocks.api.planAlbum.mockResolvedValue(plan);
    mocks.api.renderAlbumPlan.mockResolvedValue({
      album_wav_path: `${outputDir}\\album_continuous_1.wav`,
      manifest_path: `${outputDir}\\manifest.json`,
      tracks: [],
    });
    const harness = await renderHookHarness();

    await act(async () => {
      await harness.current().importFiles([first.path, second.path]);
    });
    await waitFor(() => {
      expect(harness.current().tracks).toHaveLength(2);
    });

    await act(async () => {
      await harness.current().exportAlbumPlan();
    });

    expect(mocks.api.renderAlbumPlan).toHaveBeenCalledWith(
      plan,
      expect.any(Array),
      outputDir,
    );
    expect(lastExportDirectory(localStorage, "album")).toBe(outputDir);
    await act(async () => {
      harness.root.unmount();
    });
  });

  it("does not render an album plan when the folder picker is cancelled", async () => {
    const first = makeTrack("album-cancel-1", "C:/audio/album cancel one.wav");
    const second = makeTrack("album-cancel-2", "C:/audio/album cancel two.wav");
    mocks.api.importTracks.mockResolvedValue([first, second]);
    mocks.api.analyzeTracks.mockResolvedValue([
      makeAnalysis(first.id),
      makeAnalysis(second.id),
    ]);
    mocks.open.mockResolvedValue(null);
    const harness = await renderHookHarness();

    await act(async () => {
      await harness.current().importFiles([first.path, second.path]);
    });
    await waitFor(() => {
      expect(harness.current().tracks).toHaveLength(2);
    });

    await act(async () => {
      await harness.current().exportAlbumPlan();
    });

    expect(mocks.open).toHaveBeenCalled();
    expect(mocks.api.planAlbum).not.toHaveBeenCalled();
    expect(mocks.api.renderAlbumPlan).not.toHaveBeenCalled();
    expect(harness.current().albumRendering).toBe(false);
    expect(harness.current().albumExportReport).toBeNull();
    expect(lastExportDirectory(localStorage, "album")).toBeNull();
    await act(async () => {
      harness.root.unmount();
    });
  });

  it("does not expose the legacy album export hook action", async () => {
    const harness = await renderHookHarness();

    expect("exportAlbum" in harness.current()).toBe(false);
    expect("isExportingAlbum" in harness.current()).toBe(false);
    await act(async () => {
      harness.root.unmount();
    });
  });
});

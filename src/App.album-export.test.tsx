import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

import App from "./App";
import type { ImportedTrack, QualityCheck, RenderJob } from "./bindings";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean })
  .IS_REACT_ACT_ENVIRONMENT = true;

const mocks = vi.hoisted(() => ({
  tm: null as Record<string, unknown> | null,
}));

vi.mock("./hooks/useTrackMaster", () => ({
  useTrackMaster: () => {
    if (!mocks.tm) throw new Error("mock tm not configured");
    return mocks.tm;
  },
}));

const track: ImportedTrack = {
  id: "album-track-1",
  path: "/audio/album-track-1.wav",
  display_name: "album-track-1.wav",
  source_format: "wav",
  duration_seconds: 120,
  sample_rate: 44_100,
  channels: 2,
};

const cleanCheck: QualityCheck = {
  level: "info",
  code: "export_ok",
  message: "No export issues detected.",
};

const warningCheck: QualityCheck = {
  level: "warning",
  code: "streaming_headroom_low",
  message: "Streaming headroom is tight.",
};

function renderJob(outputPaths: string[]): RenderJob {
  return {
    id: "render-job-1",
    kind: "master",
    target_tracks: [track.id],
    status: { status: "done" },
    progress: 1,
    started_at_iso: "2026-05-19T00:00:00Z",
    output_paths: outputPaths,
  };
}

function baseTrackMasterState(): Record<string, unknown> {
  return {
    mode: "album",
    setMode: vi.fn(),
    saveProjectAs: vi.fn(),
    openProjectFromDisk: vi.fn(),
    tracks: [track],
    selectedTrackId: null,
    selectedTrack: null,
    selectedAnalysis: undefined,
    selectedWaveform: undefined,
    selectedSettings: undefined,
    selectedRegion: null,
    selectTrack: vi.fn(),
    removeTrack: vi.fn(),
    openImportDialog: vi.fn(),
    isAnalyzing: false,
    isLoadingWaveform: false,
    isDragOver: false,
    isExporting: false,
    isRendering: false,
    previewStale: false,
    updatePreview: vi.fn(),
    exportMaster: vi.fn(),
    error: null,
    clearError: vi.fn(),
    lastExportReceipt: null,
    clearExportReceipt: vi.fn(),
    reorderTracks: vi.fn(),
    overrideAlbum: new Set(),
    albumArcKind: "cinematic",
    albumIntensity: 1,
    albumTitle: "",
    albumRendering: false,
    albumExportReport: null,
    setAlbumArc: vi.fn(),
    setAlbumIntensity: vi.fn(),
    setAlbumTitle: vi.fn(),
    exportAlbumPlan: vi.fn(),
    transport: {
      isPlaying: false,
      currentTimeSec: 0,
      playbackKind: "source",
      loop: false,
      volumeMatch: false,
      exportLufsPreview: true,
      peakDbfs: -120,
      compressionGr: { low: -120, mid: -120, high: -120 },
      lufsMomentary: -120,
      lufsIntegrated: -120,
      spectrumDb: [],
    },
    liveUpdateStats: { attempts: 0, applied: 0, lastAt: null },
    renderProgress: null,
    undo: vi.fn(),
    redo: vi.fn(),
    canUndo: false,
    canRedo: false,
    setPreset: vi.fn(),
    setIntensity: vi.fn(),
    setEqBand: vi.fn(),
    setAdvanced: vi.fn(),
    setInputGain: vi.fn(),
    setOutputGain: vi.fn(),
    setDeliveryProfile: vi.fn(),
    togglePlay: vi.fn(),
    seek: vi.fn(),
    setPlaybackKind: vi.fn(),
    toggleLoop: vi.fn(),
    setVolumeMatch: vi.fn(),
    setExportLufsPreview: vi.fn(),
    advancedOpen: false,
    toggleAdvanced: vi.fn(),
    setRegion: vi.fn(),
    clearRegion: vi.fn(),
    albumIntent: null,
    updateAlbumIntent: vi.fn(),
    selectedIsOverriding: false,
    followingAlbumIntent: false,
    toggleOverrideAlbum: vi.fn(),
    userPresets: [],
    savingPreset: false,
    saveCurrentPreset: vi.fn(),
    deleteUserPresetById: vi.fn(),
  };
}

async function renderApp(): Promise<{ container: HTMLDivElement; root: Root }> {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => {
    root.render(<App />);
  });
  return { container, root };
}

afterEach(() => {
  document.body.innerHTML = "";
  mocks.tm = null;
});

describe("album export actions", () => {
  it("shows a single Album Export button in album mode", async () => {
    mocks.tm = baseTrackMasterState();

    const { container, root } = await renderApp();

    const exportButtons = Array.from(container.querySelectorAll("button")).filter(
      (button) => button.textContent?.trim() === "Export Album",
    );
    expect(exportButtons).toHaveLength(1);
    await act(async () => {
      root.unmount();
    });
  });

  it("shows a quiet export journey and clean result on completed exports", async () => {
    mocks.tm = {
      ...baseTrackMasterState(),
      lastExportReceipt: {
        trackId: track.id,
        outputPath: "/Users/daniel/Masters/album-track-1__master.wav",
        checks: [cleanCheck],
        job: renderJob(["/Users/daniel/Masters/album-track-1__master.wav"]),
        kind: "track",
      },
    };

    const { container, root } = await renderApp();

    expect(container.querySelector(".receipt-medallion-clean")?.textContent).toContain(
      "Clean",
    );
    const steps = Array.from(container.querySelectorAll(".receipt-journey-step"));
    expect(steps.map((step) => step.textContent?.trim())).toEqual([
      "Analyze",
      "Master",
      "Quality",
      "Saved",
    ]);
    expect(container.querySelector(".receipt-path-name")?.textContent).toBe(
      "album-track-1__master.wav",
    );

    await act(async () => {
      root.unmount();
    });
  });

  it("marks completed exports for review when quality checks warn", async () => {
    mocks.tm = {
      ...baseTrackMasterState(),
      lastExportReceipt: {
        trackId: track.id,
        outputPath: "/Users/daniel/Masters/album-track-1__master.wav",
        checks: [warningCheck],
        job: renderJob(["/Users/daniel/Masters/album-track-1__master.wav"]),
        kind: "track",
      },
    };

    const { container, root } = await renderApp();

    expect(container.querySelector(".receipt-medallion-review")?.textContent).toContain(
      "Review",
    );
    expect(container.querySelector(".receipt-summary")?.textContent).toContain(
      "1 item to review",
    );

    await act(async () => {
      root.unmount();
    });
  });
});

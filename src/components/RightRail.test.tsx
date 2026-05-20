import { act } from "react";
import type { ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { AnalysisResult, MasteringSettings } from "../bindings";
import { MasterOutPanel, RightRail } from "./RightRail";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean })
  .IS_REACT_ACT_ENVIRONMENT = true;

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

const HOT_SOURCE_ANALYSIS: AnalysisResult = {
  track_id: "track-1",
  lufs_integrated: -10.5,
  lufs_short_term_max: -8.8,
  true_peak_dbtp: 0.2,
  dynamic_range_lu: 3.3,
  spectral_balance: { low: 0.3, mid: 0.4, high: 0.3 },
  transient_density: 0.5,
  stereo_width: 0.5,
  recommended_universal: DEFAULT_SETTINGS,
  measured_at_iso: "2026-05-20T00:00:00.000Z",
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

async function renderNode(node: ReactNode): Promise<{
  container: HTMLDivElement;
  root: Root;
}> {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => {
    root.render(node);
  });
  return { container, root };
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("MasterOutPanel", () => {
  it("does not show source analysis as live output while idle", async () => {
    const { container, root } = await renderNode(
      <MasterOutPanel
        isAnalyzing={false}
        peakDbfs={-120}
        isPlaying={false}
        lufsMomentary={-120}
        lufsIntegrated={-120}
      />,
    );

    expect(container.textContent).toContain("idle");
    expect(container.textContent).not.toContain("-10.5");
    expect(container.textContent).not.toContain("0.2");
    expect(container.querySelector(".tp-bar-label")?.textContent).toBe("PK");
    expect(
      (container.querySelector(".tp-bar-fill") as HTMLElement | null)?.style.height,
    ).toBe("0%");

    await act(async () => {
      root.unmount();
    });
  });

  it("shows live playback tick values only while playing", async () => {
    const { container, root } = await renderNode(
      <MasterOutPanel
        isAnalyzing={false}
        peakDbfs={-8.5}
        isPlaying
        lufsMomentary={-9.7}
        lufsIntegrated={-10.5}
      />,
    );

    expect(container.textContent).toContain("LIVE");
    expect(container.textContent).toContain("Momentary");
    expect(container.textContent).toContain("Since Play");
    expect(container.textContent).toContain("Live Peak");
    expect(container.textContent).toContain("-9.7");
    expect(container.textContent).toContain("-10.5");
    expect(container.textContent).toContain("-8.5");

    await act(async () => {
      root.unmount();
    });
  });
});

describe("RightRail source checks", () => {
  it("labels pre-export analysis as source measurements", async () => {
    const { container, root } = await renderNode(
      <RightRail
        analysis={HOT_SOURCE_ANALYSIS}
        lastChecks={undefined}
        canExport
        isExporting={false}
        isRendering={false}
        onExport={vi.fn()}
        previewStale={false}
        canRenderPreview
        onUpdatePreview={vi.fn()}
      />,
    );

    expect(container.textContent).toContain("SOURCE CHECK");
    expect(container.textContent).toContain("Source true peak 0.2 dBTP");
    expect(container.textContent).toContain("Source loudness -10.5 LUFS");
    expect(container.textContent).toContain("Source dynamic range 3.3 LU");

    await act(async () => {
      root.unmount();
    });
  });
});

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

import { LoudnessTarget } from "./App";
import type {
  AdvancedSettings,
  DeliveryProfile,
  MasteringSettings,
} from "./bindings";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean })
  .IS_REACT_ACT_ENVIRONMENT = true;

const DEFAULT_ADVANCED: AdvancedSettings = {
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

function makeSettings(
  deliveryProfile: DeliveryProfile,
  advanced: Partial<AdvancedSettings> = {},
): MasteringSettings {
  return {
    preset: { kind: "universal" },
    intensity: 0.5,
    eq_low_db: 0,
    eq_low_mid_db: 0,
    eq_mid_db: 0,
    eq_high_db: 0,
    volume_match: false,
    input_gain_db: 0,
    output_gain_db: 0,
    delivery_profile: deliveryProfile,
    advanced: { ...DEFAULT_ADVANCED, ...advanced },
  };
}

async function renderLoudnessTarget(props: {
  settings: MasteringSettings;
  onAdvanced: (advanced: AdvancedSettings) => void;
  onDeliveryProfile: (profile: DeliveryProfile) => void;
}): Promise<{ container: HTMLDivElement; root: Root }> {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => {
    root.render(<LoudnessTarget {...props} />);
  });
  return { container, root };
}

async function chooseProfile(container: Element, id: string): Promise<void> {
  const select = container.querySelector("select");
  if (!(select instanceof HTMLSelectElement)) {
    throw new Error("LoudnessTarget select not found");
  }
  await act(async () => {
    select.value = id;
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("LoudnessTarget component", () => {
  it("forces Custom before writing an explicit LUFS pick shadowed by a delivery profile", async () => {
    const onAdvanced = vi.fn();
    const onDeliveryProfile = vi.fn();
    const { container, root } = await renderLoudnessTarget({
      settings: makeSettings("streaming-universal"),
      onAdvanced,
      onDeliveryProfile,
    });

    await chooseProfile(container, "cd-master");

    expect(onDeliveryProfile).toHaveBeenCalledWith("custom");
    expect(onAdvanced).toHaveBeenCalledWith({
      ...DEFAULT_ADVANCED,
      lufs_offset_db: -9,
    });
    await act(async () => {
      root.unmount();
    });
  });

  it("also force-flips the Off / Natural pick even when it writes null over null", async () => {
    const onAdvanced = vi.fn();
    const onDeliveryProfile = vi.fn();
    const { container, root } = await renderLoudnessTarget({
      settings: makeSettings("loud-rock", { lufs_offset_db: null }),
      onAdvanced,
      onDeliveryProfile,
    });

    await chooseProfile(container, "off");

    expect(onDeliveryProfile).toHaveBeenCalledWith("custom");
    expect(onAdvanced).toHaveBeenCalledWith({
      ...DEFAULT_ADVANCED,
      lufs_offset_db: null,
    });
    await act(async () => {
      root.unmount();
    });
  });
});

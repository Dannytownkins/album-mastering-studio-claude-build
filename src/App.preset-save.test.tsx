import { act } from "react";
import type { ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

import { PresetTiles, UserPresetSection } from "./App";
import type { UserPreset } from "./bindings";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean })
  .IS_REACT_ACT_ENVIRONMENT = true;

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

describe("preset save placement", () => {
  it("keeps the save action beside the Preset label instead of a separate row", async () => {
    const onSave = vi.fn();
    const { container, root } = await renderNode(
      <PresetTiles
        selected={{ kind: "universal" }}
        onChange={vi.fn()}
        savingPreset={false}
        onSave={onSave}
      />,
    );

    expect(container.textContent).not.toContain("Save current as preset");

    const saveToggle = container.querySelector(
      '[aria-label="Save current settings as preset"]',
    );
    expect(saveToggle).toBeInstanceOf(HTMLButtonElement);

    await act(async () => {
      saveToggle?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    const input = container.querySelector(
      'input[placeholder="Preset name"]',
    ) as HTMLInputElement | null;
    expect(input).toBeInstanceOf(HTMLInputElement);

    await act(async () => {
      if (!input) return;
      const setter = Object.getOwnPropertyDescriptor(
        HTMLInputElement.prototype,
        "value",
      )?.set;
      setter?.call(input, "Desk check");
      input.dispatchEvent(new Event("input", { bubbles: true }));
      input.dispatchEvent(new Event("change", { bubbles: true }));
    });

    const form = container.querySelector("form.preset-save-inline");
    await act(async () => {
      form?.dispatchEvent(
        new Event("submit", { bubbles: true, cancelable: true }),
      );
    });

    expect(onSave).toHaveBeenCalledWith("Desk check");
    await act(async () => {
      root.unmount();
    });
  });

  it("does not render an empty save-preset row below the chain", async () => {
    const { container, root } = await renderNode(
      <UserPresetSection
        presets={[]}
        onDelete={vi.fn()}
        onApply={vi.fn()}
      />,
    );

    expect(container.textContent?.trim()).toBe("");
    expect(container.querySelector(".user-presets-add-inline")).toBeNull();
    await act(async () => {
      root.unmount();
    });
  });

  it("still renders saved preset chips when they exist", async () => {
    const preset: UserPreset = {
      id: "preset-1",
      name: "My preset",
      kind: "track",
      created_at_iso: "2026-05-19T00:00:00Z",
      settings: {
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
      },
    };
    const { container, root } = await renderNode(
      <UserPresetSection
        presets={[preset]}
        onDelete={vi.fn()}
        onApply={vi.fn()}
      />,
    );

    expect(container.textContent).toContain("MY PRESETS");
    expect(container.textContent).toContain("My preset");
    await act(async () => {
      root.unmount();
    });
  });
});

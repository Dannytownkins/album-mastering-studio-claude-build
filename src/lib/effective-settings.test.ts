import { describe, expect, it } from "vitest";

import type {
  AdvancedSettings,
  DeliveryProfile,
  MasteringSettings,
} from "../bindings";
import {
  effectiveLoudnessTarget,
  LOUDNESS_PROFILES,
  loudnessTargetDisplay,
  profileIdForLufs,
} from "./effective-settings";

// Frontend mirror tests for the `effective_*` accessors. The Rust
// source-of-truth tests live in `src-tauri/src/types.rs` under
// `effective_settings_tests`; these tests verify the frontend helper
// honors the same shadowing rule so the LoudnessTarget readout never
// lies about what the chain will target.
//
// First Vitest file in the repo (scaffold for future frontend tests).
// `effectiveLoudnessTarget` is a small pure function but a useful
// canary: it depends on `DELIVERY_PROFILE_TARGET_LUFS` from
// bindings.ts, so this test also catches any divergence between the
// generated bindings and the Rust enum.

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
  profile: DeliveryProfile,
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
    delivery_profile: profile,
    advanced: { ...DEFAULT_ADVANCED, ...advanced },
  };
}

describe("effectiveLoudnessTarget", () => {
  it("returns the profile's target when delivery_profile is non-Custom", () => {
    // Mirrors Rust:
    // effective_target_lufs_profile_overrides_advanced
    const settings = makeSettings("streaming-universal", {
      lufs_offset_db: -9, // would-be user override
    });
    expect(effectiveLoudnessTarget(settings)).toBe(-14);
  });

  it("falls through to advanced.lufs_offset_db when delivery_profile is Custom", () => {
    // Mirrors Rust:
    // effective_target_lufs_custom_uses_advanced_value
    const settings = makeSettings("custom", { lufs_offset_db: -9 });
    expect(effectiveLoudnessTarget(settings)).toBe(-9);
  });

  it("returns null when delivery_profile is Custom and advanced.lufs_offset_db is null", () => {
    // Mirrors Rust:
    // effective_target_lufs_custom_with_none_advanced_returns_none
    const settings = makeSettings("custom");
    expect(effectiveLoudnessTarget(settings)).toBeNull();
  });

  it("reports the right target for every non-Custom profile", () => {
    // Mirrors Rust:
    // effective_target_lufs_known_for_every_non_custom_profile
    const cases: Array<[DeliveryProfile, number]> = [
      ["streaming-universal", -14],
      ["apple-music", -16],
      ["cd", -14],
      ["vinyl-premaster", -18],
      ["loud-rock", -10.5],
      ["broadcast-eu", -23],
      ["broadcast-us", -24],
    ];
    for (const [profile, expected] of cases) {
      const settings = makeSettings(profile);
      expect(
        effectiveLoudnessTarget(settings),
        `profile ${profile} must report ${expected} LUFS`,
      ).toBe(expected);
    }
  });

  it("ignores volume_match (orthogonal to the landing target)", () => {
    // VM toggle should not change the effective target — same reason
    // the Rust accessor doesn't consult volume_match, and the same
    // reason the live-preview-cache hash strips it.
    const off = makeSettings("custom", { lufs_offset_db: -12 });
    const on: MasteringSettings = { ...off, volume_match: true };
    expect(effectiveLoudnessTarget(off)).toBe(-12);
    expect(effectiveLoudnessTarget(on)).toBe(-12);
  });
});

describe("LOUDNESS_PROFILES (quick-select dropdown options)", () => {
  it("exposes the four canonical dropdown entries", () => {
    // Single source of truth — the rendered <option> list AND the
    // profileIdForLufs lookup both consume this array.
    const ids = LOUDNESS_PROFILES.map((p) => p.id);
    expect(ids).toEqual(["streaming", "loud-streaming", "cd-master", "off"]);
    const lufs = LOUDNESS_PROFILES.map((p) => p.lufs);
    expect(lufs).toEqual([-14, -11, -9, null]);
  });
});

describe("profileIdForLufs", () => {
  it("matches each canonical profile target to its id", () => {
    expect(profileIdForLufs(-14)).toBe("streaming");
    expect(profileIdForLufs(-11)).toBe("loud-streaming");
    expect(profileIdForLufs(-9)).toBe("cd-master");
  });

  it("treats null as 'off / natural'", () => {
    expect(profileIdForLufs(null)).toBe("off");
  });

  it("returns 'custom' for any value outside the canonical set", () => {
    expect(profileIdForLufs(-12)).toBe("custom");
    expect(profileIdForLufs(-6)).toBe("custom");
    expect(profileIdForLufs(-10.5)).toBe("custom");
  });

  it("matches within ±1e-3 LU tolerance", () => {
    // Floating-point fuzz on the comparison — -14 ± 0.0005 still
    // maps to streaming, -14 ± 0.002 is "custom."
    expect(profileIdForLufs(-13.9995)).toBe("streaming");
    expect(profileIdForLufs(-14.0005)).toBe("streaming");
    expect(profileIdForLufs(-14.002)).toBe("custom");
  });
});

describe("loudnessTargetDisplay (the LoudnessTarget readout)", () => {
  it("on Streaming profile reports the profile's -14 target, NOT raw advanced", () => {
    // The headline trust-pattern fix from this session, written
    // as a single aggregate assertion: when the user is on a
    // non-Custom profile, the readout shows what the chain is
    // actually targeting (the profile's value), the dropdown
    // selection matches, and the formatted text is the rounded
    // LUFS value.
    const settings: MasteringSettings = {
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
        ...DEFAULT_ADVANCED,
        lufs_offset_db: -9, // would be shadowed by the profile
      },
    };
    const result = loudnessTargetDisplay(settings);
    expect(result.current).toBe(-14);
    expect(result.profileId).toBe("streaming");
    expect(result.displayText).toBe("-14.0");
  });

  it("on Custom + null advanced reports 'no target'", () => {
    const settings: MasteringSettings = {
      preset: { kind: "universal" },
      intensity: 0.5,
      eq_low_db: 0,
      eq_low_mid_db: 0,
      eq_mid_db: 0,
      eq_high_db: 0,
      volume_match: false,
      input_gain_db: 0,
      output_gain_db: 0,
      delivery_profile: "custom",
      advanced: { ...DEFAULT_ADVANCED, lufs_offset_db: null },
    };
    const result = loudnessTargetDisplay(settings);
    expect(result.current).toBeNull();
    expect(result.profileId).toBe("off");
    expect(result.displayText).toBe("—");
  });

  it("on Custom + user-typed -12 reports 'custom' profileId and the typed value", () => {
    const settings: MasteringSettings = {
      preset: { kind: "universal" },
      intensity: 0.5,
      eq_low_db: 0,
      eq_low_mid_db: 0,
      eq_mid_db: 0,
      eq_high_db: 0,
      volume_match: false,
      input_gain_db: 0,
      output_gain_db: 0,
      delivery_profile: "custom",
      advanced: { ...DEFAULT_ADVANCED, lufs_offset_db: -12 },
    };
    const result = loudnessTargetDisplay(settings);
    expect(result.current).toBe(-12);
    // -12 doesn't match any quick-select entry → "custom" lights
    // up the Custom dropdown option.
    expect(result.profileId).toBe("custom");
    expect(result.displayText).toBe("-12.0");
  });
});

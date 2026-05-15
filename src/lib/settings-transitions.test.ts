import { describe, expect, it } from "vitest";

import type {
  AdvancedSettings,
  DeliveryProfile,
  MasteringSettings,
} from "../bindings";
import {
  applyAdvancedWithProfileFlip,
  shouldFlipToCustomOnLoudnessPick,
  SHADOWED_ADVANCED_KEYS,
} from "./settings-transitions";

// Mechanical gates for B7 (auto-flip-to-Custom on shadowed-field
// edit) and the LoudnessTarget quick-select force-flip. Before this
// extraction, both decisions lived inside React callbacks and could
// only be verified by manual click-testing; now they're pure
// functions with explicit input/output contracts.

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

describe("applyAdvancedWithProfileFlip (B7 — auto-flip-to-Custom)", () => {
  it("flips delivery_profile to Custom when a shadowed field changes on a non-Custom profile", () => {
    // Repro: user on StreamingUniversal types -9 into the LUFS target
    // field. Pre-B7, advanced.lufs_offset_db would be -9 but
    // delivery_profile would stay "streaming-universal" and the chain
    // would target -14. With the flip, profile becomes "custom" and
    // -9 is authoritative.
    const prev = makeSettings("streaming-universal", { lufs_offset_db: null });
    const next = applyAdvancedWithProfileFlip(prev, {
      ...prev.advanced,
      lufs_offset_db: -9,
    });
    expect(next.delivery_profile).toBe("custom");
    expect(next.advanced.lufs_offset_db).toBe(-9);
  });

  it("does NOT flip when the profile is already Custom", () => {
    const prev = makeSettings("custom", { lufs_offset_db: -14 });
    const next = applyAdvancedWithProfileFlip(prev, {
      ...prev.advanced,
      lufs_offset_db: -9,
    });
    expect(next.delivery_profile).toBe("custom"); // unchanged
    expect(next.advanced.lufs_offset_db).toBe(-9);
  });

  it("does NOT flip when the edit touches a NON-shadowed advanced field", () => {
    // `warmth` is an advanced field but isn't shadowed by delivery
    // profiles — editing it shouldn't promote the session to Custom.
    const prev = makeSettings("streaming-universal", { warmth: 0.0 });
    const next = applyAdvancedWithProfileFlip(prev, {
      ...prev.advanced,
      warmth: 0.5,
    });
    expect(next.delivery_profile).toBe("streaming-universal"); // unchanged
    expect(next.advanced.warmth).toBe(0.5);
  });

  it("does NOT flip when the shadowed field's value didn't actually change (degenerate case)", () => {
    // Documents the known edge case: typing the SAME value (e.g.
    // null -> null when the field was already null) is detected as
    // "no diff" by the value-comparison and produces no flip. This
    // is fine because the displayed value didn't change either; the
    // visual asymmetry can't be observed. The LoudnessTarget quick-
    // select uses `shouldFlipToCustomOnLoudnessPick` instead, which
    // captures explicit intent regardless of value diff.
    const prev = makeSettings("streaming-universal", { lufs_offset_db: null });
    const next = applyAdvancedWithProfileFlip(prev, {
      ...prev.advanced,
      lufs_offset_db: null,
    });
    expect(next.delivery_profile).toBe("streaming-universal");
  });

  it("flips on a change to ANY of the four shadowed keys", () => {
    // Discriminator test: each shadowed key must independently trigger
    // the flip. If we add a new shadowed key to types.rs and forget to
    // mirror it in SHADOWED_ADVANCED_KEYS, this test won't catch that
    // directly — but reading the SHADOWED_ADVANCED_KEYS constant from
    // the module under test ensures we're at least exercising the
    // declared set.
    const baseline = makeSettings("streaming-universal");
    const changes: Partial<AdvancedSettings>[] = [
      { lufs_offset_db: -9 },
      { ceiling_dbtp: -2 },
      { bit_depth: 16 },
      { target_sample_rate: 44_100 },
    ];
    // Sanity: SHADOWED_ADVANCED_KEYS should match the four keys we're testing.
    expect(SHADOWED_ADVANCED_KEYS).toEqual([
      "lufs_offset_db",
      "ceiling_dbtp",
      "bit_depth",
      "target_sample_rate",
    ]);
    for (const change of changes) {
      const next = applyAdvancedWithProfileFlip(baseline, {
        ...baseline.advanced,
        ...change,
      });
      expect(
        next.delivery_profile,
        `editing ${Object.keys(change)[0]} must flip to custom`,
      ).toBe("custom");
    }
  });

  it("preserves the rest of MasteringSettings — only advanced + delivery_profile change", () => {
    // Discriminator: the helper shouldn't accidentally drop or mutate
    // other fields like intensity / EQ / preset.
    const prev = makeSettings("apple-music", { lufs_offset_db: null });
    prev.intensity = 0.7;
    prev.eq_mid_db = 3.5;
    prev.preset = { kind: "tape" };
    const next = applyAdvancedWithProfileFlip(prev, {
      ...prev.advanced,
      lufs_offset_db: -12,
    });
    expect(next.intensity).toBe(0.7);
    expect(next.eq_mid_db).toBe(3.5);
    expect(next.preset).toEqual({ kind: "tape" });
    expect(next.volume_match).toBe(prev.volume_match);
  });
});

describe("shouldFlipToCustomOnLoudnessPick (quick-select force-flip)", () => {
  it("returns false when the user picks the 'custom' dropdown entry (no-op)", () => {
    expect(shouldFlipToCustomOnLoudnessPick("custom", "streaming-universal")).toBe(false);
    expect(shouldFlipToCustomOnLoudnessPick("custom", "custom")).toBe(false);
  });

  it("returns true when picking a real loudness option while on a non-Custom profile", () => {
    expect(shouldFlipToCustomOnLoudnessPick("streaming", "streaming-universal")).toBe(true);
    expect(shouldFlipToCustomOnLoudnessPick("loud-streaming", "apple-music")).toBe(true);
    expect(shouldFlipToCustomOnLoudnessPick("cd-master", "vinyl-premaster")).toBe(true);
  });

  it("returns true on 'off / natural' pick when profile is non-Custom (the null->null no-op gap)", () => {
    // The bug this fixes: pre-fix, picking "Off / Natural" while on
    // StreamingUniversal wrote null to advanced.lufs_offset_db. Since
    // it was already null, the B7 auto-flip's diff check returned false
    // → no flip → chain kept targeting -14. User's "Off" intent was
    // silently ignored. The explicit force-flip path closes this gap.
    expect(shouldFlipToCustomOnLoudnessPick("off", "streaming-universal")).toBe(true);
    expect(shouldFlipToCustomOnLoudnessPick("off", "broadcast-eu")).toBe(true);
  });

  it("returns false when the user picks any option while already on Custom", () => {
    // No flip needed; the value write alone is sufficient.
    expect(shouldFlipToCustomOnLoudnessPick("streaming", "custom")).toBe(false);
    expect(shouldFlipToCustomOnLoudnessPick("off", "custom")).toBe(false);
  });
});

import { describe, expect, it } from "vitest";

import type {
  AdvancedSettings,
  AnalysisResult,
  DeliveryProfile,
  MasteringSettings,
  TrackId,
} from "../bindings";
import {
  applyAdvancedWithProfileFlip,
  applyChainDispatchOverrides,
  applyDeliveryProfileSelection,
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

describe("applyDeliveryProfileSelection", () => {
  it("writes a named profile's target, ceiling, and bit depth into Advanced", () => {
    const prev = makeSettings("loud-rock", {
      lufs_offset_db: -10.5,
      ceiling_dbtp: -1,
      bit_depth: 24,
    });

    const next = applyDeliveryProfileSelection(prev, "cd");

    expect(next.delivery_profile).toBe("cd");
    expect(next.advanced.lufs_offset_db).toBe(-14);
    expect(next.advanced.ceiling_dbtp).toBe(-1);
    expect(next.advanced.bit_depth).toBe(16);
  });

  it("makes Custom inherit the currently effective profile values", () => {
    const prev = makeSettings("vinyl-premaster", {
      lufs_offset_db: -9,
      ceiling_dbtp: -0.5,
      bit_depth: 16,
    });

    const next = applyDeliveryProfileSelection(prev, "custom");

    expect(next.delivery_profile).toBe("custom");
    expect(next.advanced.lufs_offset_db).toBe(-18);
    expect(next.advanced.ceiling_dbtp).toBe(-3);
    expect(next.advanced.bit_depth).toBe(24);
  });

  it("preserves non-delivery advanced controls when selecting a profile", () => {
    const prev = makeSettings("streaming-universal", {
      warmth: 0.4,
      presence_air: 0.6,
      compression_density: 0.2,
    });

    const next = applyDeliveryProfileSelection(prev, "broadcast-us");

    expect(next.advanced.warmth).toBe(0.4);
    expect(next.advanced.presence_air).toBe(0.6);
    expect(next.advanced.compression_density).toBe(0.2);
  });
});

describe("applyChainDispatchOverrides (VM session-level + source_lufs injection)", () => {
  function stubAnalysis(lufs: number): AnalysisResult {
    return {
      track_id: "track-a" as TrackId,
      lufs_integrated: lufs,
      lufs_short_term_max: lufs,
      true_peak_dbtp: -1,
      dynamic_range_lu: 8,
      spectral_balance: { low: 0.33, mid: 0.34, high: 0.33 },
      transient_density: 0.5,
      stereo_width: 0.5,
      recommended_universal: makeSettings("custom"),
      measured_at_iso: "2026-05-15T12:00:00Z",
      inferred_role: "album_track",
      role_confidence: "moderate",
      inferred_character: null,
      character_confidence: null,
      spectral_balance_6band: null,
      transient_flux: 0.5,
      stereo_correlation: 0.0,
      dynamic_range_p95_p10_db: 8,
      lufs_short_term_max_3s: lufs,
      energy_density_score: 0.5,
    };
  }

  it("overrides settings.volume_match with the transport value (VM is session-level)", () => {
    // Pre-Phase-A4-hotfix-3, per-track settings.volume_match would
    // persist and silently disagree with the transport checkbox after
    // track switches. The dispatch-time override forces consistency.
    const base = makeSettings("custom");
    base.volume_match = false;
    const onTrue = applyChainDispatchOverrides(base, null, {}, true);
    const onFalse = applyChainDispatchOverrides(base, null, {}, false);
    expect(onTrue.volume_match).toBe(true);
    expect(onFalse.volume_match).toBe(false);
  });

  it("injects source_lufs_integrated from analysisMap when the trackId resolves", () => {
    const base = makeSettings("custom");
    const trackId = "track-a" as TrackId;
    const analysisMap: Record<TrackId, AnalysisResult> = {
      [trackId]: stubAnalysis(-13.4),
    };
    const result = applyChainDispatchOverrides(base, trackId, analysisMap, false);
    expect(result.source_lufs_integrated).toBeCloseTo(-13.4, 6);
  });

  it("does NOT inject source_lufs when trackId is null", () => {
    const base = makeSettings("custom");
    const analysisMap: Record<TrackId, AnalysisResult> = {
      ["track-a" as TrackId]: stubAnalysis(-13.4),
    };
    const result = applyChainDispatchOverrides(base, null, analysisMap, false);
    expect(result.source_lufs_integrated ?? null).toBeNull();
  });

  it("does NOT inject source_lufs when analysisMap has no entry for the track", () => {
    const base = makeSettings("custom");
    const trackId = "untracked-id" as TrackId;
    const result = applyChainDispatchOverrides(base, trackId, {}, false);
    expect(result.source_lufs_integrated ?? null).toBeNull();
  });

  it("does NOT inject non-finite source_lufs (NaN, Infinity)", () => {
    // Defensive: an analysis that returned non-finite shouldn't
    // poison the chain's VM cap math downstream.
    const base = makeSettings("custom");
    const trackId = "track-a" as TrackId;
    for (const bogus of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY]) {
      const analysisMap: Record<TrackId, AnalysisResult> = {
        [trackId]: stubAnalysis(bogus),
      };
      const result = applyChainDispatchOverrides(base, trackId, analysisMap, false);
      expect(result.source_lufs_integrated ?? null).toBeNull();
    }
  });

  it("preserves the rest of the settings — only volume_match + source_lufs are touched", () => {
    // Discriminator: intensity, EQ, preset, advanced fields should
    // all pass through unchanged. The override only writes to the
    // two fields it documents.
    const base = makeSettings("streaming-universal", {
      lufs_offset_db: -12,
      ceiling_dbtp: -1.5,
    });
    base.intensity = 0.7;
    base.eq_low_db = 2.5;
    base.preset = { kind: "tape" };
    const trackId = "track-a" as TrackId;
    const result = applyChainDispatchOverrides(
      base,
      trackId,
      { [trackId]: stubAnalysis(-13) },
      true,
    );
    expect(result.intensity).toBe(0.7);
    expect(result.eq_low_db).toBe(2.5);
    expect(result.preset).toEqual({ kind: "tape" });
    expect(result.delivery_profile).toBe("streaming-universal");
    expect(result.advanced.lufs_offset_db).toBe(-12);
    expect(result.advanced.ceiling_dbtp).toBe(-1.5);
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

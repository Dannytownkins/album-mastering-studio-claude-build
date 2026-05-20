// Pure helpers for the "what should the settings change to" decisions
// that the hook fires in response to user actions. Extracted from
// `useTrackMaster.ts` (B7 auto-flip-to-Custom on shadowed-field edit)
// and `App.tsx` (LoudnessTarget quick-select force-flip) so both
// decision rules live alongside their Vitest tests.
//
// Write-direction siblings of `effective-settings.ts` (the read-
// direction shadowing helper). Together they enforce UI-truthfulness
// across the two directions: the readout shows what the chain is
// targeting (effective-settings) and the setters keep the displayed
// value authoritative when the user edits anything the chain reads
// (settings-transitions).

import type {
  AdvancedSettings,
  AnalysisResult,
  DeliveryProfile,
  MasteringSettings,
  TrackId,
} from "../bindings";
import {
  DELIVERY_PROFILE_BIT_DEPTH,
  DELIVERY_PROFILE_CEILING_DBTP,
  DELIVERY_PROFILE_TARGET_LUFS,
} from "../bindings";
import {
  effectiveBitDepth,
  effectiveCeilingDbtp,
  effectiveLoudnessTarget,
} from "./effective-settings";

/// Keys on `AdvancedSettings` that a non-Custom `DeliveryProfile`
/// would shadow at render time (see `MasteringSettings::effective_*`
/// in src-tauri/src/types.rs). When the user edits one of these
/// directly, the displayed value MUST become the value export uses —
/// otherwise the UI lies about what the chain is targeting.
///
/// Mirror of the Rust precedence rules tested in
/// `effective_settings_tests`.
export const SHADOWED_ADVANCED_KEYS = [
  "lufs_offset_db",
  "ceiling_dbtp",
  "bit_depth",
  "target_sample_rate",
] as const;

/// B7 — produce the next `MasteringSettings` after a user edit to
/// `AdvancedSettings`. When the edit changes any shadowed field AND
/// the current `delivery_profile` is non-Custom (which would shadow
/// the typed value), force-flip the profile to Custom so the typed
/// value drives export. All other fields pass through.
///
/// Why a separate helper: pre-extraction this logic lived inside the
/// hook's `updateSettings(prev => ...)` updater closure, which made
/// it unreachable from tests. As a pure function it's mechanically
/// gateable — no React, no Tauri, no mock setup.
///
/// Degenerate case: typing the SAME value the field already holds
/// (e.g. null -> null) is detected as "no diff" and produces no
/// flip. Acceptable because the displayed value didn't change
/// either, so the visual asymmetry can't be observed. (If we ever
/// need "force flip on every shadowed-field interaction even when
/// the value didn't differ," the call site can opt in by writing
/// `setDeliveryProfile("custom")` explicitly — the LoudnessTarget
/// quick-select does this via `shouldFlipToCustomOnLoudnessPick`
/// below.)
export function applyAdvancedWithProfileFlip(
  prev: MasteringSettings,
  advanced: AdvancedSettings,
): MasteringSettings {
  const shadowedChanged = SHADOWED_ADVANCED_KEYS.some(
    (key) => prev.advanced[key] !== advanced[key],
  );
  const nextProfile: DeliveryProfile =
    shadowedChanged && prev.delivery_profile !== "custom"
      ? "custom"
      : prev.delivery_profile;
  return {
    ...prev,
    advanced,
    delivery_profile: nextProfile,
  };
}

/// Delivery profile picker write rule. Selecting a named profile writes
/// that profile's visible delivery defaults into the editable Advanced
/// fields as well as changing the enum. Selecting Custom inherits the
/// currently effective values, so Custom means "keep what I was seeing
/// and let me override it" instead of falling back to stale hidden state.
export function applyDeliveryProfileSelection(
  prev: MasteringSettings,
  profile: DeliveryProfile,
): MasteringSettings {
  if (profile === "custom") {
    return {
      ...prev,
      delivery_profile: "custom",
      advanced: {
        ...prev.advanced,
        lufs_offset_db: effectiveLoudnessTarget(prev),
        ceiling_dbtp: effectiveCeilingDbtp(prev),
        bit_depth: effectiveBitDepth(prev),
      },
    };
  }
  return {
    ...prev,
    delivery_profile: profile,
    advanced: {
      ...prev.advanced,
      lufs_offset_db: DELIVERY_PROFILE_TARGET_LUFS[profile],
      ceiling_dbtp: DELIVERY_PROFILE_CEILING_DBTP[profile],
      bit_depth: DELIVERY_PROFILE_BIT_DEPTH[profile],
    },
  };
}

/// Wire-time overrides applied to a per-track `MasteringSettings`
/// before it's sent to the backend audio chain. Two responsibilities:
///
///   1. **Volume Match is session-level, not per-track.** The VM
///      checkbox state lives on the transport, not on individual
///      track settings — so every chain dispatch overrides
///      `settings.volume_match` with the current transport value
///      (passed as `volumeMatchOverride`). Pre-fix, per-track
///      `settings.volume_match` would persist across track switches
///      and "lose" the VM toggle (hotfix-3 in the Phase A4 series).
///
///   2. **`source_lufs_integrated` injection.** When the user has
///      analyzed a track, its integrated LUFS is in `analysisMap`.
///      Injecting it on the dispatched settings lets the chain's
///      VM cap (B3 fix) bound its attenuation by the limiter
///      ceiling — see `src-tauri/src/dsp.rs` Volume Match block.
///      `null` / non-finite values are skipped so the field stays
///      unset (the chain's fallback path applies).
///
/// Pure-data helper extracted from the `withSourceLufs` closure in
/// `useTrackMaster.ts` so the override behavior is mechanically
/// gateable. The hook glues `volumeMatchRef.current` and
/// `analysisMap` from React state; the rules are tested here.
export function applyChainDispatchOverrides(
  settings: MasteringSettings,
  trackId: TrackId | null,
  analysisMap: Record<TrackId, AnalysisResult>,
  volumeMatchOverride: boolean,
): MasteringSettings {
  const result: MasteringSettings = {
    ...settings,
    volume_match: volumeMatchOverride,
  };
  if (trackId) {
    const sourceLufs = analysisMap[trackId]?.lufs_integrated;
    if (sourceLufs !== undefined && sourceLufs !== null && Number.isFinite(sourceLufs)) {
      result.source_lufs_integrated = sourceLufs;
    }
  }
  return result;
}

/// LoudnessTarget quick-select — should the dropdown pick force a
/// flip to Custom alongside the underlying `lufs_offset_db` write?
///
/// Returns true when:
///   * the user picked a real loudness option (NOT the "custom"
///     dropdown entry, which is a no-op), and
///   * the current delivery profile is non-Custom (so the typed
///     value would otherwise be shadowed by the profile).
///
/// The "Off / Natural" entry writes `lufs_offset_db = null`. Pre-fix,
/// when the user picked it while on a non-Custom profile, the B7
/// auto-flip (which detects value DIFFS) didn't fire because
/// `null -> null` doesn't diff — even though the user's intent had
/// shifted from "use the profile's target" to "no target at all."
/// This helper captures the explicit-pick intent regardless of
/// whether the underlying value differs.
export function shouldFlipToCustomOnLoudnessPick(
  pickedId: string,
  currentProfile: DeliveryProfile,
): boolean {
  if (pickedId === "custom") return false;
  return currentProfile !== "custom";
}

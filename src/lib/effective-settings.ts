// Frontend mirrors of the `MasteringSettings::effective_*` accessors in
// `src-tauri/src/types.rs`. The Rust accessors are the source of truth
// (tested in `types.rs::effective_settings_tests`); these helpers exist
// so the UI displays don't lie about what the chain is targeting when
// a non-Custom DeliveryProfile is shadowing advanced fields.
//
// Why a separate module: the LoudnessTarget readout (App.tsx) was
// reading raw `advanced.lufs_offset_db` for both its display number
// and the dropdown's selected value. When a non-Custom profile was
// active, the chain WAS targeting the profile's value but the readout
// showed "—" — same trust-failure pattern as VM-in-export (B3) and
// B7's write-direction auto-flip-to-Custom, this time in the read
// direction. Extracting the rule lets it live alongside its tests.

import type { MasteringSettings } from "../bindings";
import { DELIVERY_PROFILE_TARGET_LUFS } from "../bindings";

/// The four quick-select options on the LoudnessTarget dropdown.
/// Single source of truth for both the rendered options AND the
/// profileId-lookup that drives the dropdown's selected value.
export const LOUDNESS_PROFILES: ReadonlyArray<{
  id: string;
  label: string;
  lufs: number | null;
}> = [
  { id: "streaming", label: "Streaming default (-14)", lufs: -14 },
  { id: "loud-streaming", label: "Spotify Loud (-11)", lufs: -11 },
  { id: "cd-master", label: "Hot master (-9)", lufs: -9 },
  { id: "off", label: "Off / Natural", lufs: null },
];

/// Effective target LUFS that the chain will actually apply.
///
/// Mirror of `MasteringSettings::effective_target_lufs`. When
/// `delivery_profile !== "custom"`, the profile's target wins —
/// `advanced.lufs_offset_db` is shadowed. When `delivery_profile ===
/// "custom"`, the helper falls through to `advanced.lufs_offset_db`.
/// `null` means "no target" — the chain skips its landing block.
export function effectiveLoudnessTarget(
  settings: MasteringSettings,
): number | null {
  const profileTarget = DELIVERY_PROFILE_TARGET_LUFS[settings.delivery_profile];
  if (profileTarget !== null && profileTarget !== undefined) {
    return profileTarget;
  }
  return settings.advanced.lufs_offset_db ?? null;
}

/// Match a LUFS value to a quick-select dropdown option id. Returns
/// the canonical id ("streaming", "loud-streaming", "cd-master", "off")
/// when the value matches a `LOUDNESS_PROFILES` entry within ±1e-3 LU,
/// otherwise returns "custom" to indicate the value falls outside the
/// quick-select set. Null maps to "off / natural."
export function profileIdForLufs(lufs: number | null): string {
  if (lufs === null) return "off";
  for (const p of LOUDNESS_PROFILES) {
    if (p.lufs !== null && Math.abs(p.lufs - lufs) < 1e-3) return p.id;
  }
  return "custom";
}

/// Aggregate display state for the LoudnessTarget UI block. Computes:
///
///   * `current` — the effective target the chain will use, mirror
///     of `effectiveLoudnessTarget`.
///   * `profileId` — the dropdown's selected value, derived from
///     `current` via `profileIdForLufs`.
///   * `displayText` — the formatted readout above the dropdown
///     ("-14.0", "—" for no target).
///
/// Single pure entry-point so the LoudnessTarget component can pass
/// `settings` in and get everything it needs out, without inline
/// formatting / matching logic.
export interface LoudnessTargetDisplay {
  current: number | null;
  profileId: string;
  displayText: string;
}

export function loudnessTargetDisplay(
  settings: MasteringSettings,
): LoudnessTargetDisplay {
  const current = effectiveLoudnessTarget(settings);
  return {
    current,
    profileId: profileIdForLufs(current),
    displayText: current !== null ? current.toFixed(1) : "—",
  };
}

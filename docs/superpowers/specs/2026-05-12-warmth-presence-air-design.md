# Warmth and Presence/Air Advanced Controls — Design

**Date**: 2026-05-12
**Status**: Approved (Dan), ready for implementation plan
**Slice scope**: Wire the `warmth` and `presence_air` Advanced controls in `MasteringSettings::AdvancedSettings`. Both fields already exist as `Option<f32>` in `types.rs` and as sliders in `AdvancedPanel`; they currently do nothing.

## Problem

`AdvancedSettings::warmth` and `AdvancedSettings::presence_air` are surfaced in the UI but unwired. Their `NumberField` sliders read `0..1` and write to the settings, but `ChainCoeffs::from_settings` never reads them, so the chain ignores them. The UI labels are currently suffixed `"(coming soon)"`.

This slice removes two more `"(coming soon)"` labels and gives the user surgical character control beyond what the eight presets bake in.

## Industry research summary

Extracted from `docs/research/most-recent-mastering-app-research.md` (subagent pass, 2026-05-12):

| Question | Finding | Citation |
|---|---|---|
| Warmth = pure EQ or EQ+saturation blend? | **Pure EQ**, with saturation as a separate downstream stage that the preset already drives. | "Commercial tools layer EQ first... then apply optional saturation as a distinct stage." (§3) |
| Warmth frequency center? | Low-mids ~200–600 Hz. No commercial tool publishes an exact Hz value. | LANDR "Warm" style: "fuller low-mids, intimate top" (§1.1); Sonible smart:limit "Body" tilt control (§1.10) |
| Warmth dB range? | Unspecified publicly. Ozone Stabilizer max boost ≤ 9 dB; existing Warmth preset's low-shelf is +1.5 dB. Industry-typical range ±3–6 dB. | Ozone Stabilizer §1.2 line 66; existing `Preset::Warmth` in `dsp.rs` |
| Presence/Air frequency? | **Two distinct concepts** in some tools: Presence ~2–5 kHz (peak), Air >8–10 kHz (shelf). Sonible smart:limit treats them as separate controls. | Sonible smart:limit "Presence, Air tilt controls" (§1.10 line 169); Mastering The Mix ISOL8 5-band split (§1.12) |
| Air control = EQ or saturation? | **Predominantly pure EQ** (often adaptive STFT-domain in premium tools). Saturation, if present, is layered afterward in the signal chain. | "Air controls are almost always pure EQ + adaptive shaping, not saturation-driven" (§1.2, §1.10) |
| Default value? | **Neutral / 0 dB / off** until the user explicitly increases. No baseline saturation. | LANDR "Balanced" default; Sonible's sound-shaping module "unlocked by Learn pass" (off until set) (§1.1, §1.10) |
| Stacking with other EQ? | **Additive** on top of the primary EQ stage, not replacing. | Signal chain consensus (§3, lines 300–313) |

**Anchor model**: Sonible smart:limit exposes Drive, Bass, Body, Presence, Air as a separate sound-shaping module after the Learn pass. This is the closest commercial analog to a dual warmth/presence-air control set in a single module, and it's a pure-EQ tilt-style control set.

## Design

### Control shape

Both controls are **one-sided sliders** (0 to +N dB, never negative). The labels themselves carry directional semantics ("warmth" means "add warmth"; "air" means "add air") — a negative value would be a *brightness* or *darkness* control with the wrong label. The existing 3-band EQ already lets the user cut those frequencies if needed.

Slider range stays `0..1` in the UI (matching the existing `NumberField` configuration). Internally this maps to `0..+4 dB`.

### Numeric values

| Control | Type | Frequency | Slope/Q | dB at slider 1.0 | Default |
|---|---|---|---|---|---|
| `warmth` | Low-shelf | **300 Hz** | 0.7 | **+4 dB** | None (slider null/0 = no effect) |
| `presence_air` | High-shelf | **10 kHz** | 0.7 | **+4 dB** | None |

Why these specific numbers:

- **Warmth @ 300 Hz**: between the main Low band (200 Hz) and the main Mid band (1.5 kHz), filling the "body" region Sonible labels as a separate control. 300 Hz keeps it distinct enough from the main Low @ 200 Hz that the two shelves act on perceptually different content; users who want very low warmth still have the main Low band.
- **Presence/Air @ 10 kHz**: above the main High shelf (6 kHz), separating "sparkle/air" from "brightness." Users who want to control the 6 kHz region use the main High band; 10 kHz is the "open top" region the research consistently puts in the air/sparkle category.
- **Slope 0.7**: matches the existing 3-band EQ's `BiquadCoeffs::low_shelf(_, _, _, 0.7)` and `high_shelf(_, _, _, 0.7)` calls. Broad, non-ringing shelves, consistent with the rest of the chain.
- **+4 dB max**: conservative musical range. Within the research's ±3–6 dB typical band. Big enough to be audible without being a tonal sledgehammer.
- **One-sided 0..+4**: research-aligned (industry tilt controls treat warmth/air as additive). Cleaner semantics. UI slider configuration stays consistent with the existing `0..1, step 0.05` NumberField.

### Chain placement

Both new biquads sit in **Pass 1** of `MasteringChain::process_frame_inplace`, alongside the existing low/mid/high biquads:

```
input gain → low_shelf (200Hz) → mid_peaking (1.5kHz) → high_shelf (6kHz)
           → warmth_shelf (300Hz)   ← NEW
           → presence_air_shelf (10kHz)   ← NEW
           → [end of pass 1, width transform, pass 2 saturation, limiter, vm, output gain]
```

Adding two more biquads to the per-channel state increases per-frame cost by ~2 multiply-adds plus 2 state reads. Negligible at 44.1/48 kHz on modern hardware.

### Skip-guard for backward compatibility

`BiquadCoeffs::low_shelf` and `high_shelf` already early-return identity when `gain_db.abs() < 1.0e-4`, so the no-touch path is byte-equivalent to the pre-slice behavior without an explicit guard. The slider value just maps cleanly to dB and is handed to the biquad constructor:

```rust
let warmth_db = settings.advanced.warmth
    .unwrap_or(0.0)
    .clamp(0.0, 1.0)
    * 4.0;  // 0..1 -> 0..+4 dB
let warmth = BiquadCoeffs::low_shelf(sr, 300.0, warmth_db, 0.7);
```

When `warmth_db ≈ 0`, the biquad's own early-return produces identity coefficients, the biquad still runs but is mathematically a unit transformation, and existing test outputs are preserved exactly.

The same shape applies to `presence_air` with `BiquadCoeffs::high_shelf(sr, 10_000.0, presence_air_db, 0.7)`.

### ChainCoeffs data model

Two new fields on `ChainCoeffs`:

```rust
pub struct ChainCoeffs {
    pub low: BiquadCoeffs,
    pub mid: BiquadCoeffs,
    pub high: BiquadCoeffs,
    pub warmth: BiquadCoeffs,        // ← NEW
    pub presence_air: BiquadCoeffs,  // ← NEW
    // ... existing fields unchanged
}
```

Two new state fields on `ChannelState`:

```rust
pub struct ChannelState {
    low: BiquadState,
    mid: BiquadState,
    high: BiquadState,
    warmth: BiquadState,        // ← NEW
    presence_air: BiquadState,  // ← NEW
}
```

Two new biquad processing steps in `process_frame_inplace` (Pass 1) and `process_sample` (legacy path).

### Frontend

Drop `"(coming soon)"` from both labels in `AdvancedPanel`:

- `"Warmth (coming soon)"` → `"Warmth"`
- `"Presence/Air (coming soon)"` → `"Presence/Air"`

The slider config (`step 0.05, min 0, max 1, format v.toFixed(2)`) stays unchanged — the 0..1 range maps internally to 0..+4 dB.

## Testing strategy

Unit tests in `dsp.rs::mod tests` (new test cases alongside the existing width tests):

1. **`warmth_default_is_identity`**: settings with `warmth: None` produce a `ChainCoeffs::warmth == BiquadCoeffs::identity()`. Guarantees the no-touch path is byte-equivalent.
2. **`warmth_at_one_lifts_300hz_band`**: drive a synthetic signal containing a 300 Hz tone through the chain with `warmth: Some(1.0)`; assert the output's 300 Hz band energy is higher than the input's. Pin the audible direction.
3. **`presence_air_default_is_identity`**: same as test 1, for the high shelf.
4. **`presence_air_at_one_lifts_10khz_band`**: same as test 2, for 10 kHz.
5. **`chain_coeffs_clamps_warmth_into_range`**: user value 5.0 clamps to 1.0; user value -1.0 clamps to 0.0 (matching the width-clamp pattern).

Integration: no new contract tests required. The existing `presets_produce_distinct_chain_coefficients` test and the real-fixture render tests should continue passing unchanged (warmth and presence_air default to None for all presets — no current preset opts in).

## Out of scope (for this slice)

- **Saturation interaction**: the research is clear that warmth/air are pure-EQ controls; saturation stays preset-driven. Future polish could expose a separate "Saturation Drive" slider as Sonible's smart:limit does — that's a different slice.
- **Adaptive air (Ozone Clarity / STFT-domain)**: Ozone and other premium tools use multiband dynamic shaping for their air controls. This slice ships a static shelf; an adaptive variant is a future enhancement once the static control is in user hands and feedback is in.
- **Splitting `presence_air` into two separate controls** (Presence ~3 kHz peak + Air ~10 kHz shelf): the existing UI field is a single `presence_air`, and the `types.rs` schema is a single `Option<f32>`. Splitting would be a wider refactor of `AdvancedSettings`, plus migration for any persisted sessions. Keeping it as a single high-shelf @ 10 kHz preserves the schema and matches the "air" half of the Sonible/research pair. If user feedback wants a separate presence band, that's a follow-up.
- **Per-preset defaults**: presets continue to set `advanced: AdvancedSettings::default()` (all None). A future polish could let specific presets seed non-zero warmth/air values (e.g. `Preset::Warmth` setting `warmth: Some(0.4)`); that's a preset-rebalancing slice and benefits from Dan's listening notes first.

## Acceptance criteria

- `cargo check --tests` clean.
- `cargo test --lib` adds 5 new tests, all pass. Existing tests untouched.
- `cargo test` full suite passes (still 56 + 5 = 61 expected, or higher if any existing test was previously skipped).
- `npm run build` clean, bundle delta well under 1 KB raw (label-text + no new dependencies).
- Dan can drag the Warmth slider in the running app and hear a low-mid lift; same for Presence/Air at the high end.
- Both `"(coming soon)"` labels are gone from `AdvancedPanel`.

## References

- `docs/research/most-recent-mastering-app-research.md` — industry survey (sections cited inline above).
- `docs/HANDOFF_2026-05-12.md` — P0 §5 "Wire warmth and presence_air."
- `src-tauri/src/dsp.rs` — existing `ChainCoeffs`, `ChannelState`, `BiquadCoeffs`, `MasteringChain` to follow.
- `src-tauri/src/types.rs` — existing `AdvancedSettings.warmth`, `AdvancedSettings.presence_air` (no schema changes needed).
- `src/App.tsx` — existing `AdvancedPanel` NumberField wiring (labels only change).

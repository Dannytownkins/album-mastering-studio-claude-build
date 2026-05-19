# YES Master — 7-Band EQ Expansion Plan-Doc Prompt

**Date:** 2026-05-19
**Slice:** EQ band expansion (4 → 7 user-facing bands)
**Status:** Plan-doc step per ADR 0002 — implementation gated on Dan's plan review
**Audience for the prompt section:** Claude Code (paste the "Prompt for Claude Code" section verbatim)

---

## Context

Following the EQ architecture recon (`EQ_ARCHITECTURE_RECON_2026-05-19.md`), Dan and Vera agreed on a 7-band user-facing EQ layout that:

- Expands from 4 visible nodes to 7 (3 knob-bound macro bands + 4 drag-only surgical bands)
- Preserves the product philosophy: **advanced EQ is a recovery tool, not a creative tool**
- Preserves the layer separation: **presets are proprietary, opaque to user; user offsets are additive on top**
- Defers freq/Q sweep to a future slice (bands first; parametric later if at all)

The plan doc step is required per ADR 0002 (cross-machine plan handoffs). Claude Code writes the plan, Dan reviews, then implementation proceeds with mechanical gates and slow-lane real-fixture verification.

---

## Sequence

1. **Close the eagle-eye audit first.** This 7-band slice does NOT bundle with the audit. The audit closes out the 2026-05-18 evening / 2026-05-19 documentation arc cleanly.
2. **Open this as a new arc.** Paste the prompt below to Claude Code.
3. **Review the plan doc CC produces.** Approve, request changes, or push back on uncertainty.
4. **Implementation only after plan approval.** DSP changes require slow-lane verification (`AMS_RUN_REAL_FIXTURE=1 cargo test`) before merge.

---

## Design summary (for reference)

### Target band layout

| Band | Frequency | Filter type | Default Q | UI surface |
|---|---|---|---|---|
| Sub | 80 Hz | Peaking bell | 0.8 | Visual EQ, drag-only (surgical) |
| Low | 200 Hz | Low-shelf | slope 0.7 (existing) | Knob + Visual EQ mirror (macro) |
| Low-Mid | 400 Hz | Peaking bell | 0.9 (existing) | Visual EQ, drag-only (surgical) |
| Mid | 1500 Hz | Peaking bell | 0.8 (existing) | Knob + Visual EQ mirror (macro) |
| High-Mid | 3500 Hz | Peaking bell | 0.9 | Visual EQ, drag-only (surgical) |
| High | 6000 Hz | High-shelf | slope 0.7 (existing) | Knob + Visual EQ mirror (macro) |
| Air | 12 000 Hz | High-shelf | slope 0.7 | Visual EQ, drag-only (surgical) |

### What stays the same
- The sub-highpass (`sub_highpass`, 22-40 Hz, preset-locked) stays preset-locked. NOT user-facing.
- The Advanced `warmth` shelf at 300 Hz stays in the Advanced panel, unchanged.
- The Advanced `presence_air` shelf at 10 kHz stays in the Advanced panel, unchanged. (The new `Air` band at 12 kHz is distinct from `presence_air`.)
- The 3-knob Tone Shape row stays at exactly 3 knobs (Low / Mid / High).
- The Visual EQ shows USER OFFSETS ONLY. Preset baselines stay invisible.
- Horizontal drag stays disabled (DSP doesn't support variable freq/Q yet).

### What changes
- 3 new biquad stages added to the DSP chain (Sub, High-Mid, Air).
- 3 new fields on `PresetCalibration` (`sub_db`, `high_mid_db`, `air_db`) — all 8 presets default to 0.0 dB.
- 3 new fields on `MasteringSettings` (`eq_sub_db`, `eq_high_mid_db`, `eq_air_db`) — default 0.0.
- Visual EQ component renders 7 nodes with visual hierarchy (macro vs surgical treatment).

---

## Prompt for Claude Code

> 7-band EQ expansion — plan doc, not implementation. Write the plan and stop. Per ADR 0002, this needs to land in repo notes before implementation begins.
>
> **Product philosophy this slice must honor:**
>
> 1. **The advanced EQ is a RECOVERY tool, not a creative tool.** Its job is to let users back off granularly when preset+intensity overcooks their mix in a specific region. It is not a surgical mastering EQ. It is not Pro Tools. Users who want full surgical control should dial intensity to 0 and EQ themselves, or take the master elsewhere.
>
> 2. **Presets are the proprietary product layer.** The preset's internal calibration is NOT user-editable and NOT visible on the Visual EQ. The Visual EQ shows USER OFFSETS only, additive on top of the invisible preset character. This is current behavior — preserve it. Do not add an "effective curve" display, do not surface preset baseline values on the EQ nodes, do not let switching presets move the user's node positions.
>
> 3. **Layer separation:** Preset+Intensity (proprietary character) → 3-knob Tone Shape LOW/MID/HIGH (basic user bridge) → Visual EQ surgical bands (advanced user recovery) → Advanced panel (deeper voicing). Each layer adds onto the previous. The user works in their own layer; the preset's layer is opaque.
>
> **Target layout (7 user-facing bands):**
>
> | Band | Frequency | Filter type | Default Q | UI surface |
> |---|---|---|---|---|
> | Sub | 80 Hz | Peaking bell | 0.8 | Visual EQ, drag-only (surgical) |
> | Low | 200 Hz | Low-shelf | slope 0.7 (existing) | Knob + Visual EQ mirror (macro) |
> | Low-Mid | 400 Hz | Peaking bell | 0.9 (existing) | Visual EQ, drag-only (surgical) |
> | Mid | 1500 Hz | Peaking bell | 0.8 (existing) | Knob + Visual EQ mirror (macro) |
> | High-Mid | 3500 Hz | Peaking bell | 0.9 | Visual EQ, drag-only (surgical) |
> | High | 6000 Hz | High-shelf | slope 0.7 (existing) | Knob + Visual EQ mirror (macro) |
> | Air | 12 000 Hz | High-shelf | slope 0.7 | Visual EQ, drag-only (surgical) |
>
> 3 new biquad stages (Sub, High-Mid, Air). 4 existing stages (Low, Low-Mid, Mid, High) unchanged. The existing sub-highpass (`sub_highpass`, preset-locked) stays preset-locked — it is NOT promoted to the user-facing surface. The existing Advanced shelves (`warmth` at 300 Hz, `presence_air` at 10 kHz) stay where they are — out of scope for this slice. (`Air` at 12 kHz is a new distinct stage from `presence_air` at 10 kHz; they coexist with different frequencies and different UI surfaces. The naming overlap is acceptable but worth flagging in the plan.)
>
> **Chain order extension** (in both `process_frame_inplace` and `process_sample` in `dsp.rs`): insert Sub before Low; insert High-Mid between Mid and High; insert Air after High. Frequency-monotonic order preserved. The existing `warmth` and `presence_air` stages keep their current positions per the chain order documented in `dsp.rs:1815-1816`.
>
> **The plan doc must cover:**
>
> 1. **DSP changes:** exact line ranges in `dsp.rs` for chain order extension, new biquad stage construction, and `ChainCoeffs::from_settings` extension. Confirm frequencies and Q values against existing band conventions.
>
> 2. **`PresetCalibration` extension** (`dsp.rs:280-335`): 3 new fields — `sub_db: f32`, `high_mid_db: f32`, `air_db: f32`. Default values for ALL 8 PRESETS: 0.0 dB across the board for all 3 new fields. This is intentional — neutral defaults preserve current preset character exactly until Dan tunes the new bands in a future listening batch. Verify there are no audible deltas from current behavior on any preset after this change (any drift means the implementation is wrong).
>
> 3. **`MasteringSettings` extension:** 3 new fields — `eq_sub_db: f32`, `eq_high_mid_db: f32`, `eq_air_db: f32`. Default 0.0. Surface through `src/bindings.ts` and the TypeScript state model.
>
> 4. **Runtime mapping extension:** mirror the existing pattern (`dsp.rs:701-720`) for the 3 new bands — `effective_sub_db = preset.sub_db * preset_scale + settings.eq_sub_db`, etc.
>
> 5. **Visual EQ component changes** (`VisualEqPanel.tsx`): extend `BANDS` constant to 7 bands. Implement visual hierarchy:
>    - **Macro bands (Low, Mid, High)** — larger node (existing size or slightly larger), brighter color, possibly an outline ring or anchor halo. Should read instantly as "these are the same as the knobs above."
>    - **Surgical bands (Sub, Low-Mid, High-Mid, Air)** — smaller node, distinct but harmonious color palette (different hue or muted variant). Should read as "these are the surgical layer."
>    - Both types use the same vertical-drag interaction. Horizontal drag stays disabled (DSP doesn't support variable freq/Q yet — that's a future slice).
>    - Color choices should be reviewed against existing design tokens. Don't invent new colors without checking the design system first.
>
> 6. **Visual EQ response curve update** (`VisualEqPanel.tsx:91-116, 173-187`): the approximation curve renderer needs to extend to 7 bands. Keep the existing Gaussian-for-bells and sigmoid-for-shelves approach — just extend to the new bands. This is still an approximation, not actual filter response, per the existing comment at lines 7-12.
>
> 7. **State model changes** (TypeScript): the EQ band state currently lives as 4 flat fields on `MasteringSettings` (`eq_low_db`, `eq_low_mid_db`, `eq_mid_db`, `eq_high_db`). Extend to 7. Update `useTrackMaster.ts` setter, the `onEq` prop signature on `Macros` (`App.tsx`), and any other call sites identified in the recon.
>
> 8. **Knob bindings:** the 3-knob Tone Shape row stays at exactly 3 knobs. Do NOT add knobs for Sub, High-Mid, or Air. The new bands are intentionally drag-only on the Visual EQ.
>
> 9. **Test surface:** which existing tests need extension (preset_distinctness, preset_signature, preset_loudness_balance, contracts.rs at minimum), what new tests need to be added (per-band centered_at_frequency tests like the existing `low_mid_band_centred_at_400hz_with_q_point_9` at `dsp.rs:3394`, and per-band gain-response tests). Slow-lane real-fixture render must produce output within tolerance of pre-change output for every preset (since calibration defaults are 0 dB, the new bands should be acoustically invisible until tuned).
>
> 10. **Commit shape:** propose granularity per the prior `6cbfe67` feedback. Probable shape: (a) DSP + PresetCalibration extension + runtime mapping in one commit, (b) state model + bindings + Visual EQ component changes in one commit, (c) tests in one or two commits depending on natural splits. Slow-lane fixture verification BEFORE the final push.
>
> 11. **Bonus mini-feature to consider riding along:** `PresetCalibration.science_note` is `&'static str` per preset, currently not exposed to the frontend. Plan whether to surface it as a hover/tooltip on the preset orb in the same slice or split it into a tiny separate slice. Either is fine — your judgment.
>
> 12. **Flag any uncertainty in the plan.** Where you're unsure about default Q values, color choices for the new bands, the test tolerance for "acoustically invisible at 0 dB defaults," or how the calibration table extends without breaking saved-preset deserialization, write it as uncertain. Don't paper over.
>
> **Land the plan as `docs/followups/eq-7-band-plan-2026-05-19.md`.** Stop after the plan doc lands. Wait for Dan's review and approval before any code changes. After approval, implement per the plan with mechanical gates throughout and slow-lane verification before merge.
>
> **Hard rules:**
> - Plan doc only, no implementation in this turn.
> - All 8 presets get 0.0 dB for the new bands. No tuning guesses. Listening calls are Dan's.
> - Visual EQ preserves user-offset-only display. Do NOT surface preset baselines.
> - 3 knob-bound bands stay knob-bound. No new knobs.
> - Sub-highpass stays preset-locked. Do NOT promote it to user-facing surface.
> - Don't touch existing `warmth` or `presence_air` stages. Out of scope.
> - No freq/Q sweep this slice. The DSP-doesn't-support-it constraint at `VisualEqPanel.tsx:14-18` stays true.

---

## Post-plan review checklist

When Claude Code returns the plan doc at `docs/followups/eq-7-band-plan-2026-05-19.md`:

1. **Confirm scope adherence.** Plan should NOT propose freq/Q sweep, NOT propose surfacing preset baselines, NOT propose touching `warmth` or `presence_air`, NOT propose adding knobs for the new bands.
2. **Review flagged uncertainties.** Anything CC marked uncertain — Q values, colors, test tolerance, serialization concerns — needs Dan's input before implementation.
3. **Verify the chain order proposal.** Sub before Low; High-Mid between Mid and High; Air after High. Frequency-monotonic.
4. **Verify the all-presets-0-dB-default policy is honored** for the new fields. No preset-character tuning guesses.
5. **Confirm test surface coverage.** Existing tests extended; new per-band centered-at-frequency and gain-response tests added; slow-lane fixture verification noted as required before merge.
6. **Approve or request revisions.** Once approved, CC implements with mechanical gates and slow-lane verification.

---

## Future slices (not this one)

For tracking — these are explicitly OUT OF SCOPE for the 7-band expansion but worth keeping on the radar:

- **Freq + Q sweep on drag-only bands.** Adds horizontal drag and Q control to the 4 surgical bands (Sub, Low-Mid, High-Mid, Air). 3 knob-bound bands stay fixed. Would make the surgical layer fully parametric. DSP refactor required (biquads need to recompute coefficients on every freq/Q change).
- **Warmth-shelf-vs-warmth-saturation naming collision.** `PresetCalibration.warmth` (saturation drive) and `AdvancedSettings.warmth` (300 Hz shelf) share a field name but are different processors. Code-clarity refactor.
- **Per-preset Sub/High-Mid/Air tuning.** After the 7-band expansion lands with neutral 0 dB defaults, a listening batch session tunes the 3 new bands per preset to match each preset's character.
- **`science_note` surfacing as preset orb tooltip.** Already noted as a possible ride-along for this slice. If split out, it's a tiny independent slice.
- **"Dial in your mix before YES Master" product positioning copy.** Goes in README and onboarding. Sets correct expectations for the recovery-not-surgery philosophy.

---

*Plan-doc handoff drafted by Vera, 2026-05-19. Sequence after the eagle-eye audit closes the 2026-05-18 evening arc cleanly.*

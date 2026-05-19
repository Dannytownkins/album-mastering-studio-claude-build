# YES Master — 7-Band EQ Expansion Plan-Doc Prompt

**Date:** 2026-05-19
**Slice:** EQ band expansion (4 → 7 user-facing bands)
**Status:** Plan-doc step per ADR 0002 — implementation gated on Dan's plan review
**Audience for the prompt section:** Claude Code (paste the "Prompt for Claude Code" section verbatim)

---

## Context

Following the EQ architecture recon (`EQ_ARCHITECTURE_RECON_2026-05-19.md`), Dan and Vera agreed on a 7-band user-facing EQ layout that:

- Expands from 4 visible nodes to 7 (3 knob-bound primary bands + 4 drag-only secondary bands)
- Preserves the canonical layer separation: **presets are opaque; user offsets are additive on top**
- Visual hierarchy reflects FUNCTION (primary vs secondary control surface), not editorial guidance about how the bands should be used
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
| Sub | 80 Hz | Peaking bell | 0.8 | Visual EQ, drag-only (secondary) |
| Low | 200 Hz | Low-shelf | slope 0.7 (existing) | Knob + Visual EQ mirror (primary) |
| Low-Mid | 400 Hz | Peaking bell | 0.9 (existing) | Visual EQ, drag-only (secondary) |
| Mid | 1500 Hz | Peaking bell | 0.8 (existing) | Knob + Visual EQ mirror (primary) |
| High-Mid | 3500 Hz | Peaking bell | 0.9 | Visual EQ, drag-only (secondary) |
| High | 6000 Hz | High-shelf | slope 0.7 (existing) | Knob + Visual EQ mirror (primary) |
| Sparkle | 12 000 Hz | High-shelf | slope 0.7 (matches existing shelf convention) | Visual EQ, drag-only (secondary) |

### What stays the same
- The sub-highpass (`sub_highpass`, 22-40 Hz, preset-locked) stays preset-locked. NOT user-facing.
- The Advanced `warmth` shelf at 300 Hz stays in the Advanced panel, unchanged.
- The Advanced `presence_air` shelf at 10 kHz stays in the Advanced panel, unchanged. (The new `Sparkle` band at 12 kHz is distinct from `presence_air`.)
- The 3-knob Tone Shape row stays at exactly 3 knobs (Low / Mid / High).
- The Visual EQ shows USER OFFSETS ONLY. Preset baselines stay invisible.
- Horizontal drag stays disabled (DSP doesn't support variable freq/Q yet).

### What changes
- 3 new biquad stages added to the DSP chain (Sub, High-Mid, Sparkle).
- 3 new fields on `PresetCalibration` (`sub_db`, `high_mid_db`, `sparkle_db`) — all 8 presets default to 0.0 dB.
- 3 new fields on `MasteringSettings` (`eq_sub_db`, `eq_high_mid_db`, `eq_sparkle_db`) — default 0.0.
- Visual EQ component renders 7 nodes with visual hierarchy reflecting primary vs secondary surface.

### Naming change: "Air" → "Sparkle" at 12 kHz

The original draft proposed calling the 12 kHz band "Air." That conflicts with two existing uses of "air" in the codebase:
- `PresetCalibration.air_db` — drives the 6 kHz High band (historical Codex naming)
- `AdvancedSettings.presence_air` — the 10 kHz Advanced shelf

A third "air" reference at 12 kHz would muddy the disambiguation. The 12 kHz band is renamed to **Sparkle** in the plan. Field names: `PresetCalibration.sparkle_db`, `MasteringSettings.eq_sparkle_db`. If CC has a compelling alternative (`top`, `shimmer`, etc.), flag it in the plan; the rename itself is non-negotiable.

---

## Prompt for Claude Code

> 7-band EQ expansion — plan doc, not implementation. Write the plan and stop. Per ADR 0002, this needs to land in repo notes before implementation begins.
>
> **Canonical product layer (the part that's load-bearing):**
>
> 1. **Layer separation:** Preset+Intensity (proprietary character, opaque to user) → 3-knob Tone Shape LOW/MID/HIGH (primary user surface) → Visual EQ secondary bands (additional user surface) → Advanced panel (deeper voicing). Each layer adds onto the previous. The user works in their own layer; the preset's layer is opaque.
>
> 2. **The Visual EQ shows USER OFFSETS only.** Additive on top of an invisible preset baseline. Preserve this. Do NOT add an "effective curve" display, do NOT surface preset baseline values on the EQ nodes, do NOT let switching presets move the user's node positions.
>
> 3. **Primary vs secondary surface — functional hierarchy, not editorial.** Knob-bound bands (Low/Mid/High) are the primary surface — they have two control surfaces (knob and Visual EQ node) because they're the bands most users will touch. Drag-only bands (Sub/Low-Mid/High-Mid/Sparkle) are the secondary surface — Visual EQ only. **Both kinds are legitimate user-shaping territory.** Users do what serves the track. Visual hierarchy reflects "primary vs secondary control surface," not editorial guidance about how the bands should be used.
>
> **Target layout (7 user-facing bands):**
>
> | Band | Frequency | Filter type | Default Q | UI surface |
> |---|---|---|---|---|
> | Sub | 80 Hz | Peaking bell | 0.8 | Visual EQ, drag-only (secondary) |
> | Low | 200 Hz | Low-shelf | slope 0.7 (existing) | Knob + Visual EQ mirror (primary) |
> | Low-Mid | 400 Hz | Peaking bell | 0.9 (existing) | Visual EQ, drag-only (secondary) |
> | Mid | 1500 Hz | Peaking bell | 0.8 (existing) | Knob + Visual EQ mirror (primary) |
> | High-Mid | 3500 Hz | Peaking bell | 0.9 | Visual EQ, drag-only (secondary) |
> | High | 6000 Hz | High-shelf | slope 0.7 (existing) | Knob + Visual EQ mirror (primary) |
> | Sparkle | 12 000 Hz | High-shelf | slope 0.7 (matches existing shelf convention) | Visual EQ, drag-only (secondary) |
>
> 3 new biquad stages (Sub, High-Mid, Sparkle). 4 existing stages (Low, Low-Mid, Mid, High) unchanged. The existing sub-highpass (`sub_highpass`, preset-locked) stays preset-locked — NOT promoted to the user-facing surface. The existing Advanced shelves (`warmth` at 300 Hz, `presence_air` at 10 kHz) stay where they are — out of scope for this slice.
>
> **Naming requirement:** the 12 kHz band is **Sparkle**, not "Air." The codebase already has `PresetCalibration.air_db` (6 kHz High band baseline, historical Codex name) and `AdvancedSettings.presence_air` (10 kHz Advanced shelf). A third "air" reference at 12 kHz would muddy the disambiguation. Use field names `sparkle_db` and `eq_sparkle_db`. If you'd argue for a different name (e.g. `top`, `shimmer`), flag it in the plan — but Sparkle is the default unless your case is compelling.
>
> **Chain order extension** (in both `process_frame_inplace` and `process_sample` in `dsp.rs`): insert Sub before Low; insert High-Mid between Mid and High; insert Sparkle after High. Frequency-monotonic order preserved. The existing `warmth` and `presence_air` stages keep their current positions per the chain order documented in `dsp.rs:1815-1816`.
>
> **The plan doc must cover:**
>
> 1. **DSP changes:** exact line ranges in `dsp.rs` for chain order extension, new biquad stage construction, and `ChainCoeffs::from_settings` extension. Confirm frequencies and Q values against existing band conventions.
>
> 2. **`PresetCalibration` extension** (`dsp.rs:280-335`): 3 new fields — `sub_db: f32`, `high_mid_db: f32`, `sparkle_db: f32`. Default values for ALL 8 PRESETS: 0.0 dB across the board for all 3 new fields. This is intentional — neutral defaults preserve current preset character exactly until Dan tunes the new bands in a future listening batch.
>
> 3. **`MasteringSettings` extension:** 3 new fields — `eq_sub_db: f32`, `eq_high_mid_db: f32`, `eq_sparkle_db: f32`. Default 0.0. Surface through `src/bindings.ts` and the TypeScript state model.
>
> 4. **Runtime mapping extension:** mirror the existing pattern (`dsp.rs:701-720`) for the 3 new bands — `effective_sub_db = preset.sub_db * preset_scale + settings.eq_sub_db`, etc.
>
> 5. **Visual EQ component changes** (`VisualEqPanel.tsx`): extend `BANDS` constant to 7 bands. Implement visual hierarchy that reflects the functional difference between primary and secondary surface:
>    - **Knob-bound bands (Low, Mid, High) — primary surface.** Larger node (existing size or slightly larger), brighter color, possibly an outline ring or anchor halo. Should read instantly as "these are the same as the knobs above."
>    - **Drag-only bands (Sub, Low-Mid, High-Mid, Sparkle) — secondary surface.** Smaller node, distinct but harmonious color palette (different hue or muted variant). Should read as "these are the additional bands available only on the Visual EQ."
>    - **Both kinds are legitimate user-shaping territory.** The visual differentiation reflects "primary vs secondary control surface," not "use these vs use those." Users do what serves the track.
>    - The differentiation between primary and secondary should be clear at a glance — don't make it so subtle that the bands read as the same tier. Same vertical-drag interaction for both. Horizontal drag stays disabled (DSP doesn't support variable freq/Q yet — that's a future slice).
>    - Color choices should be reviewed against existing design tokens. Don't invent new colors without checking the design system first.
>
> 6. **Visual EQ response curve update** (`VisualEqPanel.tsx:91-116, 173-187`): the approximation curve renderer needs to extend to 7 bands. Keep the existing Gaussian-for-bells and sigmoid-for-shelves approach — just extend to the new bands. This is still an approximation, not actual filter response, per the existing comment at lines 7-12.
>
> 7. **State model changes** (TypeScript): the EQ band state currently lives as 4 flat fields on `MasteringSettings` (`eq_low_db`, `eq_low_mid_db`, `eq_mid_db`, `eq_high_db`). Extend to 7. Update `useTrackMaster.ts` setter, the `onEq` prop signature on `Macros` (`App.tsx`), and any other call sites identified in the recon.
>
> 8. **Knob bindings:** the 3-knob Tone Shape row stays at exactly 3 knobs. Do NOT add knobs for Sub, High-Mid, or Sparkle. The new bands are intentionally drag-only on the Visual EQ.
>
> 9. **Test surface:** which existing tests need extension (preset_distinctness, preset_signature, preset_loudness_balance, contracts.rs at minimum), what new tests need to be added (per-band centered_at_frequency tests like the existing `low_mid_band_centred_at_400hz_with_q_point_9` at `dsp.rs:3394`, and per-band gain-response tests). **Slow-lane gate is byte-identical fixture output**, not "within tolerance" — same gate the wav_writer lift used in commits 1-2 of the engine.rs split. Since all 8 presets default the 3 new bands to 0.0 dB and the new biquads at 0 dB are identity, the slow-lane render must produce byte-identical WAV output to pre-change for every preset. If a byte differs, the implementation is wrong.
>
> 10. **Commit shape:** propose granularity per the prior `6cbfe67` feedback. Probable shape: (a) DSP + PresetCalibration extension + runtime mapping in one commit, (b) state model + bindings + Visual EQ component changes in one commit, (c) tests in one or two commits depending on natural splits. Slow-lane fixture verification BEFORE the final push.
>
> 11. **Flag any uncertainty in the plan.** Where you're unsure about:
>     - Default Q values, especially Sparkle's slope at 12 kHz (currently proposed at 0.7 to match the existing 6 kHz / 10 kHz shelf convention; argue for slope 0.5 if you think 0.7 is too tight at 12 kHz, but the conservative default is matching existing convention)
>     - Color choices for the new bands
>     - The Sparkle naming (if you'd argue for an alternative, say so)
>     - How the calibration table extends without breaking saved-preset deserialization (serde defaults, field ordering, version migration)
>     - Anything else you don't have evidence to commit to
>
>     Write it as uncertain. Don't paper over.
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
> - 12 kHz band is Sparkle, not Air. Use field names `sparkle_db` / `eq_sparkle_db`.
> - No freq/Q sweep this slice. The DSP-doesn't-support-it constraint at `VisualEqPanel.tsx:14-18` stays true.

---

## Post-plan review checklist

When Claude Code returns the plan doc at `docs/followups/eq-7-band-plan-2026-05-19.md`:

1. **Confirm scope adherence.** Plan should NOT propose freq/Q sweep, NOT propose surfacing preset baselines, NOT propose touching `warmth` or `presence_air`, NOT propose adding knobs for the new bands.
2. **Review flagged uncertainties.** Anything CC marked uncertain — Q values (especially Sparkle's slope), colors, serialization concerns, naming alternatives — needs Dan's input before implementation.
3. **Verify the chain order proposal.** Sub before Low; High-Mid between Mid and High; Sparkle after High. Frequency-monotonic.
4. **Verify the all-presets-0-dB-default policy is honored** for the new fields. No preset-character tuning guesses.
5. **Confirm test surface coverage.** Existing tests extended; new per-band centered-at-frequency and gain-response tests added; **slow-lane byte-identical fixture verification** noted as required before merge.
6. **Confirm Sparkle naming is honored** (or CC pushed back with a compelling alternative).
7. **Approve or request revisions.** Once approved, CC implements with mechanical gates and slow-lane verification.

---

## Future slices (not this one)

For tracking — these are explicitly OUT OF SCOPE for the 7-band expansion but worth keeping on the radar:

- **Freq + Q sweep on drag-only bands.** Adds horizontal drag and Q control to the 4 secondary bands (Sub, Low-Mid, High-Mid, Sparkle). 3 primary bands stay fixed. Would make the secondary surface fully parametric. Implementation is standard biquad recompute on slider change — same code path that already runs on preset/intensity changes. Lower friction than a heavy DSP refactor.
- **Per-preset Sub/High-Mid/Sparkle tuning.** After the 7-band expansion lands with neutral 0 dB defaults, a listening batch session tunes the 3 new bands per preset to match each preset's character.
- **Warmth-shelf-vs-warmth-saturation naming collision.** `PresetCalibration.warmth` (saturation drive) and `AdvancedSettings.warmth` (300 Hz shelf) share a field name but are different processors. Code-clarity refactor.
- **`science_note` surfacing as preset orb tooltip.** Tiny independent slice — do this AFTER the 7-band expansion lands, not as a ride-along. Bundling it into the EQ slice would muddy the commit shape.
- **Product positioning copy.** Goes in README and onboarding. Sets expectations that YES Master is a mastering app — bring a finished mix, the app polishes it. Doesn't editorialize about how users should use the Visual EQ.

---

*Plan-doc handoff drafted by Vera, 2026-05-19. Revised same day to drop "recovery vs creative" framing in favor of functional primary/secondary surface hierarchy. Sequence after the eagle-eye audit closes the 2026-05-18 evening arc cleanly.*

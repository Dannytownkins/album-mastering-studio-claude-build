# Review Checkpoint — 2026-05-14 (pre-preset-retune)

Reviewer: fresh Claude session. HEAD: `cdfa54c`. Build verification (read-only): `cargo test --lib` → **81/81 pass** in 0.6 s. (Note: handoff says "80/80" — count drifted by one, harmless but worth refreshing on next handoff edit.) `cargo check` clean. No code modified.

This is the first checkpoint, so file-size baselines are recorded here for next-time growth comparisons. No prior checkpoint to carry forward from.

---

## 1. State of the build

YES Master is in a coherent shape: the rename landed cleanly, the 1920×1080 console layout is settled, the audio thread now meters Original playback the same way Mastered is metered, and the test suite is green at 81 lib + ~138 integration. The dominant area of debt is **preset character is captured in data but not applied in the chain**: per-preset compressor threshold/ratio, target LUFS, and transient punch are all sitting in `PresetCalibration` and never reach `from_settings`. This is exactly what the queued P1–P6 workstream addresses, so debt is well-targeted, not drifting. Secondary debt is the front-end mega-hook (`useTrackMaster.ts` 1,501 lines) flowing into `App.tsx` (2,438 lines) — every PlaybackTick sweeps the whole app — but that's Codex's lane and not blocking.

## 2. Handoff drift

| Handoff claim | Reality | Note |
|---|---|---|
| `cargo test --lib` 80/80 | 81/81 today | Test count grew by one since `6a441d9`. Refresh the number in the next handoff. |
| `compressor_threshold_dbfs` / `compressor_ratio` are captured but not applied | Verified. `dsp.rs:582–680` `ChainCoeffs::from_settings` consumes EQ, saturation, width, gain, ceiling but not compressor fields. | Accurate — drives the next workstream. |
| `engine.rs:1188 — let energy_density = 0.5_f32` hardcoded in album export | Verified at engine.rs:1188; the literal is there, neighbouring code at engine.rs:1152 already has decoded `pcm` available. | Accurate. |
| `engine.rs:1437–1438 — INT16/INT24 scales asymmetric` | Verified: `INT16_SCALE = 32_767.0` / `INT24_SCALE = 8_388_607.0` (line numbers exact). | Accurate. |
| `dsp.rs:1042–1065 — limiter peak scan O(N) with Lagrange ISP` | Verified. Linear scan at line 1042; Lagrange-4 across all adjacent pairs at 1049–1064. | Accurate. |
| `engine.rs:1403–1422 — DitherRng is a single shared u32 across L/R` | Verified. `DitherRng { state: u32 }` at line 1403; in `write_dithered_samples` (engine.rs:1477) the same `rng` is threaded through L then R draws sequentially. | Accurate. |
| `MasteringSource::new` gained a 9th argument `spectrum_ring` | Verified at audio.rs:1170 (call site) and the struct around audio.rs:1350. | Accurate. |
| `MeteredPcmSource` exists for Original metering | Verified at audio.rs:1213–1314. | Accurate. |
| `tauri.conf.json` 1920×1080 logical / minWidth 1440 / minHeight 860 | Not re-verified inline this pass, but the relevant commits (`1ea2fa5`, `fc7fa9f` revert) match the claimed shape. | Trusted given the explicit revert commit. |

No drift between the handoff and the code. The handoff is honest.

## 3. Handoff revision drift

The active handoff has 5 revisions: `2ba39b2` (creation) → `63f452f` (DSP audit findings added) → `b977a23` → `d5be131` → `cdfa54c` (DPI clarifications).

- The DPI revisions (`b977a23`, `d5be131`, `cdfa54c`) all track real Codex code commits (`a09fe28`, `fc7fa9f`) that landed-and-reverted. Doc state mirrors final code state.
- `63f452f` adds the "DSP Debt" section, which is a documentation pass over an external second-opinion audit. It's not backed by a *new* code commit, but the claims it adds are claims about *existing* code — and I verified each one against source in pass 2. Not a doc-only-fiction case.
- No claim was added to the handoff that isn't either backed by an existing code commit or backed by code that was already in the tree.

No revision drift.

## 4. Canon drift

Sampled `docs/PRODUCT.md` against implementation. Material deviations:

- **Intentional + undocumented**: The PRODUCT.md preset list (line 174) names "Loud or Energy"; the code ships `Loud` only. Not a bug, but the canon should probably retire the "or Energy" alt or pick one.
- **Intentional + documented**: PRODUCT.md still references "Python mastering engine" / "Python sidecar" (lines 412–456) as the incumbent. The current build is fully Rust — no Python sidecar in this repo. The Python text is from the original Codex-era canon and is now historical context, not a current architectural commitment. Consider trimming on the next canon refresh.
- **Intentional + undocumented**: PRODUCT.md "Universal-First Workflow" (line 156) names the locked path as Drop → Analyze → Universal → Export; the current Track Master flow is closer to Drop → Auto-analyze → Preset tile → Adjust → Export. Universal is the default *preset*, but there's no separate "Universal settings" step distinct from preset selection. Probably fine — Universal-as-default-preset satisfies the spirit — but the wording could be reconciled.
- **Intentional + documented**: P1's compressor rule (preset compressor as base × user `compression_density` scale) is documented in the analysis doc (line 185) but isn't yet in PRODUCT.md. Will become canon-worthy after P2 lands; not drift today.

No accidental deviations found.

## 5. Hardcoded fallback audit

Searched for numeric literals assigned to analysis-shaped names + comments containing "default," "neutral," "for now," "placeholder."

Findings:

- **engine.rs:1188** — `let energy_density = 0.5_f32;` — already in the open queue as item #1; reaffirmed.
- **No other suspect literals found.** The other `0.0_f32` / `0.5_f32` matches in `audio.rs`, `dsp.rs`, `engine.rs` are accumulator initializers for sums, not analysis-shape stand-ins.
- **Zero TODO / FIXME / HACK markers in `src-tauri/src/`** — clean. (Whether that's because issues are actually resolved or because nobody writes TODOs in this repo is a separate question; the code is at least free of explicit known-debt markers.)
- The DitherRng seed `0xA11_CE` and the album-passthrough's `0.5` for AlbumArc::Custom curve_value (engine.rs:1184) are intentional defaults documented in nearby comments — not silent placeholders.

## 6. Architecture health (baseline for next checkpoint)

Recording current sizes as the **growth baseline**. No prior checkpoint, so no growth flags this round.

Rust (`src-tauri/src/`):

| File | Lines | Cohesion verdict |
|---|---|---|
| dsp.rs | 3,293 | **OK.** One responsibility — DSP chain (biquads, M/S widener, saturation, multiband comp, limiter, preset table). Large but cohesive. |
| audio.rs | 2,122 | **Suspect — split candidate.** Mixes (a) SpectrumRing/Analyzer, (b) PCM decoding, (c) AudioPlayer command surface, (d) audio-thread loop, (e) `MeteredPcmSource` + `MasteringSource` rodio sources. Five concerns. Splitting along the `// MasteringSource —` boundary at line 1196 would give a clean ~1,200/~900 split. Not urgent. |
| engine.rs | 2,033 | **OK.** Render orchestration + dither + LUFS measurement. Single concern (offline render path) with helpers. |
| album.rs | 893 | **OK.** |
| types.rs | 799 | **OK.** Type definitions. |
| exports.rs | 157 / files.rs 85 / lib.rs 83 / project.rs 98 / settings.rs 86 / main.rs 5 / jobs.rs 2 | **OK.** |

Frontend (`src/`):

| File | Lines | Cohesion verdict |
|---|---|---|
| App.tsx | 2,438 | **Suspect.** Already flagged in handoff. Codex's lane. |
| useTrackMaster.ts | 1,501 | **Suspect.** Single mega-hook driving full app state from PlaybackTick. Codex's lane. |
| RightRail.tsx | 626 | **Borderline.** Watch for growth. |
| VisualEqPanel.tsx | 403 / Knob.tsx 366 / SignalChain.tsx 284 / AlbumPanel.tsx 164 / PresetIcon.tsx 91 | **OK.** |
| bindings.ts | 339 | **OK** (generated-shape). |

No growth deltas this checkpoint (baseline). Next checkpoint: flag any of these >20% larger.

## 7. Test grading

Surveyed integration tests. Test names + the assertions reviewed inline:

- **`preset_signature.rs`** — **correctness.** Asserts Goertzel band-tilt deltas at named frequencies, Tape/Warmth third-harmonic rise above input floor, Spatial side-RMS lift on antiphase input. Each assertion is a numeric spec, not a "did it run" check.
- **`preset_loudness_balance.rs`** — **correctness.** Asserts integrated LUFS spread across all presets stays within a stated LU window.
- **`dither_absence_of_harmonics.rs`** — **correctness.** Asserts spectral floor properties of dithered output.
- **`delivery_profile_render.rs`** — **correctness.** Numeric LUFS / true-peak landings per profile.
- **`album_arc_trace.rs` / `album_character_bias.rs` / `album_render.rs`** — **correctness for arc-driven values, behavior for end-to-end render.** Album character-bias tests assert directional changes per role; render tests assert files exist + measured values are finite + within ranges. Defensible.
- **`contracts.rs`** (1,806 lines, 4 real-fixture tests gated behind `AMS_RUN_REAL_FIXTURE`) — **mixed.** The fast-lane tests check command surfaces (return shapes, error variants); the gated real-fixture tests grade actual measured LUFS / true-peak / spectrum on Dan's audio.

**No DSP function under behavior-only test coverage.** Tests grade what they're shaped to grade. Caveat: **no test exists for the album-export `energy_density` plumbing** — that's exactly why item 1 went undetected; the proposed regression test in open-queue item #1 would close the gap.

## 8. Frontend debt

Scoped to PlaybackTick path:

- The PlaybackTick listener lives in `useTrackMaster.ts:163–193` and writes into `transport` state via `setTransport`. Every subscriber to `useTrackMaster()` re-renders ~50× per second when playing.
- App.tsx's main component consumes `useTrackMaster()` directly; everything inside re-renders per tick. Heavy children include the waveform deck, EQ panel, spectrum viz, meters. **Real cost.**
- **No `React.memo` anywhere in `src/`** — confirmed, grep returned zero matches.
- **No stale-closure risks found** in the playback-tick path itself; the PlaybackTick handler closes over only `setTransport`/`setLoadedTrackId`, both stable setters.
- No `as unknown as T` casts outside `preview-mock.ts` (not searched exhaustively, but `bindings.ts` is generated).

This is Codex's lane per the handoff — flagging only, not actionable from Claude side.

## 9. Real bugs

**B1.** `engine.rs:1188` — album-export path discards per-track `energy_density` and passes literal `0.5` to `apply_album_shadow`. The PCM is already decoded one line up at engine.rs:1152, so `crate::engine::compute_energy_density_score(&pcm.samples, ...)` could feed it directly. Net effect today: the album-arc character-bias presence-band energy-gate is **dead in the album EXPORT path** while the analysis path uses the real value. Already in the open queue as item #1; reaffirmed here.

**B2.** `engine.rs:1437–1438` — `INT16_SCALE = 32_767.0` / `INT24_SCALE = 8_388_607.0`. With `clamp(-1.0, 1.0) * SCALE`, the most-negative integer (`-32_768` / `-8_388_608`) is unreachable; output range is asymmetric by 1 LSB. Audibly inconsequential (<-90 dB FS DC offset), but technically incorrect. Already in the open queue as item #7a.

No other broken-broken bugs found this pass. The DSP audit items 3, 4, 5, 6 are refinements / perf, not silent miscomputes.

## 10. Push-back list

Two items worth surfacing for operator decision; everything else passed without warranting pushback.

- **PB1. `useTrackMaster` mega-hook → mega-rerender on PlaybackTick.** The handoff classifies this as Codex's lane, but the *root cause* lives at the audio command boundary, not in any UI file. Consider routing PlaybackTick through a separate small subscriber (e.g. a dedicated meter/transport store) so the mega-hook isn't re-renderingly coupled to 50 Hz tick traffic. That's a refactor that *enables* Codex's split work rather than competing with it. Worth a Dan call before next session.
- **PB2. `transient_punch` field has been "captured but not applied" since Phase A2 alongside `compressor_*`.** P5 (transient shaper) is implied by the "Codex listening calibration" history but isn't on the active queue. If the preset retune lands without any transient-shaper movement, the Punch preset's distinctness contract test (handoff P4: "Punch preserves more transient movement") could only land via compressor settings — which arguably contradicts what "punch" means perceptually. Worth deciding before P4 if a transient shaper goes into scope or if the contract test gets reworded.

## 11. Top priorities for next session

The preset retune workstream (P1–P6) is the right next move and is well-scoped in the handoff. No reason to re-prioritise. Two suggested ordering refinements:

1. **Run P4 first as a failing test.** Write `preset_distinctness.rs` with the contract assertions (Universal-vs-Clarity, Universal-vs-Oomph, Universal-vs-Tape, Punch-vs-Loud) before touching `from_settings`. Confirm it fails on current code. Then retune (P1+P2+P3) until it passes. This makes the spec the gate, exactly per the handoff line 134 ("the test is the spec").
2. **Resolve PB2 before P4.** Decide whether the Punch-vs-Loud crest-factor distinctness assertion goes in this pass (with whatever movement compressor settings can give) or waits until a transient shaper exists. Either is defensible — but make the call up front so P4's assertions are honest.

The album-export energy_density bug (B1) is a real correctness bug but small and isolated; pick it up after the retune ships, per the existing queue.

---

End of checkpoint. Reviewer recommendation: continue per existing handoff plan with the two refinements above. No emergency fixes required this session.

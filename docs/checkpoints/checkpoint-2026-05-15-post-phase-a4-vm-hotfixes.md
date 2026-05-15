# Review Checkpoint — 2026-05-15 (post-Phase-A4 + 3 VM hotfixes)

Reviewer: fresh Claude session. HEAD: `4b9b7e9`. Build verification (read-only): `cargo check` clean in 11.7 s; `cargo test --lib` → **81/81 pass** in 0.4 s. No code modified.

Prior checkpoint: `checkpoint-2026-05-14-pre-preset-retune.md`. This checkpoint follows the Phase A4 retune ship + 3 VM hotfixes (commits `243ca18`, `b4c2a57`, `1b21172`, `51477a4`).

---

## 1. State of the build

YES Master is in a strong shape post-Phase-A4. The preset retune wired the per-preset multiband compressor (threshold/ratio/attack/release scaled by the user's `compression_density` macro) and tuned all 9 preset constants to the analysis-doc's conservative-target values; Dan's listening pass confirmed presets are "distinct from one another, match their name." The Volume Match feature went through three hotfixes in one session because wiring the compressor surfaced a long-latent VM bug whose old `attenuation = source_lufs - target_lufs` math depended on a captured-but-not-applied field (`target_lufs`). The new math estimates chain push from real gain stages (input gain + avg compressor makeup + saturation correction + output trim) and lands within ~1 dB across all presets — but the change is wide-surface enough that hotfix-3's session-level wire-time override (via `useRef`) is not yet verified by ear. The dominant area of debt today is **export-path correctness for VM**: PRODUCT.md line 262 says "Export level is unchanged" by Volume Match, but the new VM scalar is applied unconditionally in `process_frame_inplace` regardless of whether the consumer is the playback chain or the render path. Secondary debt is a previously-undetected `ISO_PLACEHOLDER` constant used as the timestamp for every render/analysis report field. Both are surfaceable, narrow fixes; neither blocks the listening-verification workstream the handoff already names as top priority.

## 2. Handoff drift

Verified every load-bearing claim in `HANDOFF_2026-05-15_session.md` against code:

| Handoff claim | Reality | Note |
|---|---|---|
| Preset compressor wired in `ChainCoeffs::from_settings` (density 0/0.5/1.0 mapping) | Verified at [dsp.rs:768-788](src-tauri/src/dsp.rs:768). Custom defaults to density 0.0, non-Custom presets default 0.5, density 1.0 adds OVERDRIVE_THRESHOLD_DB=-3.0 and OVERDRIVE_RATIO=+0.5. | Accurate. |
| All 9 PRESET_* constants retuned to conservative-target values | Verified at [dsp.rs:336-530](src-tauri/src/dsp.rs:336). Values match `PRESET_REFERENCE_ANALYSIS_2026-05-14.md` table lines 252-259 (Universal -16 dBFS / 1.8:1; Tape -22/2.4:1; Loud -23/3.5:1; etc.). New `compressor_attack_ms` / `compressor_release_ms` fields are populated per preset. | Accurate. |
| `preset_distinctness.rs` with 4 distinctness + 1 safety contract | Verified — 4 contract tests at [preset_distinctness.rs:269-371](src-tauri/tests/preset_distinctness.rs:269) + the hot-source safety pass at [preset_distinctness.rs:388-408](src-tauri/tests/preset_distinctness.rs:388). | Accurate. Thresholds softened from analysis-doc values (Clarity -1.0→-0.4, Oomph low-mid -2.0→-1.0, Clarity air +0.8→+0.4) with the structural-limit note explaining why. |
| VM source-LUFS injection (hotfix-1) and new chain-push math (hotfix-2) | Verified at [dsp.rs:887-899](src-tauri/src/dsp.rs:887). Formula is `chain_push = input_gain_db + avg_makeup_db + 5×saturation_amount + user_output_gain_db`, clamped to [-24, 0]. Comment block above documents the rationale, the prior bug's empirical 0.5–4.3 dB error, and the new ~1 dB accuracy. | Accurate. |
| Compressor `powf` skipped on quiet frames + `exp(g · LN10/20)` swap (hotfix-1 perf) | Verified — early-return when `gr_db <= 0`, no `powf` in the active branch (uses `exp(g * (LN10/20))` via the `ln10_over_20` constant). | Accurate. |
| Limiter Lagrange-4 ISP loop skipped on quiet frames (hotfix-1 perf) | Verified at [dsp.rs:1143-1145](src-tauri/src/dsp.rs:1143). `ISP_SKIP_MARGIN = 1.2` (≈+1.6 dB margin); ISP loop guarded by `peak * ISP_SKIP_MARGIN > self.ceiling_lin`. | Accurate. |
| VM is session-level via wire-time override + `useRef` for synchronous reads (hotfix-3) | Verified at [useTrackMaster.ts:190-198](src/hooks/useTrackMaster.ts:190) (ref declared + sync useEffect + chain override) and [useTrackMaster.ts:1206-1207](src/hooks/useTrackMaster.ts:1206) (synchronous ref write before `setTransport`). | Accurate. |
| `cargo test --lib` 81/81; `cargo test` 144/144 fast lane | `cargo test --lib` confirmed 81/81 in 0.40 s; cargo check clean. Full `cargo test` not re-run this checkpoint (no DSP/WAV changes since handoff). | Accurate. |
| useTrackMaster.ts grew "~30 lines for the VM ref + override" | Actual delta is 56 lines (1,501 → 1,557). Handoff likely counted only the new ref/effect/override block; the `setVolumeMatch` setter rewrite added more. | **Minor drift** — handoff rounded down by ~25 lines. No bearing on correctness; flag for next handoff edit. |
| Dan did not get to verify hotfix-3 before handoff was written | Implicit in code state (no Dan-side listening note for hotfix-3); the "Primary workstream" section is explicit. | Accurate. |

No false claims. The handoff is honest. One minor numerical-rounding slip (line-count of the VM-ref change) noted above.

## 3. Handoff revision drift

`HANDOFF_2026-05-15_session.md` was created in a single commit (`4b9b7e9`) — no revisions to drift between.

`HANDOFF.md` (the entry-point doc) has 5 revisions since the prior checkpoint baseline (`cdfa54c`):

- The Phase A4 + VM hotfix summary added in `4b9b7e9` is backed by the four code commits `243ca18` / `b4c2a57` / `1b21172` / `51477a4` from the same session. All line-level claims in the new snapshot paragraph (compressor wiring, VM math, session-level VM) trace to code I verified in Pass 1.
- The carryover items at the bottom of the snapshot ("audio thread reply timeout," "structural-limit follow-up," "export should strip VM") are described as "newly-surfaced this session, not yet queued formally" — they are observations about existing code, not claims of work done. Honest.

No revision drift.

## 4. Canon drift

Sampled `docs/PRODUCT.md` (the YES Master canon) against implementation.

- **Accidental (NEW): Volume Match applies in the export path.** PRODUCT.md line 262 / Locked Decision #22 says "Volume Match is optional and off by default" and the tooltip text says "Export level is unchanged." But `process_frame_inplace` at [dsp.rs:1713-1717](src-tauri/src/dsp.rs:1713) applies `volume_match_gain_lin` whenever it deviates from 1.0, regardless of consumer. The render path at [engine.rs:895](src-tauri/src/engine.rs:895) constructs `MasteringChain::new(..., settings)` — if the operator has VM toggled on at render time and `settings.volume_match = true`, the exported WAV is attenuated by the new chain-push estimate, off-target from `effective_target_lufs()` by 0-1 dB. The post-LUFS-landing block at [engine.rs:1223-1232](src-tauri/src/engine.rs:1223) compensates back toward the per-preset `target_lufs`, but only for the track-render path; the album path's per-track LUFS landing block at [engine.rs:1230](src-tauri/src/engine.rs:1230) likewise compensates only after the VM scalar has already affected the frame. Net effect: spec violation. See B3 below.
- **Intentional + undocumented (carryover from 2026-05-14)**: PRODUCT.md line 174 names "Loud or Energy" as a preset choice; code ships `Loud` only. Same as prior checkpoint; the canon should resolve the alt or pick one on the next refresh.
- **Intentional + documented (carryover)**: PRODUCT.md still references the Python engine / Python sidecar (lines 412-456). The Claude build has no Python sidecar — this is historical text from the canon's Codex-era origin. Not currently drift in spirit, but the wording is misleading. Same flag as last checkpoint.
- **Intentional + undocumented (carryover)**: PRODUCT.md "Universal-First Workflow" (line 156) names the locked path as Drop → Analyze → Universal → Export. Today's Track Master flow is closer to Drop → Auto-analyze → Preset tile → Adjust → Export. Universal-as-default-preset satisfies the spirit. Same flag as last checkpoint.

One **new** accidental deviation (the VM-in-export problem), three carryovers from 2026-05-14.

## 5. Hardcoded fallback audit

- **B1 carryover — [engine.rs:1188](src-tauri/src/engine.rs:1188)**: `let energy_density = 0.5_f32;` still present in the album EXPORT path. The PCM is decoded one line above at [engine.rs:1152](src-tauri/src/engine.rs:1152); `compute_energy_density_score` exists at [engine.rs:670](src-tauri/src/engine.rs:670) and is already called from the analysis path. Wiring this in is the open-queue #1 fix. Reaffirmed.
- **B4 NEW — [types.rs:747](src-tauri/src/types.rs:747)**: `pub const ISO_PLACEHOLDER: &str = "2026-05-11T12:00:00Z";` is consumed as the timestamp string in **six** spots in production code paths:
  - [engine.rs:233](src-tauri/src/engine.rs:233) — `AnalysisResult.measured_at_iso`
  - [engine.rs:944](src-tauri/src/engine.rs:944) — `RenderJob.started_at_iso` (track render)
  - [engine.rs:1295](src-tauri/src/engine.rs:1295) — `RenderJob.rendered_at_iso`
  - [engine.rs:1687](src-tauri/src/engine.rs:1687) — `RenderJob.started_at_iso` (album render)
  - [settings.rs:29](src-tauri/src/settings.rs:29) — `UserPreset.created_at_iso`
  - [album.rs:545](src-tauri/src/album.rs:545) — album entry `measured_at_iso`

  Every "when did this happen" field in reports/manifests is hardcoded to the same wrong date. The name suggests it was a planning-phase scaffold; no comment marks it as temporary. Effect: an operator comparing two render attempts can't distinguish them by timestamp; cue sheets and manifests carry a stale absolute date. Easy fix at the call sites (use `chrono::Utc::now().to_rfc3339()` or equivalent), undetected because no test asserts that two analyses produced 1 second apart have different `measured_at_iso` values.
- **No other suspect literals found.** TODO/FIXME/HACK/XXX grep is clean in `src-tauri/src/`. The `0.5` and `0.0` literals in other locations are accumulator initializers, generator constants, or sample-spec defaults — not silent placeholders.
- The DitherRng seed `0xA11_CE` ([engine.rs:1477](src-tauri/src/engine.rs:1477)) and Album::Custom's `0.5` curve_value ([engine.rs:1184](src-tauri/src/engine.rs:1184)) are intentional defaults explained by nearby comments.

## 6. Architecture health (growth + cohesion)

Comparing against the 2026-05-14 baseline:

Rust (`src-tauri/src/`):

| File | Prior | Current | Δ | Growth verdict |
|---|---:|---:|---:|---|
| dsp.rs | 3,293 | 3,426 | +133 (+4.0%) | **OK** — within 20% gate. Phase A4 added the preset-driven compressor wiring, OVERDRIVE constants, and the chain-push VM math, all on-topic. |
| audio.rs | 2,122 | 2,130 | +8 (+0.4%) | **OK** — limiter ISP-guard + compressor `exp` swap are tiny edits. |
| engine.rs | 2,033 | 2,033 | 0 | **OK** — no growth. |
| album.rs | 893 | 893 | 0 | **OK**. |
| types.rs | 799 | 799 | 0 | **OK**. |
| exports.rs / files.rs / lib.rs / project.rs / settings.rs / main.rs / jobs.rs | unchanged | | | **OK**. |

Frontend (`src/`):

| File | Prior | Current | Δ | Growth verdict |
|---|---:|---:|---:|---|
| App.tsx | 2,438 | 2,438 | 0 | **OK** — no growth. Codex's lane. |
| useTrackMaster.ts | 1,501 | 1,557 | +56 (+3.7%) | **OK** — within gate. VM hotfix-3 (ref + override + setter rewrite) is the entire delta. Cohesion still suspect (one mega-hook driving full app state). |
| RightRail.tsx | 626 | 626 | 0 | **OK** — borderline cohesion, no growth. |
| VisualEqPanel.tsx / Knob.tsx / SignalChain.tsx / AlbumPanel.tsx / PresetIcon.tsx / bindings.ts | unchanged | | | **OK**. |

No new growth flags. Cohesion suspects (`audio.rs`, `App.tsx`, `useTrackMaster.ts`) are all carryover and remain Codex's lane per the handoff.

## 7. Test grading

New test file this session:

- **`preset_distinctness.rs`** — **partial correctness**. Asserts directional EQ deltas (Clarity ↓ presence, ↑ air; Oomph ↑ sub, ↓ low-mid) and crest factor relationships (Tape compresses crest, Punch preserves crest vs Loud) on volume-matched pink-noise output. **The thresholds were softened from the analysis-doc reference numbers** (Clarity presence -1.0→-0.4, Clarity air +0.8→+0.4, Oomph low-mid -2.0→-1.0) because the chain has one Q=0.8 peak at 1500 Hz covering the entire 1.5-4 kHz band — a single narrow peak can't deliver multi-band reference deltas. The structural-limit note at lines 22-39 of the test file documents the gap honestly. The contract still gates direction + non-zero magnitude + perceptual distinguishability; if the gap turns out to matter in real listening, the structural fix (wider Q or second mid peak) becomes load-bearing.

Existing tests reviewed in last checkpoint remain unchanged. No tests have been added that grade behavior-only on a DSP path. The 6 pre-existing tests updated for the new compressor semantics (per the handoff) still gate against numeric specs.

**Gap from B4 (ISO_PLACEHOLDER) noted**: no test asserts `*_iso` fields differ between two invocations. Adding that single regression test would have caught B4 at any point in the build.

## 8. Frontend debt

Scoped to the playback-tick path. No new findings beyond carryover from 2026-05-14:

- PlaybackTick still flows through `useTrackMaster.ts` → re-renders all subscribers ~50 Hz when playing. Same as prior checkpoint.
- No `React.memo` anywhere in `src/` (grep verified zero matches). Same.
- No `as unknown as T` casts outside `preview-mock.ts`.
- The VM `useRef` introduced in hotfix-3 is a deliberate workaround for React's render-batch read-after-write race — well-commented at [useTrackMaster.ts:179-186](src/hooks/useTrackMaster.ts:179) and at [useTrackMaster.ts:1198-1210](src/hooks/useTrackMaster.ts:1198). Not a stale-closure risk; the ref is read at chain-build time, not deferred into callbacks.

**No new findings.** Codex's lane.

## 9. Real bugs

- **B1 (carryover from 2026-05-14)** — [engine.rs:1188](src-tauri/src/engine.rs:1188) album-EXPORT path discards per-track `energy_density` (literal `0.5_f32`). PCM is decoded one line above, and `compute_energy_density_score` exists at [engine.rs:670](src-tauri/src/engine.rs:670). Net effect: album-arc character-bias presence-band energy-gate is dead in the album EXPORT path while the analysis path uses the real value. Already in the open queue as item #1.
- **B2 (carryover from 2026-05-14)** — [engine.rs:1437-1438](src-tauri/src/engine.rs:1437) `INT16_SCALE = 32_767.0` / `INT24_SCALE = 8_388_607.0`. `clamp(-1.0, 1.0) * SCALE` makes the most-negative integer unreachable; output range is asymmetric by 1 LSB. Audibly inconsequential (<-90 dB FS), technically incorrect. Already in the open queue as item #7a.
- **B3 (NEW)** — Volume Match applies in the export path. [dsp.rs:1713](src-tauri/src/dsp.rs:1713) `process_frame_inplace` multiplies `volume_match_gain_lin` into every frame whenever it deviates from 1.0, regardless of whether the chain is driving playback or render. The render path at [engine.rs:895](src-tauri/src/engine.rs:895) (track) / [engine.rs:1198](src-tauri/src/engine.rs:1198) (album) constructs the chain directly from request `settings`. If the operator has VM on at render time, the exported WAV is attenuated by the chain-push estimate (per the new hotfix-2 math), then partially re-pulled by the per-preset LUFS landing block — but the landing block lives after the chain has already processed everything, so the net result is an export that's under-shooting `target_lufs` by 0-1 dB. **PRODUCT.md line 262 and Locked Decision #22 explicitly say "Export level is unchanged" by VM.** Handoff acknowledges this as "newly-surfaced this session, not yet queued formally." Fix shape: either force `settings.volume_match = false` in `engine.rs` at the render entry points (~5 lines) or add an `is_render` flag to `MasteringChain::new` that skips the VM scalar in `process_frame_inplace`. Add a regression test asserting the rendered WAV peak/LUFS doesn't shift when VM is toggled in the input settings.
- **B4 (NEW)** — `ISO_PLACEHOLDER = "2026-05-11T12:00:00Z"` is used as the timestamp for every report/manifest `*_iso` field (see Pass 4 for the 6 call sites). Every analysis, every render job, every saved preset reports the same date string. Easy fix at call sites; missed because no test asserts the field has different values on different invocations. Not silent-numeric — operator-visible in reports — but undetected because of the "looks plausible" failure mode (it's a real ISO 8601 string, just frozen).

## 10. Push-back list

- **PB-A** — **Strip VM in the render path explicitly, don't trust the per-track `settings.volume_match` field to be off.** Hotfix-3 routes the UI checkbox through a wire-time override to the chain on the playback side, but per-track `settings.volume_match` can persist any value from previous app state — including `true`. If the operator toggles VM on, listens, switches tracks (override fires, ref updates), then exports, the request payload's `settings.volume_match` may still be the value `setVolumeMatch` last set in `useTrackMaster` (line 1215 writes it into the track's settings). The render goes through that — not through the transport override. The fix-the-root-cause version: render path should never trust `settings.volume_match` regardless of how the frontend wired the override. Force-set it to false in `engine.rs` render entry points. This is the same conclusion as B3 above; calling it out here as a push-back item too because the *broader* pattern is "VM is presentation state, not export state — the chain shouldn't see it at render time."
- **PB-B (carryover)** — **`transient_punch` is still captured but never applied**, same as last checkpoint. With Phase A4's compressor wiring, the Punch vs Loud crest contract passes through compressor attack/release tuning alone. A transient shaper at chain stage would let the contract test land at the analysis-doc thresholds (0.4 → reference value) without softening, and would more honestly deliver the "Punch" preset's perceptual identity. Worth promoting from push-back to a scheduled phase if the listening verification suggests Punch doesn't feel transient-forward enough.
- **PB-C (NEW)** — **The chain's structural limit (single Q=0.8 peak at 1500 Hz) forces the distinctness contract to soft-gate.** The Phase A4 thresholds (Clarity presence -0.4 not -1.0, Oomph low-mid -1.0 not -2.0, Clarity air +0.4 not +0.8) are honest-to-the-chain but smaller than the analysis-doc deltas. If Dan's listening pass on `It's a coat` says "Clarity isn't different enough from Universal," the structural fix is to add either (a) a wider mid Q (0.8 → ~0.5) or (b) a second peak around 2.5 kHz. Either is ~15 lines of `BiquadCoeffs::peaking` + a state field + a test update. Worth a decision before the next phase starts: live with the soft thresholds and gate via Dan's ears, or invest the day to lift the thresholds and let the contract test do more of the work. The handoff calls this out in the "Newly-surfaced this session" section.

## 11. Top priorities for next session

1. **Dan's listening verification — already the handoff's primary workstream.** Two ears-on checks: (a) confirm hotfix-3 VM stays sync'd through track-switch flurries; (b) run the analysis-doc listening pass on `It's a coat` per lines 191-198 of `PRESET_REFERENCE_ANALYSIS_2026-05-14.md`. The retune ship is at risk until Dan signs off; the test contract is partial-correctness because the chain's structural limit forced the thresholds to soften. Listening is the gate.
2. **B3: Strip VM in the export path.** Spec violation per PRODUCT.md Locked Decision #22 / line 262. Smallest defensible fix: clear `settings.volume_match = false` in `engine.rs` at both `render_track_master` and `render_album_master` entry points; add a regression test rendering the same source with and without `volume_match` flipped in settings and asserting the WAV peak/LUFS are byte-equivalent. ~15 lines + ~30 lines test.
3. **B4: Fix `ISO_PLACEHOLDER` for report timestamps.** Replace the constant uses with `chrono::Utc::now().to_rfc3339()` at the six call sites in Pass 4. Add a chrono dependency if not already transitive. Add one regression test asserting `measured_at_iso` differs across two analyses produced 1 second apart. ~40 lines total.
4. **B1: Album-export `energy_density` literal.** Open-queue #1; already scoped to ~10 lines fix + ~50 lines test in the handoff.

(B2 — INT scales asymmetric — remains low-priority background work as queued.)

---

End of checkpoint. Reviewer recommendation: do the listening verification (#1) first — that gates everything downstream. If hotfix-3 holds and the presets deliver in real listening, B3 and B4 are 30-60 minutes each and ship cleanly in one session. No emergency fixes required.

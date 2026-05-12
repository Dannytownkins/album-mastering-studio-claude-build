# Handoff — 2026-05-12 (Evening, End of Phase 12.2 Wired-Controls Session)

This is the session-end snapshot for the evening of 2026-05-12. It **supersedes** the earlier same-day handoff (`docs/HANDOFF_2026-05-12.md`) but does not erase it — that file is still useful for back-context on the Phase 12.1 listening-response work that preceded this session.

For the rolling-update entry point see `docs/HANDOFF.md`. For canonical product direction see `docs/PRODUCT.md` (canon — do not modify without Dan's explicit ask).

> **If you are a future Claude session reading this for the first time:** READ EVERY SECTION. Don't skim. The most expensive failure mode is redoing work that already shipped. If anything in this doc is ambiguous or contradicts what you see in the code/git log, **ask Dan before proceeding** — don't guess.

## TL;DR

Four objective Phase 12.2 (wired-controls) slices shipped on `origin/master` this session (commits `977a2d0` → `31b3c38`); one more Phase 12.2 slice (warmth + presence_air) is fully designed, planned, and committed but not executed — waiting for a fresh session to run. After that, the only remaining Phase 12.2 item is `compression_density`, which is large enough to need its own brainstorm + spec + plan cycle.

The session converted three "(coming soon)" labels into real controls (Width, LUFS target — and the warmth/presence_air plan will add two more) and added the live clipping indicator + album-render progress bar. **No subjective sound-quality decisions were made.** Every numeric value pinned in this session came from cited industry research, the HANDOFF design notes, or empirical synthetic-signal testing.

See the next section ("Phase context") for how Phase 12.2 fits into the formal `docs/IMPLEMENTATION_PLAN.md` phase map.

Test count went from 39 → 56 (current) → 61 (after the queued plan executes). Real-fixture tests still pass unchanged. The bundle stayed essentially flat (~253.6 KB / ~77.6 KB gzipped) across all four shipped slices.

## Phase context (how this session fits into `docs/IMPLEMENTATION_PLAN.md`)

The formal phase map lives in `docs/IMPLEMENTATION_PLAN.md`. The implementation plan defines 15 phases (0 through 14). Each gets a `PHASE N CONFIRMED — proceed to N+1` sentinel line in `docs/progress.md` that **only Dan writes by hand** — agents don't advance phases.

**Where we are: Phase 12 — Private Real-Audio Fixture Loop.** Per `IMPLEMENTATION_PLAN.md`, this is the workstream that drives the app from "structurally complete" to "release-candidate quality" by listening to real material and wiring whatever the listening exposes as broken or missing.

Within Phase 12 there are informal sub-iterations the commits and earlier handoffs track:

- **Phase 12.1 — Listening response (earlier today, morning session).** Captured in `docs/HANDOFF_2026-05-12.md`. Dan's first listening pass on real audio (`It's a coat (Remastered).wav`) flagged: real-time audition wasn't actually live, presets clipping on already-mastered material with no warning, advanced sliders unwired, no Input/Output gain, etc. Phase 12.1 shipped 19 commits (`7b35e34` → `ed777e1`) addressing the immediately-fixable items: live-update bug fix, decode cache, undo/redo, Input/Output gain, "(coming soon)" labels added to placeholder sliders, render progress bar, type-in number fields.
- **Phase 12.2 — Wired-controls campaign (this evening session).** Phase 12.1 ended with a P0 backlog of unwired Advanced controls and one missing safety affordance (the live clipping indicator). Phase 12.2 is the explicit campaign to clear that backlog. Status:
  - ✓ Live clipping / output peak indicator (`977a2d0`)
  - ✓ Wire Width (`fc2674b`)
  - ✓ Album render progress events (`058794e`)
  - ✓ Wire LUFS target — refuse-upward landing (`31b3c38`)
  - ⧗ Wire Warmth + Presence/Air — **spec + plan ready, not yet executed** (`8c4c412`, `1349186`)
  - ☐ Wire `compression_density` — **needs fresh brainstorm + spec + plan** (large slice, ~300–500 lines)

**Phase 12.2 finishes when the warmth+presence_air plan ships AND compression_density ships.** That leaves Track Master with every Advanced control wired (no "(coming soon)" labels remaining) and the live clipping safety net in place.

**After Phase 12.2:** the next iteration depends on what Dan's next listening pass exposes. Possible paths:
- More Phase 12 sub-iterations if listening reveals more gaps.
- Preset rebalancing (subjective; needs Dan's ear).
- `PHASE 12 CONFIRMED — proceed to 13` if Dan is satisfied — moving to **Phase 13 (Performance Budgets)** per `IMPLEMENTATION_PLAN.md`.

**Agents do not cross phase boundaries autonomously.** Stop and ask Dan when:
- The current Phase 12 backlog is empty and there's no `PHASE 12 CONFIRMED` line.
- A proposed slice would touch Phase 13+ scope (performance work, installer hardening, etc.) before Phase 12 is sealed.

## Long-term goal (from `docs/PRODUCT.md`)

**Mission:** "Album Mastering Studio is a local desktop mastering app for real tracks and real albums. It should be something a musician or producer would be proud to run their audio through."

**Quality bar:** Release-candidate for personal albums and capable musicians/producers who want a trustworthy local workflow. Accessible enough that non-musicians can use the safe default path without being walled off by jargon.

**Two modes, in order:**

1. **Track Master first** — independent songs, fast-path workflow (Drop → Analyze → Universal → Preview/Audition → Export). Currently structurally complete per `docs/IMPLEMENTATION_PLAN.md`'s gate list; all 20 non-negotiables shipped (drag/drop, analyze, universal, large waveform, zoom, region select, loop, A/B at-same-playhead, optional Volume Match, preset tiles, intensity macro, 3-band EQ, whole-track preview, stale-preview flag, real-time audition, export button, advisory checks, non-overwriting output, autosave, undo/redo).
2. **Album Master next** — sequenced songs becoming a coherent record (Drop → Reorder → Analyze → Story/Roles → Album intent → Per-track overrides → Continuous album WAV + individuals). Structurally present but needs hands-on album-workflow validation. Refinements named in `docs/HANDOFF_2026-05-12.md` under "P2 — Album Master."

**The product gate that matters:** PRODUCT.md says "The user should be able to listen deeply before trusting an export" and "Reports support confidence, but listening remains central." Every slice should preserve or improve listening, not just metering numbers.

**What we don't do:** Claim certification, replace expert judgment, edit source files destructively, overwrite previous exports by default, force users to learn jargon, push the chain past the user's true-peak ceiling silently, copy from the parallel Codex repo.

## What shipped this session (so it isn't redone)

All four slices on `origin/master`, monotonic test growth, no regressions:

| Commit | Slice | Test count after | Bundle (KB / gz) |
|---|---|---|---|
| `977a2d0` | Phase 12.2 — Live clipping / output peak indicator | 44/44 | 253.68 / 77.57 |
| `fc2674b` | Phase 12.2 — Wire Width (Advanced) via M/S processing | 53/53 | 253.66 / 77.57 |
| `058794e` | Phase 12.2 — Album render progress events | 54/54 | 253.66 / 77.57 |
| `31b3c38` | Phase 12.2 — Wire LUFS target (refuse-upward landing) | 56/56 | 253.65 / 77.57 |

Plus two non-code commits:

| Commit | What |
|---|---|
| `8c4c412` | Spec: `docs/superpowers/specs/2026-05-12-warmth-presence-air-design.md` |
| `1349186` | Plan: `docs/superpowers/plans/2026-05-12-warmth-presence-air.md` |

The progress.md tail has prose entries for each shipped slice with verification numbers, what's-partial notes, and next-slice recommendations.

### Concrete features now live in Track Master

- **Live clipping / peak indicator** — `StaleBar`'s new chip flashes red ("CLIP") when post-output-gain peak ≥ -0.1 dBFS during mastered playback, shows live dB readout in green otherwise. Atomic-folded `f32` bit-pattern (`Arc<AtomicU32>::fetch_max`) shared between `MasteringSource` and the audio thread; swap-and-reset on every 50 ms snapshot tick.
- **Width control wired** — `Advanced.width` drives an M/S transform between EQ and saturation. Pure stereo math, side scaled by `[0, 2]` (UI slider exposes `[0, 1.5]`). Skip-guard preserves byte-equivalence on the default (1.0) path.
- **Album-export progress bar** — `render_album_master` now emits `RenderProgress` events with `kind: Album` and a fraction computed as `(track_index + within_track_fraction) / total_tracks`. The `StaleBar` message logic was refactored to read `renderProgress` directly instead of the per-mode flags so all export modes show their progress bar uniformly.
- **LUFS target wired (refuse-upward)** — `Advanced.lufs_offset_db` now drives a post-render integrated-LUFS landing pass: measure via `ebur128`, if `delta < 0` apply linear gain (safe — downward only), if `delta ≥ 0` leave samples unchanged (refuse-upward — research-confirmed industry policy). Helpers `engine::measure_integrated_lufs` and `engine::measure_integrated_lufs_at_path` are pub for test access.

### One slice fully designed but NOT yet executed

The next slice — wire `warmth` and `presence_air` Advanced controls — has a complete spec and plan committed but **no code has been written**. Pick up from `docs/superpowers/plans/2026-05-12-warmth-presence-air.md` and execute task-by-task. The plan is TDD-ordered with exact code, file paths, expected test output, and verification commands.

## Required reading (in order, before changing anything)

1. **`docs/PRODUCT.md`** — product canon. Locked decisions, mission, target user, quality bar, gates. Do not modify without Dan's explicit ask. If anything below conflicts with PRODUCT.md, PRODUCT.md wins.
2. **`CLAUDE.md`** (repo root) — non-negotiables, working style, source-import rules (no copying from the parallel Codex repo).
3. **This file** — current snapshot. Supersedes the earlier `HANDOFF_2026-05-12.md` for forward direction; that older file is still authoritative for back-context.
4. **`docs/superpowers/plans/2026-05-12-warmth-presence-air.md`** — the next slice to execute. Self-contained. Backed by:
5. **`docs/superpowers/specs/2026-05-12-warmth-presence-air-design.md`** — the approved design for the plan. Cites the research doc inline.
6. **`docs/progress.md`** — time-series log. Tail it (read with offset near the bottom) for the latest entry's "Next recommended slice" if the plan above has been completed.
7. **`docs/IMPLEMENTATION_PLAN.md`** — phase map and gate definitions.
8. **`docs/CLAUDE_WORK_LOOP.md`** — work loop format (read once if you haven't seen it).
9. **`docs/HANDOFF_2026-05-12.md`** — the prior same-day handoff. Read for back-context on Phase 12.1 / earlier session decisions. Still has the most comprehensive "Where to look for things" file map.
10. **`docs/PRIVATE_AUDIO_FIXTURES.md`** — before touching `private-audio-fixtures/`.

Do NOT read by default (only on explicit ask or specific historical detail need):

- `docs/reference/` — Codex implementation context. Don't copy from there.
- `docs/research/most-recent-mastering-app-research.md` — Dan-provided research file (960 lines). Used by this session to ground the LUFS-landing and warmth/air designs. Untracked at the moment (Dan placed it locally; not yet `git add`ed). If you need to consult it, prefer dispatching an Explore subagent against it rather than reading the whole thing into your main context.

## The work loop

Per `docs/CLAUDE_WORK_LOOP.md` and the earlier handoff:

1. Read the goal + check the progress.md tail for the latest entry's "Next recommended slice."
2. Identify the PRODUCT.md requirement it serves.
3. Inspect relevant code before assuming a decision is implemented. The "Where to look for things" section in `docs/HANDOFF_2026-05-12.md` has the file map.
4. Build one vertical slice (not a refactor; not multiple disconnected wins).
5. Add or update tests where behavior is testable.
6. Run verification:
   - `npm run build` (frontend, fast).
   - `cd src-tauri && cargo check --tests` (Rust type-check, fast, doesn't relink the main binary).
   - `cd src-tauri && cargo test` (full suite, slower; **may be blocked if `npm run tauri dev` is running** — see pitfalls).
7. If green: commit + push to `origin/master`, then append a progress.md entry using the shape in HANDOFF.md.
8. If red: leave uncommitted, append a progress.md entry describing the failure and what to try next, stop.

### Commit convention

Subject under 70 chars, descriptive body, `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>` line at the bottom. See `git log` for shipped examples in this session.

Push to `origin/master` after every passing slice. No feature branches — single-author personal project.

### Boundaries — stop and ask Dan before

These come from the active `/goal` directive and the earlier handoff. They still apply.

- Claiming Track Master release-candidate quality.
- Modifying `docs/PRODUCT.md`.
- Crossing a phase gate that requires human approval (no `PHASE N CONFIRMED — proceed to N+1` line in `progress.md`).
- Making subjective sound-quality decisions without real listening notes.
- Touching `private-audio-fixtures/`.
- Reading or copying Codex source code from the parallel repo.
- Force-pushing or rewriting history.
- Adding paid services, signing anything, making the project public.

### When in doubt — ask

If the next slice's design isn't already pinned by an approved spec or by HANDOFF design notes, **stop and ask Dan** rather than guess at:

- Frequency centers, dB ranges, Q values, or other DSP parameters that affect audible character.
- Preset numeric tuning (the listening-driven P2 work).
- UX copy that affects user trust ("Volume Match" tooltip, advisory wording, etc.).
- Whether to surface a piece of metering or DSP info in the UI vs the export receipt vs a report.
- Schema changes to `MasteringSettings` / `AdvancedSettings` / `ProjectState` (always add `#[serde(default)]` for new fields; ask before removing or renaming any).

## Next slice (immediate) — execute the queued plan

**Path:** `docs/superpowers/plans/2026-05-12-warmth-presence-air.md` (committed `1349186`).

**Spec:** `docs/superpowers/specs/2026-05-12-warmth-presence-air-design.md` (committed `8c4c412`).

**Why:** Two `"(coming soon)"` labels — Warmth and Presence/Air — become real controls. Design grounded in `docs/research/most-recent-mastering-app-research.md` (Sonible smart:limit, LANDR, BandLab, Ozone consensus = pure-EQ shelves, one-sided, additive). 5 new unit tests; no contract tests added (the existing presets all leave warmth/presence_air at None, so no contract test fixture changes).

**Tasks (summary — read the plan for full step-by-step):**

1. Wire warmth biquad: add fields to `ChainCoeffs` and `ChannelState`, compute in `from_settings` (low-shelf @ 300 Hz, slope 0.7, slider 0..1 → 0..+4 dB), apply in `process_frame_inplace` and `process_sample`. 3 tests up-front (default identity, at-one lifts 100 Hz by >+3 dB while leaving 5 kHz untouched, clamp).
2. Add presence_air tests (implementation lands in Task 1 since both fields had to be added together to compile). 2 tests (default identity, at-one lifts 18 kHz by >+3 dB while leaving 1 kHz untouched).
3. Drop "(coming soon)" labels in `src/App.tsx` `AdvancedPanel` for both controls.
4. Full verification triad + append progress.md entry + commit + push. Expected `cargo test` end state: **61/61 pass**.

**Sub-skill to use:** `superpowers:executing-plans` (inline) or `superpowers:subagent-driven-development` (dispatch per task — recommended for a clean context).

## Remaining open work — after the queued plan completes

Roughly ordered by readiness. Items already in `docs/HANDOFF_2026-05-12.md` are not duplicated here unless their status changed; consult that file for full design notes.

**Phase mapping for everything below:**
- **Still inside Phase 12.2 (this session's campaign):** `compression_density`. The queued warmth+presence_air plan plus this item finish Phase 12.2.
- **Still inside Phase 12 broadly:** preset rebalancing (listening-driven response), the rendered-LUFS export-receipt gap (sub-iteration of the listening loop).
- **Future-phase items, gated by `PHASE 12 CONFIRMED`:** Phase 8.x Album Master refinements, Phase 9.2 editable role UI, Phase 11.2.d polyphase FIR, Phase 13 performance budgets, Phase 14 release / installer hardening.
- **Ongoing / cross-phase polish:** typography pass, SVG preset icons, deferred infrastructure (vitest, multi-slot decode cache).

Do not jump into a future-phase item without checking `progress.md` for a `PHASE N CONFIRMED — proceed to N+1` line. If it's missing, ask Dan.

### P0 — objective, no listening required (one item left, finishes Phase 12.2)

#### `compression_density` (real envelope-following compressor)

- **Status:** Last remaining unwired Advanced control after warmth/presence_air ships. Shipping this closes Phase 12.2.
- **Scope per HANDOFF_2026-05-12 estimate:** ~300–500 lines. Real attack/release envelope follower + gain-reduction curve, applied before the brick-wall limiter.
- **Recommendation:** **Brainstorm + spec + plan first** using `superpowers:brainstorming` and `superpowers:writing-plans` (same flow this session used for warmth/presence_air). Don't dive into code without an approved design.
- **Research to consult:** `docs/research/most-recent-mastering-app-research.md` covers commercial compressor behavior; an Explore subagent extract on "compression / glue / multiband dynamics" would mirror the LUFS-landing and warmth-air research passes this session ran.

### P1 — UX polish Dan asked for

#### Typography pass (HANDOFF P1 #6)

- Dan: "UI overall could use larger text overall."
- Pure CSS slice; subjective enough to want Dan's eye on the result before committing.

#### SVG preset icons (HANDOFF P1 #7)

- Dan's reference screenshot from the parallel Codex build had distinct icons per preset. Adds visual hierarchy. Open-source icon set (Lucide etc.) preferred — do NOT copy from the Codex build.

### P2 — Album Master + later

#### Phase 9.2 editable role / character UI (HANDOFF P2 #9)

- Album Master refinement: let users override the inferred role/character from analysis.

#### Phase 8.x Album Master refinements (HANDOFF P2 #10)

- Cross-track loudness uniformity, per-track-role audio adaptation (currently inferred but not applied to the chain), boundary/gap/crossfade primitives (Phase 10).
- **Album-mode LUFS landing.** Per-track decision is the same as Track Master (refuse-upward). The album-wide question (apply target to the continuous album WAV vs per-track) is a product decision and belongs here.

#### Phase 11.2.d polyphase FIR true-peak (HANDOFF P2 #11)

- Lagrange-4 at 3 points (currently shipped) is good but ITU-R BS.1770 specifies a polyphase FIR. Low priority.

### P2 — subjective, needs Dan's ear

#### Preset rebalancing

- Dan: "some of the presets are pretty intense and not all that differentiated from each other." Wait for Dan to flag specific presets ("Tape is too dark"); adjust values in `dsp.rs::ChainCoeffs::from_settings` (the per-preset `match` block around line 175). Each adjustment is a focused commit so Dan can A/B vs the prior tuning.

### Deferred infrastructure

#### Frontend `vitest` infrastructure (HANDOFF infra #13)

- Currently no automated tests for frontend behavior. Undo/redo, slider events, IPC wiring all rely on Dan's manual smoke.

#### Multi-slot decode cache (HANDOFF infra #14)

- Single-slot LRU is fine for Track Master single-file workflow. Album sessions will thrash.

#### Export receipt: rendered LUFS vs source LUFS

- **New gap noted in the LUFS-landing slice (`31b3c38`).** `useTrackMaster.ts::exportMaster` sets `ExportReport.measured_lufs` from `selectedAnalysis.lufs_integrated` — that's the **source** LUFS, not the rendered output's LUFS. This is a pre-existing limitation; fixing it would let `run_export_checks` correctly compare against the user's target. Required before adding a `lufs_target_unmet` advisory.

## Where to look for things

Use the comprehensive file map in `docs/HANDOFF_2026-05-12.md` ("Where to look for things" section). It's still accurate; this session only modified existing files inside that map and added two new ones in `docs/superpowers/`:

- `docs/superpowers/specs/` — approved design specs (one entry so far, the warmth/presence_air design).
- `docs/superpowers/plans/` — implementation plans (one entry so far, the warmth/presence_air plan).

These two directories were created this session. Future spec/plan cycles should put their outputs here too.

### What changed in code this session (so you don't relearn)

- `src-tauri/src/audio.rs` — Phase 12.2 peak fold added to `MasteringSource`; `AudioThreadState` owns the shared peak atomic; `audio_thread`'s snapshot construction swap-and-converts; new `linear_to_dbfs` helper. 4 new audio module tests (clipping reflection, clean signal, swap-reset, conversion sanity).
- `src-tauri/src/dsp.rs` — Phase 12.2 width transform: `ChainCoeffs::width_side_scale`, `apply_width_stereo` helper, `process_frame_inplace` refactored into EQ-pass + width + saturation-pass. 9 new dsp module tests (apply_width_*, ChainCoeffs default/clamp, end-to-end through chain).
- `src-tauri/src/engine.rs` — Phase 12.2 LUFS landing: new `measure_integrated_lufs` / `measure_integrated_lufs_at_path` helpers; `mastering_render_with_progress` applies refuse-upward gain when `lufs_offset_db` is set. Phase 12.2 album progress: `album_render_with_progress` with per-chunk callback emission; `album_render` becomes a thin wrapper. 2 new contract tests for LUFS landing + 1 for album progress.
- `src-tauri/src/lib.rs` — Phase 12.2 peak: `peak_dbfs` forwarded into the `PlaybackTick` emit.
- `src-tauri/src/types.rs` — Phase 12.2: `PlaybackTick.peak_dbfs` field with `#[serde(default)]`.
- `src-tauri/tests/contracts.rs` — Phase 12.2: new `write_sine_wav_at_amplitude` helper (`write_sine_wav` becomes one-line wrapper); 1 album-progress test, 2 LUFS-landing tests.
- `src/App.tsx` — Phase 12.2: `ClippingIndicator` component in `StaleBar`; StaleBar text logic decoupled from `isRendering`; `AdvancedPanel` Width label "(coming soon)" dropped.
- `src/App.css` — Phase 12.2: `.clip-indicator` styles + `@keyframes clip-pulse` animation.
- `src/bindings.ts` — Phase 12.2: `PlaybackTick.peak_dbfs` added.
- `src/hooks/useTrackMaster.ts` — Phase 12.2: `transport.peakDbfs` field, updated from each `onPlaybackTick`.

### Documents this session added

- `docs/superpowers/specs/2026-05-12-warmth-presence-air-design.md` — approved spec for the queued slice.
- `docs/superpowers/plans/2026-05-12-warmth-presence-air.md` — TDD-ordered implementation plan.
- `docs/HANDOFF_2026-05-12_evening.md` — this file.

### Documents this session referenced but did not modify

- `docs/PRODUCT.md` — canon, unchanged.
- `docs/HANDOFF_2026-05-12.md` — the morning's handoff. Comprehensive file map and pitfalls section still valid. Open-work prioritization is now superseded by this evening file but the design notes for each item are still useful.
- `docs/IMPLEMENTATION_PLAN.md` — phase map. Unchanged.
- `docs/CLAUDE_WORK_LOOP.md` — work loop format. Unchanged.
- `docs/CLAUDE_BUILD_BRIEF.md` — build context. Unchanged.
- `docs/PARALLEL_BUILD_NOTES.md` — independence from Codex repo. Unchanged.
- `docs/PRIVATE_AUDIO_FIXTURES.md` — fixture conventions. Unchanged.
- `docs/SCHEDULE_PROMPT.md` — `/schedule` routine prompt. Unchanged.
- `docs/adr/0001-tauri-rust-stack.md` — Tauri + Rust ADR. Unchanged.
- `docs/research/most-recent-mastering-app-research.md` — Dan-provided industry research. **Not yet committed** — currently untracked in the repo. If a future session needs it tracked, Dan can `git add` and commit it; until then, it lives locally only.

## Verification state

- `cargo test --lib`: **19/19 pass** (will become 24/24 after the queued plan executes).
- `cargo test` (full): **56/56 pass** (will become 61/61). Real-fixture tests pass; their numbers are pinned in the `phase_12_1_real_fixture_metering_snapshot` test's stdout (run with `--nocapture` to see).
- `cargo check --tests`: clean.
- `npm run build`: clean. **253.65 KB / 77.57 KB gzipped** (slight variance per commit; never exceeded 253.7 KB).
- `npm run tauri dev`: not run by agents. Dan's manual smoke is the integration layer.

Real-audio metering snapshot from Phase 12.1 (still the freshest one):

| | Source | Master (default Universal @ 0.5) | Delta |
|---|---|---|---|
| LUFS integrated | -14.61 | -13.05 | +1.55 LU |
| True peak (BS.1770) | -3.97 dBTP | -2.42 dBTP | +1.55 dB |
| Dynamic range | 5.15 LU | 5.15 LU | +0.00 |
| Spectral balance | low 0.476 / mid 0.491 / high 0.034 | — | — |
| Inferred role | AlbumTrack (Unsure) | — | — |
| Inferred character | Dark (Moderate) | — | — |

## Pitfalls (carried forward from earlier handoff + new ones)

These cost Dan and Claude time during prior sessions. Re-reading them before each slice is cheap insurance.

### Carried from `docs/HANDOFF_2026-05-12.md`

1. **`cargo test` fails when `npm run tauri dev` is running.** Windows holds `target/debug/album-mastering-studio.exe` locked; cargo can't relink. Symptoms: `error: failed to remove file ... (os error 5)`. Workarounds: ask Dan to close the dev app; or `cargo check --tests` for type-only verification; or run the previous build's test binary directly.
2. **`eprintln!` from `audio.rs` doesn't reach Dan's `cmd` terminal.** Tauri-dev's subprocess spawn drops the binary's stderr. Use in-app affordances instead (the `liveUpdateStats` counter and the new `ClippingIndicator` are working models).
3. **React 18 batched-updates trap.** Don't rely on side-effect assignments inside `setState((prev) => { x = mutate(prev); return ... })` for synchronous reads after the setState call. Compute the value from closure-captured state first, then setState.
4. **Add `#[serde(default)]` to every new field on `MasteringSettings` and similar persisted structs.** Old sessions must still deserialize. See `input_gain_db`, `output_gain_db`, `PlaybackTick.peak_dbfs` for the pattern.
5. **Cargo.toml line-ending churn (LF ↔ CRLF) on Windows.** Don't commit it. `git checkout -- src-tauri/Cargo.toml` before committing if `git status` shows it as modified after a cargo run.
6. **DevTools is not reliable.** Dan can't open F12 while actually working. Don't propose "open DevTools and report what you see" — build in-app affordances instead.
7. **`AdvancedPanel` had placeholder fields.** Five of eight Advanced controls were unwired at the start of the day. After this session and the queued plan: Width ✓, LUFS target ✓, Warmth (plan ready), Presence/Air (plan ready). Only `compression_density` remains.
8. **Decode cache is a single-slot LRU.** Fine for Track Master single-file workflow; album sessions will thrash. Multi-slot is deferred infrastructure.
9. **`mastering_render` uses 4096-frame chunks** for progress reporting. The chain's per-frame state flows correctly between chunks because we call into the same `chain` instance. Don't refactor this loop to use `Vec::chunks_mut().for_each()` without preserving state continuity. This now also applies to `album_render_with_progress` (`058794e`).

### New in this session

10. **Atomic-f32 via `AtomicU32::fetch_max(bits)` only works for non-negative finite values.** IEEE 754 bit ordering matches numeric ordering only on `[0, +inf)`. We guarantee this by storing `|x|` and filtering NaN/inf at the writer. Don't extend this pattern to signed peak values without revisiting the math. (See `audio.rs::MasteringSource::next`, `peak_linear.fetch_max`.)
11. **JSON can't carry `-inf` cleanly.** Use a sentinel like `SILENCE_DBFS = -120.0` for "no signal in the window" instead of `f32::NEG_INFINITY`. Same applies to any future dB-domain wire fields.
12. **Test assumptions about chain output need empirical verification.** The first iteration of `lufs_target_refuses_to_amplify_quiet_render` assumed a 0.5-amplitude sine through Universal/intensity 0.5 would render quieter than -6 LUFS. It actually rendered at -4.5 LUFS, LOUDER than -6, so the refuse-upward branch wasn't exercised. The fix: use a much quieter source (0.02 amplitude) and Custom/intensity 0 to neutralize the chain. **Mental-math estimates of LUFS through this chain are easily off by 5+ dB.** Always check with the actual measurement helper if a test fails.
13. **`StaleBar` message logic was decoupled from `isRendering` in `058794e`.** It now reads `progressPct` directly so all export modes (track preview, track master, album) show the right progress message. If you add a new render mode, just emit `RenderProgress` with the right `kind`; the bar will render correctly without touching `StaleBar` again.
14. **Album-render bytewise equivalence after chunking.** When `058794e` chunked `album_render` into 4096-frame slices for sub-track progress, the existing `album_render_writes_continuous_and_individual_masters` test still passed — because `chain.process_interleaved` does its own per-frame iteration internally, so chunking the call site doesn't change the chain's per-sample math. If you ever chunk another loop, verify the existing real-fixture tests still pass before assuming equivalence.

## Operating philosophy (carried from the morning handoff, still applies)

What worked across this session:

- **Small commits per slice, pushed immediately.** Easier to revert, easier to review, smaller blast radius.
- **Tests-first when the bug is unclear.** The `mastering_source_applies_live_coeff_updates_via_channel` test (earlier session) and the empirical-correction loop on the LUFS refuse-upward test (this session) both narrowed bugs in seconds.
- **In-app diagnostic affordances.** The `liveUpdateStats` counter and the new `ClippingIndicator` are both working patterns for verification without DevTools.
- **Honesty in UI labels.** "(coming soon)" is better than a silent no-op. Removing one of those labels per slice is concrete progress toward release-candidate.
- **Research-first for subjective designs.** The LUFS-landing and warmth/air designs both went through an Explore-subagent research extraction against Dan's research doc before any code was written. Saved a wrong-policy implementation in both cases.

What didn't work in prior sessions (still relevant):

- `eprintln!` for diagnostics Dan would need to see.
- Assuming React 18 setState semantics matched React 17.
- Asking Dan to open DevTools while working.

## A final note for the next session

Read this file end-to-end before touching anything. **Do not** start by jumping to `git log` and pattern-matching the commits — the commit messages don't carry the boundary information about "what's done, what's queued, what needs Dan's ear vs Dan's eye." This document does.

If a section here contradicts what you see in the code, or if the plan path doesn't exist, or if `progress.md`'s tail describes a state that doesn't match `git log`, **ask Dan before proceeding**. Don't guess. The cheapest failure mode is being wrong slowly.

If you're continuing the wired-controls campaign past the queued plan: the remaining `compression_density` slice is genuinely large and benefits from a fresh brainstorm + spec + plan cycle. Don't try to code it directly from the HANDOFF design fragment.

If listening notes come in from Dan that override the queue (preset rebalancing, "Tape feels too dark," etc.) — those take precedence per the goal directive's "subjective sound-quality decisions" clause.

---

*Last updated: 2026-05-12 evening, end of Phase 12.2 wired-controls session. Six commits shipped (`977a2d0` → `1349186`, four code slices + spec + plan). Next agent: execute the queued plan at `docs/superpowers/plans/2026-05-12-warmth-presence-air.md`.*

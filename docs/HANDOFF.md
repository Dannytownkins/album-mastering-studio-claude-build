# Handoff — YES Master (Claude Build)

This document is the entry point for any Claude session — interactive or scheduled — picking up work on this repo. Read this first, then start the loop below.

> **Current snapshot (2026-05-15 evening).** 22 commits today after the Phase A4 morning handoff, every one mechanically gated. Full B1–B7 audit queue resolved, four-layer perf defense for live preview (8 s window + VM cap + coalescer with playback barriers + landing-gain cache), three-tier PCM resolution (local cache → off-thread prewarm cache with target guard → fresh decode) eliminates the 1-2 s decode stall, shared ceiling-bounded landing helper replaces three drift-prone copies, Vitest scaffolded with four `src/lib/` pure-helper modules covering effective-settings / settings-transitions / history-stack. Most recent commit (`74d704f`) addresses four Codex review items: auto-prewarm on import/restore/openProject paths, stale-prewarm-evicts-newer target guard, source_lufs_integrated doc/test-name drift, and LoudnessTarget display extracted to a pure helper. Full snapshot detail in `docs/HANDOFF_2026-05-15_evening.md` (read this — it's the entry point for the next session).
>
> **Addendum (2026-05-17 + 2026-05-18).** Codex landed `f85482c` (2026-05-17) closing the deferred Codex item 4: mock-API React/Vitest coverage for auto-prewarm dispatches (restore/import/openProject), Export-LUFS toggle dispatch, and LoudnessTarget force-to-Custom DOM (+6 Vitest tests via a one-line `export` on `LoudnessTarget`). Today (2026-05-18) Claude closed the `audio.rs` split candidate in three sequential mechanical-only commits: `fcd5ec3` extracts `SpectrumRing` + `SpectrumAnalyzer` → `src/spectrum.rs` (175 lines); `03d79e3` extracts `decode_full` + `decode_to_peaks` + Symphonia surface → `src/decode.rs` (220 lines, with engine.rs's 5 call sites updated); `abedc64` extracts `MeteredPcmSource` + `MasteringSource` (+ COEFFS constants, bumped to `pub(crate)`) → `src/sources.rs` (423 lines). Net: `audio.rs` 3,655 → 2,883 lines (-21%). Behavior byte-identical; test matrix unchanged. HEAD: `abedc64`. Full session detail in `docs/progress.md` under `2026-05-18 — audio.rs split refactor (3-pass)`.
>
> **Addendum (2026-05-18 evening).** Export destination UX now asks for explicit save locations for track and album exports, the duplicate Album Export button was removed, and deferred taste checks live in `docs/followups/listening-batch-2026-05-19.md`.
>
> **Test totals:** `cargo test --lib` **144/144**; `cargo test --target-dir target-tests` full fast lane pass; **Vitest 49/49** across 5 files (was 43 pre-Codex-item-4); `npm run build` clean.
>
> **What's open / next.** The autonomous queue is effectively empty of items that don't need Dan's input. Three plausible directions: (1) Dan's listening verification batch — five items queued in the checkpoint, would benefit from a focused listening hour; (2) async live-preview measurement on a worker thread — paused this session pending Dan's input because the cost-benefit shifted with the 4-layer perf defense in place; (3) a new product surface (Reference Track UX, Album Master gaps) — needs Dan's nomination.
>
> **Codex owns the UI lane** for the moment. Do not edit `src/App.tsx`, `src/App.css`, `src/components/RightRail.tsx`, or `src/components/AlbumPanel.tsx` from the Claude side unless a UI change strictly forces it AND you've pulled latest. App.tsx WAS touched this session for the B7 / LoudnessTarget fixes; coordinate before any further App.tsx work.
>
> **New pattern: `src/lib/` pure helpers + co-located Vitest.** Four modules so far:
> - `src/lib/effective-settings.ts` (+ test) — read-direction shadowing helpers + LoudnessTarget display.
> - `src/lib/settings-transitions.ts` (+ test) — write-direction transitions: B7 auto-flip, LoudnessTarget force-flip, VM session-level + source_lufs injection.
> - `src/lib/history-stack.ts` (+ test) — undo/redo stack arithmetic, generic over T.
>
> Future frontend slices: extract decision logic into `src/lib/*`, write Vitest cases next to it, glue from the hook.
>
> Major capabilities already live: realtime BS.1770-4 momentary + integrated LUFS metering on both Original and Mastered playback; 4-band EQ chain (200 / 400 / 1500 / 6000 Hz) with Phase A4 conservative-target preset calibration; per-preset multiband compressor with user `compression_density` macro scaling preset baseline; 8 delivery profile shadows; TPDF dither at integer output (now symmetric-range, post-B2); 6-band FFT spectral analysis + transient flux + stereo correlation + dynamic-range P95-P10 + 3 s short-term LUFS max + energy-density composite; Album Master (4 named arcs + Custom, position-aware character inference, per-character bias); session-level Volume Match A/B with ceiling-bounded chain-push estimate; live FFT spectrum under the EQ panel; ceiling-bounded LUFS landing across all three render paths via shared helper; live-preview-matches-export with a 4-layer perf defense (window + cap + coalescer + cache); decode-stall-eliminating prewarm cache populated off the audio thread; Vitest test harness with mechanical gates for every trust-pattern fix this session. Older session snapshots: `docs/HANDOFF_2026-05-15_session.md` (Phase A4 + VM hotfixes, morning), `docs/HANDOFF_2026-05-14_session.md`, etc.

## Read first (in order)

1. `CLAUDE.md` — repo rules, non-negotiables, working style, fast/slow test lanes.
2. `docs/PRODUCT.md` — product canon and source of truth (now titled **YES Master Product Canon**). Do not modify without Dan's explicit ask.
3. **`docs/HANDOFF_2026-05-15_evening.md`** — the latest dated handoff. Carries the full 22-commit session inventory, the four-layer perf defense architecture, the decode-stall fix's three-tier cache, the trust-pattern fix consolidation, the `src/lib/` pure-helper pattern, the four Codex review items addressed in `74d704f`, and the deferred follow-up list.
4. `docs/HANDOFF_2026-05-15_session.md` — the morning handoff (Phase A4 ship + 3 VM hotfixes). Useful for back-context on what was in flight at the start of today.
5. `docs/checkpoints/checkpoint-2026-05-15-end-of-mechanical-gates-session.md` — session inventory written before the Codex review items landed. Captures architectural state, file-size growth, trust-pattern fix consolidation.
6. `docs/checkpoints/checkpoint-2026-05-15-post-phase-a4-vm-hotfixes.md` — morning audit checkpoint that opened the B1–B7 queue.
7. `docs/PRESET_REFERENCE_ANALYSIS_2026-05-14.md` — the calibration analysis that drove Phase A4. Conservative Target Table (lines 252–259) is what landed.
8. `docs/checkpoints/checkpoint-2026-05-14-pre-preset-retune.md` — the review checkpoint that anchored Phase A4.
7. `docs/IMPLEMENTATION_PLAN.md` — phase map and gates (back-context; mostly closed).
8. `docs/progress.md` — append-only slice log; tail entry is "where we are now."
9. `docs/CLAUDE_WORK_LOOP.md` — work loop format.

Do not re-elicit design that already exists in those docs. The spec is settled. Find the next unfinished slice and work it.

## What "next slice" means

The current state lives in `docs/progress.md`. The last entry's "Next recommended slice" tells you where to start. If it's stale (e.g. the slice has been worked but progress.md hasn't been updated), inspect the repo and `git log` to confirm before starting.

If there's no clear "next slice", read the active phase entry in `docs/IMPLEMENTATION_PLAN.md` and pick the smallest unfinished requirement.

## The loop

1. Read the slice goal and what product requirement from `docs/PRODUCT.md` it serves.
2. Inspect relevant research/architecture docs if DSP, presets, metering, or delivery are involved.
3. Implement one vertical slice. Do not refactor unrelated code.
4. Add or update tests where behavior is testable.
5. Run verification (see below).
6. If verification passes: commit + push, then append a progress.md entry.
7. If verification fails: leave the work uncommitted, append a progress.md entry describing the failure and what to try next, stop.

Never advance to the next phase without a `PHASE N CONFIRMED — proceed to N+1` sentinel line in `docs/progress.md`. Dan writes that line manually after he's satisfied with phase quality.

## Verification commands

```powershell
# Frontend (run from repo root)
npm install
npm run build              # tsc -b && vite build

# Backend (run from src-tauri/)
cd src-tauri
cargo check
cargo test                            # fast lane — real-fixture tests skip with a printed advisory
$env:AMS_RUN_REAL_FIXTURE = "1"
cargo test                            # slow lane — ~5 min including the real-fixture metering snapshot
Remove-Item Env:\AMS_RUN_REAL_FIXTURE  # back to fast lane afterwards
```

See `CLAUDE.md` for the full "Test workflow — fast / slow lanes" reasoning. Run the slow lane before any commit that touches the DSP chain, the WAV writer, or LUFS landing math.

`npm run tauri dev` is the interactive smoke check (opens a window). Do not run it in autonomous sessions — it blocks. Dan runs it manually when he wants to eyeball the app.

**Dev-binary lock workaround.** When Dan has `npm run tauri dev` running, the standard `cargo test` build can fail with `cannot remove file 'target/debug/album-mastering-studio.exe'` (the executable name still uses the pre-rename slug). Two paths:

- `cargo test --lib` — lib unit tests only, doesn't link the main bin
- `cargo test --tests --target-dir target-tests` — integration tests in a scratch target dir; `rm -rf target-tests` after

Both work reliably mid-session without asking Dan to close the dev binary.

If a verification step fails, debug. Do not commit broken state.

## Progress note shape

After every verified slice, append to `docs/progress.md`:

```markdown
## YYYY-MM-DD — Phase N.M: <slice name>

Goal:

What changed:

Verification:

Real-audio fixture used:

What failed or remains partial:

Next recommended slice:
```

Keep it tight. The next session reads this to orient — don't bury the lead.

## Commit shape

```
Phase N.M: <slice name>

- Bulleted what-changed (one line per meaningful change)

Verification:
- <command>: <result>

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

Subject line under 70 chars. Push to `origin/master` after every passing slice. No feature branches — this is a single-author personal project.

## Mechanical correctness first — the workflow agreement (2026-05-15)

Dan has a day job and can't be a per-commit verification loop. The
agreement going forward:

- **Every behavioral fix ships with an automated repro test.** Write
  the failing test first (or alongside the fix), confirm it fails on
  the bug, fix until it passes, run the full suite, commit. The test
  is the gate — not Dan's ears.
- **"Verification" in commit messages ends with passing tests + grep
  evidence**, never "pending Dan's ears" or "manual verification
  required." A commit that needs Dan to verify it isn't ready to ship.
- **Listening sessions are batched, not per-commit.** When something
  needs subjective evaluation (does this preset *sound* like Punch),
  add it to a "pending listening checks" list in the active handoff
  doc. Trigger Dan only when (a) enough items have accumulated for
  an efficient session, OR (b) a specific product-taste decision is
  blocking and only Dan's ears can answer it. Default cadence: zero
  per-commit asks; batch every ~5 commits or when the next slice
  genuinely depends on a listening result.
- **Mechanical first, listening last.** Each subjective evaluation
  Dan does should already be downstream of all the automated checks
  the slice could have. If a mechanical test could catch the bug,
  write it. Don't outsource regression detection to Dan's ears.
- **Bounce-back-as-manual-testing is the failure mode.** If I find
  myself writing "try this scenario by hand and tell me if it
  works," the right move is almost always to write the test that
  answers the question mechanically.

This applies in both directions: I don't ask Dan to verify code I
wrote, and I don't claim "it works" without the test that proves it.

**Test harnesses available:**

- **Rust unit + integration tests** (`cargo test --lib`,
  `cargo test --target-dir target-tests` for full fast lane). Run
  every commit that touches `src-tauri/`.
- **Vitest (frontend, jsdom env)** via `npm test`. Picks up any
  `src/**/*.test.{ts,tsx}` file. Use for pure-TS helpers, hook
  reducers, and any logic extractable from React components. The
  canonical first test is `src/lib/effective-settings.test.ts` —
  mirrors a Rust test (`effective_settings_tests`) and gates the
  frontend's shadowing helper.
- **`npm run build`** (`tsc -b && vite build`) is a TypeScript
  type-check + production bundle. Run on every frontend change.

## Autonomy boundaries

**Allowed without asking Dan:**

- Add npm or cargo dependencies if needed for the current slice
- Install dev tooling
- Rewrite scaffolded placeholder code
- Add/modify/remove tests
- Commit + push verified slices to `master`
- Update `docs/progress.md` and `docs/IMPLEMENTATION_PLAN.md` status notes
- Replace placeholder assets (icons, CSS) when a real version is ready

**Not allowed without asking:**

- Modify `docs/PRODUCT.md` (product canon)
- Skip a phase or jump past a `PHASE N CONFIRMED` gate
- Touch `private-audio-fixtures/` (private audio)
- Force-push, rewrite history, push to a non-master branch
- Buy/install paid services or sign anything
- Make the project public

## When to stop and ask

- The slice requires a product decision `docs/PRODUCT.md` doesn't answer.
- Two consecutive slices fail verification.
- A library/framework lock-in is needed beyond what ADR 0001 already covers.
- You hit a phase boundary and there's no `PHASE N CONFIRMED` line in progress.md.
- You'd need to touch private audio fixtures.

When you stop, append a progress.md entry that clearly states the blocker.

## Scheduling autonomous runs

The `/schedule` routine prompt for this repo lives in `docs/SCHEDULE_PROMPT.md`. Dan registers it via `/schedule create` — Claude cannot register routines itself. Each scheduled run is a fresh context; this `HANDOFF.md` is the entry point every time.

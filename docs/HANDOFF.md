# Handoff — YES Master (Claude Build)

This document is the entry point for any Claude session — interactive or scheduled — picking up work on this repo. Read this first, then start the loop below.

> **Current snapshot (2026-05-15, Phase A4 shipped + 3 VM hotfixes).** Phase A4 preset retune + compressor wiring landed at `243ca18` — every preset now applies its compressor identity (threshold/ratio/attack/release) by default scaled by the user's `compression_density` macro, all 9 PRESET_* constants retuned to the conservative-target values from `docs/PRESET_REFERENCE_ANALYSIS_2026-05-14.md`. Dan confirmed audibly: presets are "really good and defined, all distinct from one another, match their name." Three hotfixes followed because the new compressor surfaced a long-latent Volume Match bug: `b4c2a57` (source_lufs injection on first chain build + compressor `powf` skip + limiter Lagrange ISP guard — Dan: "real-time play is clean"); `1b21172` (VM math rewrite — old `attenuation = source_lufs - target_lufs` was wrong because `target_lufs` is in the captured-but-not-applied list; new estimate from chain's actual gain stages lands within ~1 dB across all presets); `51477a4` (VM is session-level via wire-time override + useRef for synchronous reads — fixes "VM gets lost on track switch and stays lost"). **Hotfix-3 wasn't verified before session end** — first move next session is to confirm VM stays sync'd through clicking around, then move to open queue #1 (album-export `energy_density` literal at engine.rs:1188). Full-suite tests: `cargo test --lib` **81/81**; `cargo test` **144/144** fast lane; `npm run build` clean.
>
> **Next workstream — listening verification + open queue #1.** See `docs/HANDOFF_2026-05-15_session.md` for the listening checklist (analysis-doc lines 191–198), the structural-limit follow-up that softened two distinctness contract thresholds, the "audio thread reply timeout" toast Dan saw mid-session, and the `engine.rs:1188` album-export bug (~10 lines of fix + ~50 lines of regression test).
>
> **Codex owns the UI lane** for the moment. Do not edit `src/App.tsx`, `src/App.css`, `src/components/RightRail.tsx`, or `src/components/AlbumPanel.tsx` from the Claude side unless a UI change strictly forces it AND you've pulled latest. Phase A4 + the VM hotfixes lived in `src-tauri/src/dsp.rs`, `src-tauri/tests/preset_*.rs`, `src-tauri/src/audio.rs` (limiter/compressor perf), and `src/hooks/useTrackMaster.ts` (VM wiring) — all safe to keep iterating in.
>
> Major capabilities already live: realtime BS.1770-4 momentary + integrated LUFS metering on both Original and Mastered playback, 4-band EQ chain (200 / 400 / 1500 / 6000 Hz) with Phase A4 conservative-target preset calibration, **per-preset multiband compressor with user `compression_density` macro scaling preset baseline**, 8 delivery profile shadows, TPDF dither at integer output, 6-band FFT spectral analysis + transient flux + stereo correlation + dynamic-range P95-P10 + 3 s short-term LUFS max + energy-density composite, Album Master (4 named arcs + Custom, position-aware character inference, per-character bias), **session-level Volume Match A/B with chain-gain-stage-estimated attenuation**, **live FFT spectrum** under the EQ panel. Older session snapshots remain authoritative for back-context: `docs/HANDOFF_2026-05-15_session.md` (newest), `docs/HANDOFF_2026-05-14_session.md`, `docs/HANDOFF_2026-05-13_session.md`, `docs/HANDOFF_2026-05-12_night.md`, etc.

## Read first (in order)

1. `CLAUDE.md` — repo rules, non-negotiables, working style, fast/slow test lanes.
2. `docs/PRODUCT.md` — product canon and source of truth (now titled **YES Master Product Canon**). Do not modify without Dan's explicit ask.
3. **`docs/HANDOFF_2026-05-15_session.md`** — the latest dated handoff. Carries Phase A4 ship + 3 VM hotfix summary, listening-verification checklist, file-ownership constraints with Codex, and the open queue.
4. `docs/HANDOFF_2026-05-14_session.md` — yesterday's handoff (Phase A4 plan as written before it was executed; useful for understanding the original design intent vs what shipped).
5. `docs/PRESET_REFERENCE_ANALYSIS_2026-05-14.md` — the calibration analysis that drove Phase A4. Conservative Target Table (lines 252–259) is what landed.
6. `docs/checkpoints/checkpoint-2026-05-14-pre-preset-retune.md` — the review checkpoint that anchored Phase A4. The two ordering refinements (P4 first as failing test; ship Punch-vs-Loud crest assertion compressor-only) were both honored.
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

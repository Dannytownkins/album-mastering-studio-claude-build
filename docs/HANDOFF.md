# Handoff — Album Mastering Studio (Claude Build)

This document is the entry point for any Claude session — interactive or scheduled — picking up work on this repo. Read this first, then start the loop below.

> **Current snapshot:** Phase A1–A5 + Phase B Steps 1–7 + Volume Match LUFS fix + Codex audit slices 1–3 + Phase B+ Step 8 validation suite (7 tests) + UI restyle slices 1–2 are all done and on master. **HEAD is the latest UI-restyle commit** (check `git log -1`).  Tests green: `cargo test --lib` 80/80, `cargo test` 138/138 (80 lib + 40 contracts + 2 album_render + 6 step-8 test binaries; +1 ignored debug helper) including the real-fixture metering snapshot.  **Next unfinished workstream is UI restyle slice 3 (preset tiles)** per `docs/UI_CSS_RESTYLE_PLAN_2026-05-14.md`. After that the restyle queue continues: slice 4 (console controls), slice 4b (VisualEqPanel v1), slice 5 (right rail reorder), slice 6 (responsive check). The Codex audit's slices 6 (test split) and 7 (first-play decode latency) are queued behind the UI work.  Major capabilities now live: live BS.1770-4 momentary + integrated LUFS metering, 4-band EQ chain (200 / 400 / 1500 / 6000 Hz) with full Codex preset calibration, delivery profile shadows (8 profiles), TPDF dither in int outputs, 6-band FFT spectral analysis + transient flux + stereo correlation + dynamic-range P95-P10 + true 3 s short-term LUFS max + energy-density composite, Album Master mode (4 named arcs + Custom, position-aware character inference, per-character LUFS pull + per-character EQ/width/warmth/intensity bias), **proper LUFS-matched Volume Match A/B** (source - target attenuation, not just "undo input gain").  Older snapshots remain authoritative for the pre-A1 back-context: `docs/HANDOFF_2026-05-13_session.md`, `docs/HANDOFF_2026-05-12_night.md`, etc.

## Read first (in order)

1. `CLAUDE.md` — repo rules, non-negotiables, working style.
2. `docs/PRODUCT.md` — product canon and source of truth. Do not modify without Dan's explicit ask.
3. `docs/IMPLEMENTATION_PLAN.md` — phase map and gates.
4. `docs/progress.md` — current state; the last entry's "Next recommended slice" is where you start.
5. `docs/CLAUDE_WORK_LOOP.md` — work loop format.

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
cargo test
```

`npm run tauri dev` is the interactive smoke check (opens a window). Do not run it in autonomous sessions — it blocks. Dan runs it manually when he wants to eyeball the app.

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

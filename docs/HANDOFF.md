# Handoff — Album Mastering Studio (Claude Build)

This document is the entry point for any Claude session — interactive or scheduled — picking up work on this repo. Read this first, then start the loop below.

> **Current snapshot: `docs/HANDOFF_2026-05-13_session.md`** — closeout of the Phase 12.2 P0 wired-controls campaign, P1 polish (typography + SVG icons), AND a substantial listening-pass + bolder-layout pass on top.  HEAD `18e9040`.  Tests green (`cargo test --lib`: 32/32).  Major new capabilities: live BS.1770 momentary LUFS metering on the audio thread, three-column shell with signal-chain visualization, Save As / Open Project (`.ams.json` round-trip), and a meter-height bug fix that was responsible for "MASTER OUT seems like it's not working."  See also `docs/DANDOFF_2026-05-13.md` for Dan's testing checklist on the new machine.  Older snapshots remain authoritative for back-context: `docs/HANDOFF_2026-05-12_night.md` (compression / typography / icons plan), `docs/HANDOFF_2026-05-12_evening.md` (Phase 12.2 wired-controls campaign), `docs/HANDOFF_2026-05-12.md` (Phase 12.1).

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

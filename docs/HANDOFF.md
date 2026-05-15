# Handoff — YES Master (Claude Build)

This document is the entry point for any Claude session — interactive or scheduled — picking up work on this repo. Read this first, then start the loop below.

> **Current snapshot (2026-05-14 evening, post-audit).** App renamed Album Mastering Studio → **YES Master**. Current HEAD `4878140`. All seven UI restyle slices + the post-restyle UX restructure shipped earlier in the day; Codex then did four more passes: a console-layout pass (`a3fcc25`) that locks Track Master to a 5-row CSS grid + adds a real-PCM `MeteredPcmSource` for Original playback so peak / LUFS / FFT spectrum populate during A/B (not just Mastered) and removes Levels + Stereo Width from the deck meters column; the rename itself (`6a441d9`); the canvas bump to 1920×1080 with minWidth 1440 / minHeight 860 (`1ea2fa5`); and a zoom-reset (`4878140`) that forces 100% on every launch and reverts the variable-zoom keybindings from `4f1e53d`. Tests last green at `6a441d9`: `cargo test --lib` 80/80, `cargo test` 138/138 fast lane (`AMS_RUN_REAL_FIXTURE=1` for the slow lane). `npm run build` clean. Rerun both as a first move next session.
>
> **Next workstream — preset character retuning.** Driven by `docs/PRESET_REFERENCE_ANALYSIS_2026-05-14.md`, which compared the app's presets against real online-mastered references and found that compressor fields like `compressor_threshold_dbfs` / `compressor_ratio` are captured per preset but **not applied** in the live chain — so presets feel like minor EQ variations instead of distinct creative directions. Six sub-slices (P1–P6) are documented in `docs/HANDOFF_2026-05-14_session.md` — read that doc next. After preset retuning, the open queue's #1 is a **verified literal bug** — `engine.rs:1188 let energy_density = 0.5_f32` hardcoded in the album EXPORT render loop, dead-coding the album-arc character bias's energy-gate. See the "DSP Debt — Audit findings" section in `HANDOFF_2026-05-14_session.md` for the full audit verification + slot order.
>
> **Codex owns the UI lane** for the moment. Do not edit `src/App.tsx`, `src/App.css`, `src/components/RightRail.tsx`, or `src/components/AlbumPanel.tsx` from the Claude side unless a preset change strictly forces it AND you've pulled latest. Preset retuning lives in `src-tauri/src/dsp.rs` + `src-tauri/tests/preset_*.rs`; safe to ship there.
>
> Major capabilities already live: realtime BS.1770-4 momentary + integrated LUFS metering on both Original and Mastered playback, 4-band EQ chain (200 / 400 / 1500 / 6000 Hz) with Codex preset calibration baseline, 8 delivery profile shadows, TPDF dither at integer output, 6-band FFT spectral analysis + transient flux + stereo correlation + dynamic-range P95-P10 + 3 s short-term LUFS max + energy-density composite, Album Master (4 named arcs + Custom, position-aware character inference, per-character bias), proper LUFS-matched Volume Match A/B, **live FFT spectrum** under the EQ panel (audio-thread `rustfft` → atomic ring → snapshot tick → React render). Older session snapshots remain authoritative for back-context: `docs/HANDOFF_2026-05-14_session.md` (newest), `docs/HANDOFF_2026-05-13_session.md`, `docs/HANDOFF_2026-05-12_night.md`, etc.

## Read first (in order)

1. `CLAUDE.md` — repo rules, non-negotiables, working style, fast/slow test lanes.
2. `docs/PRODUCT.md` — product canon and source of truth (now titled **YES Master Product Canon**). Do not modify without Dan's explicit ask.
3. **`docs/HANDOFF_2026-05-14_session.md`** — the latest dated handoff. Carries the preset-retuning workstream plan (P1–P6), file-ownership constraints with Codex, and the open queue.
4. `docs/PRESET_REFERENCE_ANALYSIS_2026-05-14.md` — the calibration analysis driving the next workstream. The Conservative Target Table (lines 252–259) is the values to land on.
5. `docs/IMPLEMENTATION_PLAN.md` — phase map and gates (back-context; mostly closed).
6. `docs/progress.md` — append-only slice log; tail entry is "where we are now."
7. `docs/CLAUDE_WORK_LOOP.md` — work loop format.

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

# Handoff — YES Master (Claude Build)

This document is the entry point for any Claude session — interactive or scheduled — picking up work on this repo. Read this first, then start the loop below.

## Cross-platform considerations

- Build commands: macOS uses `npm run build:mac` (`tauri build --bundles app,dmg`); Windows uses `npm run build:windows` (`tauri build --bundles msi,nsis`). Tauri's Windows installer docs name MSI and NSIS as the Windows installer outputs; `app` is the macOS app-bundle target, so do not change the Windows script to `app,msi`. The Windows script uses `rimraf` to remove `produce_dialog_smoke.exe` cross-shell before bundling.
- Windows bundling has an explicit `bundle.windows.webviewInstallMode` block set to Tauri's documented default (`downloadBootstrapper`, silent). This is intentional until the first Windows build proves whether a different WebView2 install mode is needed.
- Code signing: macOS is currently ad-hoc signed for local use; wider macOS distribution needs Apple Developer ID + notarization. Windows distribution needs Authenticode signing once YES Master leaves Dan's own machines.
- Save/export paths: the Tauri dialog plugin returns native paths on each OS (`/` on macOS/Linux, `\` on Windows). Frontend tests now pin both separator styles flowing through to render unchanged.

> **Current snapshot (2026-05-19 Mac pickup).** YES Master is treated as a private cross-platform desktop mastering app - Mac and Windows targeted; Linux deferred. Latest dated handoff is `docs/HANDOFF_2026-05-19_evening.md`; read that first after `CLAUDE.md` and `docs/PRODUCT.md`, then read the tail of `docs/progress.md`. Current `master` contains the full Phase B seven-band EQ slice, the post-merge Album Master/Visual EQ CSS polish (`4aba442`), the cross-machine handoff corrections, and the Mac SHA portability closeout recorded in `docs/progress.md`. The safety tag `pre-eq-7-band-2026-05-19` is pushed to origin at `450a14f`.
>
> **Prior session detail.** The 22-commit Phase A4 session lives in `docs/HANDOFF_2026-05-15_evening.md`; the 2026-05-18 audio split and evening inventory live in `docs/HANDOFF_2026-05-18_evening.md`.
>
> **Addendum (2026-05-18 evening).** Export destination UX now asks for explicit save locations for track and album exports, the duplicate Album Export button was removed, the legacy frontend `exportAlbum` hook was removed, and deferred taste/infrastructure checks live in `docs/followups/listening-batch-2026-05-19.md` plus `docs/followups/infrastructure-2026-05-19.md`.
>
> **Addendum (2026-05-19).** Cross-platform packaging parity is now guarded mechanically: `npm run build:mac` builds `app,dmg`; `npm run build:windows` builds `msi,nsis`; `src/lib/mac-app-packaging.test.ts` and `src/lib/windows-app-packaging.test.ts` statically pin Tauri config, icons, scripts, explicit `bundle.windows.webviewInstallMode`, cross-shell `rimraf`, and release binary hygiene. Windows installer execution still must be verified on Windows.
>
> **Addendum (2026-05-19 late UI polish).** The visible preset/signal-chain area was tightened without platform-specific code: the signal chain is a static bar, the old dropdown affordance is gone, preset saving moved into a compact `+` beside the Preset label, and the preset row plus signal-chain row now share an 8-column rhythm. These React/CSS changes apply to macOS and Windows builds.
>
> **Addendum (2026-05-19 Phase B).** The seven-band EQ expansion landed on master through `b424b36`; it adds Sub, Low-Mid, High-Mid, and Sparkle Visual EQ nodes around the existing Low/Mid/High Tone Shape knobs while preserving byte-identical chain-output SHA snapshots. Post-merge CSS polish at `4aba442` removed the duplicate Album Master album-order row, prevented desktop vertical scrolling in Album Master at Dan's native Windows resolution, and reduced the primary EQ node/halo thickness.
>
> **Addendum (2026-05-19 Mac SHA closeout).** The first Mac `cargo test preset_byte_identity` run produced stable Mac-specific SHAs for the eight named presets while Custom stayed identical. `src-tauri/src/dsp.rs` now keeps Windows and macOS SHA snapshots side-by-side, selected by OS, so both machines are guarded without changing the audio path.
>
> **Test totals:** Latest full Phase B final gate on Windows: Rust lib **174/174**, Vitest **81/81**, `npm run build` clean, full `cargo test` green, and `AMS_RUN_REAL_FIXTURE=1 cargo test` green. Latest Mac pickup gate: `cargo test preset_byte_identity` 10/10 after OS-gated SHAs, Vitest **81/81**, `npm run build` clean, Rust lib **173/173**, full `cargo test` green, and `npm run build:mac` produced `.app` + `.dmg`. The 174→173 Rust lib count drift is recorded in `docs/progress.md`; no failure was observed.
>
> **What's open / next.** Listening-taste work for Sub / High-Mid / Sparkle is still deferred until Dan has studio monitors. While that is blocked, the practical non-listening options are the documented infrastructure/cleanup follow-ups: `apply_album_shadow` support for the new bands, the separate `process_sample` low-mid fix slice, the `album_render` dead-code decision, the optional `science_note` tooltip, or the eagle-eye audit. Private audio fixtures are intentionally not tracked; only run the real-fixture slow lane on the Mac if Dan intentionally brings those files over.
>
> **Codex owns the UI lane** for the moment. Do not edit `src/App.tsx`, `src/App.css`, `src/components/RightRail.tsx`, or `src/components/AlbumPanel.tsx` from the Claude side unless a UI change strictly forces it AND you've pulled latest. App.tsx WAS touched this session for the B7 / LoudnessTarget fixes; coordinate before any further App.tsx work.
>
> **New pattern: `src/lib/` pure helpers + co-located Vitest.** Five modules so far:
> - `src/lib/effective-settings.ts` (+ test) — read-direction shadowing helpers + LoudnessTarget display.
> - `src/lib/settings-transitions.ts` (+ test) — write-direction transitions: B7 auto-flip, LoudnessTarget force-flip, VM session-level + source_lufs injection.
> - `src/lib/history-stack.ts` (+ test) — undo/redo stack arithmetic, generic over T.
> - `src/lib/compressor-auto.ts` (+ test) — read-only Auto compressor values with units for the UI lane.
> - `src/lib/export-location.ts` (+ test) — last-used export folder persistence and cross-platform path helpers.
>
> Future frontend slices: extract decision logic into `src/lib/*`, write Vitest cases next to it, glue from the hook.
>
> Recent UI regression gates:
> - `src/components/SignalChain.test.tsx` — signal chain remains static, with no dropdown affordance.
> - `src/App.preset-save.test.tsx` — preset save lives in the Preset header and the separate save row stays removed.
> - `src/App.layout-css.test.ts` — preset row and signal chain share the aligned 8-column rhythm.
>
> Major capability inventory lives in `docs/HANDOFF_2026-05-19_evening.md`, `docs/HANDOFF_2026-05-18_evening.md`, and `docs/PRODUCT.md`; keep this file focused on current state and handoff mechanics.

## Mac pickup prompt

Historical prompt used for the first Mac pickup after the Windows seven-band EQ merge:

```text
We are picking up YES Master on the Mac laptop after the 2026-05-19 Windows seven-band EQ merge.

Please:
1. Pull latest master and tags: git pull --ff-only && git fetch --tags.
2. Read CLAUDE.md, docs/PRODUCT.md, docs/HANDOFF.md, docs/HANDOFF_2026-05-19_evening.md, docs/followups/infrastructure-2026-05-19.md, and the tail of docs/progress.md.
3. Confirm HEAD is 871d3ae or newer and tag pre-eq-7-band-2026-05-19 points at 450a14f.
4. From src-tauri, run cargo test preset_byte_identity first. This was the first Mac check for the Windows-pinned Phase B SHA snapshots and is now closed by the Mac SHA closeout progress entry.
5. From the repo root, run npm install if needed, npm test, npm run build, and npm run build:mac if local Tauri packaging prerequisites are installed.
6. From src-tauri, run cargo test --lib and cargo test. Run AMS_RUN_REAL_FIXTURE=1 cargo test only if the private real fixture exists locally on the Mac.
7. Do not start listening-taste changes, async worker changes, Reference Track UX, Album Master dashboard/report work, dead-code cleanup, or process_sample fixes until Dan nominates the next slice.

If any command fails, stop and report the exact command + failure before changing behavior.
```

## Read first (in order)

1. `CLAUDE.md` — repo rules, non-negotiables, working style, fast/slow test lanes.
2. `docs/PRODUCT.md` — product canon and source of truth (now titled **YES Master Product Canon**). Do not modify without Dan's explicit ask.
3. **`docs/HANDOFF_2026-05-19_evening.md`** — latest dated handoff and current entry point.
4. `docs/followups/listening-batch-2026-05-19.md` — deferred subjective checks.
5. `docs/followups/infrastructure-2026-05-19.md` — distribution/cleanup debt.
6. `docs/HANDOFF_2026-05-15_evening.md` — prior major architecture snapshot.
7. `docs/HANDOFF_2026-05-15_session.md` — Phase A4 + VM hotfix back-context.
8. `docs/checkpoints/checkpoint-2026-05-15-end-of-mechanical-gates-session.md`.
9. `docs/checkpoints/checkpoint-2026-05-15-post-phase-a4-vm-hotfixes.md`.
10. `docs/PRESET_REFERENCE_ANALYSIS_2026-05-14.md` — calibration analysis.
11. `docs/IMPLEMENTATION_PLAN.md` — phase map and gates.
12. `docs/progress.md` — append-only slice log.
13. `docs/CLAUDE_WORK_LOOP.md` — work loop format.

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
# PowerShell / Windows
# Frontend (run from repo root)
npm install
npm run build              # tsc -b && vite build
npm test
npm run build:windows      # Windows machine only; emits NSIS .exe + MSI

# Backend (run from src-tauri/)
cd src-tauri
cargo check
cargo test                            # fast lane — real-fixture tests skip with a printed advisory
$env:AMS_RUN_REAL_FIXTURE = "1"
cargo test                            # slow lane — ~5 min including the real-fixture metering snapshot
Remove-Item Env:\AMS_RUN_REAL_FIXTURE  # back to fast lane afterwards
```

```bash
# Bash / macOS or Linux
# Frontend (run from repo root)
npm install
npm run build              # tsc -b && vite build
npm test
npm run build:mac          # macOS machine only; emits .app + .dmg

# Backend (run from src-tauri/)
cd src-tauri
cargo check
cargo test                            # fast lane — real-fixture tests skip with a printed advisory
AMS_RUN_REAL_FIXTURE=1 cargo test     # slow lane — ~5 min including the real-fixture metering snapshot
unset AMS_RUN_REAL_FIXTURE            # back to fast lane afterwards if exported earlier
```

See `CLAUDE.md` for the full "Test workflow — fast / slow lanes" reasoning. Run the slow lane before any commit that touches the DSP chain, the WAV writer, or LUFS landing math.

`npm run tauri dev` is the interactive smoke check (opens a window). Do not run it in autonomous sessions — it blocks. Dan runs it manually when he wants to eyeball the app.

**Dev-binary lock workaround.** When Dan has `npm run tauri dev` running, the standard `cargo test` build can fail while replacing the running desktop binary: `target/debug/album-mastering-studio.exe` on Windows or `target/debug/album-mastering-studio` on macOS/Linux. Two paths:

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

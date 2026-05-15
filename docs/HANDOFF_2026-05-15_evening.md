# Handoff — YES Master — 2026-05-15 evening

> **One-paragraph snapshot.** 22 commits today on top of the Phase A4 morning handoff (`4b9b7e9`), every one mechanically gated. The full B1–B7 audit queue resolved with regression tests, four perf concerns closed (8 s preview window + VM cap + coalescer with playback barriers + landing-gain cache by settings hash), three-tier PCM resolution (local cache → shared prewarm cache → fresh decode) eliminates the 1–2 s freeze on first Mastered click, shared ceiling-bounded landing helper replaces the three drift-prone copies, Vitest scaffolded with three pure-helper modules in `src/lib/`. Most recent commit also addresses four Codex review items: auto-prewarm on import/restore/openProject paths, stale-prewarm-evicts-newer guard, `source_lufs_integrated` doc/test-name drift, and LoudnessTarget display extracted to a pure helper. HEAD: `74d704f`. Tests: `cargo test --lib` **144/144**, `cargo test --target-dir target-tests` full fast lane pass, **Vitest 43/43** across 3 modules, `npm run build` clean. **Autonomous queue is effectively empty** — next meaningful progress depends on Dan's listening session or product direction.

## Read first (in order)

1. `CLAUDE.md` — repo non-negotiables + fast/slow test-lane workflow.
2. `docs/PRODUCT.md` — product canon. Title is **YES Master**. Do not modify without explicit ask.
3. **This file** (`docs/HANDOFF_2026-05-15_evening.md`).
4. `docs/HANDOFF_2026-05-15_session.md` — morning's handoff (Phase A4 ship + 3 VM hotfixes). Useful for back-context on what was in flight at the start of today.
5. `docs/checkpoints/checkpoint-2026-05-15-end-of-mechanical-gates-session.md` — the session inventory written before the Codex review items landed. Captures the architectural state, file-size growth, and the trust-pattern fix consolidation.
6. `docs/checkpoints/checkpoint-2026-05-15-post-phase-a4-vm-hotfixes.md` — the morning audit checkpoint that opened the B1–B7 queue.
7. Tail of `docs/HANDOFF.md` — the always-loaded entry point. Already updated with the new snapshot text.

Do not read by default: `docs/reference/`, `docs/research/`. Optional context for the original port.

## Current branch state

- **HEAD**: `74d704f` (Codex review fixes: auto-prewarm, stale-prewarm guard, doc drift, LoudnessTarget display helper).
- **Remote**: pushed to `origin/master`. No feature branches.
- **Local tree**: clean. `src-tauri/target-tests/` is now gitignored explicitly.
- **Tests**:
  - `cargo test --lib` **144/144** (~3 s)
  - `cargo test --target-dir target-tests` full fast lane pass
  - `npm test` **43/43** across `effective-settings.test.ts`, `settings-transitions.test.ts`, `history-stack.test.ts` (~5 ms test runtime, ~800 ms total)
  - `npm run build` clean (~600 ms)

## What landed this session (22 commits)

The morning handoff opened with the B1–B7 audit queue unresolved. Today closed all of it and most adjacent perf concerns.

| Commit | Slice |
|---|---|
| `568f559` | B3: Strip VM from export paths + saturation chip retune + first audit checkpoint |
| `8026889` | Match live preview to export LUFS landing (Codex parallel work) |
| `1c271de` | B6: ceiling-bounded LUFS landing — let the slider push upward (track + preview paths) |
| `16cfe33` | B7: auto-flip to Custom when manually editing shadowed export fields |
| `3322bda` | Batch 1: B1 (album energy_density) + B4 (ISO_PLACEHOLDER timestamps) + B5 (album-simple LUFS landing) |
| `26caf75` | B6 follow-up: ceiling-bounded landing on album-plan render path |
| `e3948db` | Fix VM over-attenuation cap + 8 s preview-LUFS window |
| `5979445` | Coalesce UpdateChain in audio command loop |
| `d92fe4a` | Mechanical gates for the three live-preview repros |
| `b4461ae` | Handoff: mechanical-first workflow agreement |
| `61f1a69` | Cache live-preview landing gains by settings hash |
| `53e0317` | Extract shared ceiling-bounded landing helper |
| `3b5e59b` | LoudnessTarget readout reflects effective target (B7 read-direction sibling) |
| `9ca1fd9` | B2: symmetric-range integer quantization for 16/24-bit export |
| `1f6c6a4` | Coalescer: playback barriers + mtime-aware landing-cache invalidation |
| `4106009` | Vitest scaffold + first frontend mechanical-gate test |
| `dac1408` | Decode-stall fix: prewarm cache populated off the audio thread |
| `7db4874` | gitignore: target-tests/ (dev-binary-lock workaround scratch dir) |
| `4f33828` | Extract B7 + LoudnessTarget flip logic, add Vitest gates |
| `9b6ab29` | Extract withSourceLufs → applyChainDispatchOverrides + Vitest gates |
| `cc797be` | Checkpoint: end of mechanical-gates session (20 commits) |
| `3890b6c` | Extract undo/redo stack math, add Vitest gates |
| `74d704f` | Codex review fixes: auto-prewarm + stale-prewarm guard + doc drift + LoudnessTarget display helper |

**Test totals: 81 → 144 Rust lib (+63), 0 → 43 Vitest (+43), 0 new "needs Dan's listening" items in the deferred queue.**

## Pattern that consolidated this session: `src/lib/` pure helpers + co-located Vitest

Every frontend slice now follows the same shape. Decision logic lives in `src/lib/*.ts` as pure functions; tests live next to them as `*.test.ts`; the React hook holds only state-glue. Four modules in place:

```
src/lib/
├── api.ts                       (Tauri command wrappers — no tests yet)
├── effective-settings.ts        (read-direction shadowing helpers + LoudnessTarget display)
├── effective-settings.test.ts   (13 Vitest cases)
├── settings-transitions.ts      (write-direction transitions: B7 auto-flip, LoudnessTarget force-flip, VM session-level)
├── settings-transitions.test.ts (16 Vitest cases)
├── history-stack.ts             (undo/redo stack arithmetic, generic over T)
├── history-stack.test.ts        (14 Vitest cases)
├── preview-mock.ts              (dev preview backend)
└── tauri-runtime.ts             (Tauri shim)
```

**Next frontend slice should follow the same pattern.** Decision logic extractable from a React callback → put it in `src/lib/`, write Vitest cases, glue from the hook.

## Trust-pattern fixes consolidated (5 user-visible touchpoints)

Five "what the UI shows vs what export does" disagreements were untested at session start. All are now gated:

| Fix | Direction | Gate |
|---|---|---|
| **B3** (VM in export) | write (chain) | `tests/export_volume_match.rs` — Rust integration |
| **B7** (auto-flip on edit) | write (UI → settings) | `applyAdvancedWithProfileFlip` — Vitest |
| **LoudnessTarget readout** | read (settings → UI) | `loudnessTargetDisplay` / `effectiveLoudnessTarget` — Vitest |
| **LoudnessTarget pick force-flip** | write (UI → profile) | `shouldFlipToCustomOnLoudnessPick` — Vitest |
| **VM session-level** (Phase A4 hotfix-3) | write (transport → chain) | `applyChainDispatchOverrides` — Vitest |

## Four-layer perf defense on live preview

The morning had two real perf complaints from Dan (audio seek reply timeout, VM over-attenuation on aggressive settings). All four layers now in place:

1. **8 s preview window** in `export_landing_gain_lin_for_preview` — full chain + BS.1770 runs on the middle 8 s instead of the full PCM (~15-20x faster per call).
2. **VM cap** in `dsp.rs::from_settings` — bounds the raw chain-push estimate by `(ceiling - typical_crest) - source_lufs` so aggressive settings can't over-attenuate.
3. **Coalescer + playback barriers** in `audio.rs::coalesced_command_sequence` — knob-spam UpdateChains collapse to the latest per segment; Play / PlayMaster / Stop split the queue so a pre-barrier stale UpdateChain can't be reordered after a track switch.
4. **Landing-gain cache** (`PreviewLandingCache`) on `AudioThreadState` — settings-hash keyed, zero-cost repeat lookup, cleared on track change via path-OR-mtime mismatch.

The 4 layers compose: knob-spam during a settings change pays at most ONE 20 ms measurement; replaying the same settings is free; track switches don't poison either cache.

## Decode-stall fix (three-tier PCM resolution)

The morning's first-Mastered-click 1-2 s freeze on long WAVs is gone. `resolve_pcm_with_caches` in `audio.rs` consults:

1. **Tier 1**: `AudioThreadState.decoded_cache` (currently-playing PCM). Used by UpdateChain for live-preview measurements — never touched by prewarm so prewarming a different track can't poison live preview.
2. **Tier 2**: `SharedDecodedCache` on `AudioPlayer`. Populated by the new `prewarm_decode` Tauri command on `tauri::async_runtime::spawn_blocking`, so the decode runs off the audio thread. **Prewarm-target guard** (Codex review fix): each prewarm declares its target at start and checks the target still matches before writing — slow prewarms whose target was superseded drop their result silently.
3. **Tier 3**: Fresh `decode_full` on the audio thread (the cold path prewarm exists to avoid).

`prewarm_decode` is fired fire-and-forget from four frontend paths: `selectTrack` (explicit click), `loadRecentSession` (autosave restore), `importTracks` (file import auto-select), `openProjectFromDisk` (project open). Auto-select coverage is the Codex review fix that landed in `74d704f`.

## Most recent commit: Codex review items (74d704f)

Four mechanical items Codex flagged after the prewarm cache + coalescer barriers landed:

1. **Auto-prewarm gap**: `prewarmDecode` fired only on `selectTrack`. Three other auto-select paths bypassed it. Fixed by adding fire-and-forget `api.prewarmDecode(path)` to each.
2. **Stale-prewarm-evicts-newer race**: single-slot cache could be overwritten by a slow prewarm finishing after a newer selection. Fixed via `prewarm_target` on `AudioPlayer` + a target-match check before writing the cache.
3. **`source_lufs_integrated` doc/test drift**: the test `volume_match_independent_of_source_lufs` and a dsp.rs comment claimed VM didn't use source LUFS, but the cap (added later) does. Renamed test to `volume_match_uncapped_estimate_independent_of_source_lufs` with a docstring pointing to `volume_match_caps_attenuation_at_limiter_bound` for the capped case. Updated the dsp.rs comment with a "LATER REFINEMENT" block.
4. **Full mock-API Vitest tests**: deferred. The "LoudnessTarget effective readout" half landed as `loudnessTargetDisplay` + 8 new Vitest cases. The "auto-prewarm" and "Export LUFS toggle dispatch" halves need a component-render harness (`@testing-library/react`) which is its own scaffolding slice — see "Open follow-ups" below.

## Codex's parallel lane

Codex still owns UI / CSS files in principle. **App.tsx was touched this session** for the B7 + LoudnessTarget fixes; coordinate before any further App.tsx work to avoid merge conflict.

Files **safe to edit** for next likely workstreams:

- `src-tauri/src/dsp.rs` — chain coefficients, presets, VM cap math
- `src-tauri/src/engine.rs` — render paths, landing helpers, energy density wiring
- `src-tauri/src/audio.rs` — audio thread, MasteringSource, three-tier cache (the file is now 3,558 lines; future split candidate)
- `src-tauri/src/album.rs` — album planning + character bias
- `src-tauri/src/types.rs` — type definitions + `effective_*` accessors
- `src-tauri/tests/*.rs` — all integration tests
- `src/hooks/useTrackMaster.ts` — state hook; net shrinkage this session as logic extracted into `src/lib/`
- `src/lib/*.ts` — the new pure-helper modules + their Vitest tests
- `src/lib/api.ts` — Tauri command wrappers (stable)

## Open follow-ups (none blocking, prioritized)

### Codex's item 4 completion — mock-API Vitest coverage

The renderHook-based half of Codex's review item 4 is deferred:
- **Auto-prewarm spy tests**: mock `api.prewarmDecode`, render the hook, simulate import / restore / openProject, assert the spy was called with the right path.
- **Export LUFS toggle dispatch**: mock `api.updateChain`, render the hook, call `setExportLufsPreview(true)`, assert dispatch with `previewLufsLanding=true`.
- **LoudnessTarget pick force-flip end-to-end**: render the component, click a dropdown option, assert `onDeliveryProfile("custom")` + `onAdvanced(...)` were called.

Requires scaffolding `@testing-library/react` (devDep + jsdom is already in place). One slice. The pure-helper coverage from this session is the minimum gate; full integration would compound it.

### Async live-preview measurement on a worker thread

Paused this session. The four-layer perf defense closed the audible cliff Dan reported. Residual cold-path cost is ~20 ms per genuinely-novel settings change — well within the audio loop's 50 ms tick budget. Multi-hour slice with non-trivial new surface (version stamping, cancellation on track-change, worker lifetime). Pursue only if Dan re-prioritizes after listening.

### `audio.rs` split candidate

3,558 lines mixing five concerns (PCM/decode caches, audio command loop, coalescer, source + metering, tests). Natural fault lines exist for a future refactor session. Not urgent — file is still readable and grew mostly via co-located tests this session.

### Reference Track feature

Dan said earlier he wants to think about the UX before implementation. Skipped this session.

## Pending listening checks (batch for Dan's next session)

Carried forward from the morning checkpoint, plus new items from today's fixes. Roll through these whenever Dan has a focused listening hour — none are asked per-commit per the mechanical-correctness-first workflow agreement:

1. **B6 ceiling-bounded LUFS landing** — does the Loudness Target slider feel responsive on quieter sources / lower-intensity material across all three render paths (track / album-simple / album-plan)?
2. **VM cap on aggressive settings** — Dan's reproducible case (Tape preset + Intensity 100% + +13 dB input gain + VM on). Should land near source LUFS (~-13 to -14), not -24.
3. **Decode-stall fix end-to-end** — select track, click Mastered, the swap should be sub-100 ms instead of the prior 1-2 s freeze. Also verify the auto-prewarm paths (import a new file → click Mastered fast; restart app → click Mastered immediately; open a saved project → click Mastered).
4. **LoudnessTarget readout truthfulness** — pick a non-Custom delivery profile from the dropdown, readout should show the profile's target (e.g. StreamingUniversal → -14, LoudRock → -10.5) instead of "—".
5. **Preset character on real material** — does each preset still deliver its named identity post-Phase A4 retune? Dan's existing analysis-doc listening pass on `It's a coat` covers this.

## Verification commands

```powershell
# Frontend (run from repo root)
npm install
npm test                    # Vitest, 43/43 in ~1 s
npm run build               # tsc -b && vite build, clean ~600 ms

# Backend (run from src-tauri/)
cd src-tauri
cargo check                 # quick syntax/type pass
cargo test --lib            # ~3 s, 144 tests, no integration
cargo test                  # full fast lane — real-fixture tests skip without AMS_RUN_REAL_FIXTURE=1
$env:AMS_RUN_REAL_FIXTURE = "1"
cargo test                  # slow lane, ~5 min if private fixture is present
Remove-Item Env:\AMS_RUN_REAL_FIXTURE
```

**Dev binary lock workaround** (when `npm run tauri dev` is running and `cargo test` fails with "cannot remove file `target/debug/album-mastering-studio.exe`"):

```powershell
cargo test --lib                              # lib only — bypasses the lock
cargo test --tests --target-dir target-tests  # scratch build dir, all tests
```

`target-tests/` is gitignored as of this session.

## Memory carried forward

Durable preferences from prior sessions + this session's working agreements:

- **Mechanical correctness first.** Every behavioral fix ships with an automated repro test. Tests are the gate, never "pending Dan's ears." Listening sessions are batched, not per-commit. Captured in `HANDOFF.md` above the autonomy boundaries.
- **High autonomy on this repo.** Install deps, run tests, commit + push to master when work is verified. No feature branches; single-author. Do NOT push a non-master branch.
- **No check-in chatter.** When Dan says "dive in" / "keep going" / "go", chain commits. Don't `AskUserQuestion` between every slice.
- **Hold evidence under pressure.** If the test or the data backs the call, hold it. Don't capitulate to social pressure.
- **No under-building.** Dan needs features day-one. Don't v1-then-v2-stage when the spec is clear.
- **Listening calls are Dan's.** Sound-quality decisions only happen when Dan signals "I listened to it." Don't fake them.
- **`PRODUCT.md` is canon.** Read-only without explicit ask.
- **Codex collaborates on UI.** Codex owns App.tsx, App.css, RightRail.tsx, AlbumPanel.tsx. App.tsx WAS touched this session for B7 + LoudnessTarget; pull first before any further App.tsx work.
- **Pure-helper-in-`src/lib/` + co-located Vitest is the new pattern** for frontend slices. Established this session — three modules in place, follow the shape for the next.

## When to stop and ask

- The slice requires a product decision `docs/PRODUCT.md` doesn't answer.
- A listening-verification step needs Dan's ears AND nothing else mechanical can be done first.
- You would need to scaffold a major new dev dependency (e.g., `@testing-library/react` for the deferred Codex item 4) — that's its own slice, worth confirming scope.
- Two consecutive slices fail their own contract tests.
- Codex has recently touched a file you're about to edit and you haven't pulled.

When you stop, append a `progress.md` entry that clearly states the blocker.

## Commit shape

Match this session's established pattern:

```
<slice tag>: <slice name>

<one-paragraph what + why, including why-the-fix-shape>

<file-level changes bulleted>

<mechanical gates added — list each test name + what it asserts>

Verification:
- cargo check: clean
- cargo test --lib: N/N pass
- cargo test --target-dir target-tests: full fast lane pass
- npm test: M/M pass (Vitest)
- npm run build: clean

<listening note: required / not required + why>

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

Subject under 70 characters. Push to `origin/master` after every passing slice.

## Codex review addendum — handoff completeness check

Codex reviewed this handoff read-only after commit `47861b3`. Overall verdict:
the handoff is strong enough for a fresh session to continue without
rediscovering the architecture or pulling Dan into per-commit QA. The
mechanical-first workflow agreement is clear, the major live-preview/export
architecture is captured, and the recent Codex review fixes are represented.

Small consistency fixes recommended for the next doc-maintenance pass:

1. **`docs/progress.md` is stale relative to this handoff.** `HANDOFF.md`
   still says the tail of `progress.md` is "where we are now," but the tail
   still points to the old VM hotfix / `energy_density` follow-up that this
   evening session superseded. Either append a current `progress.md` entry for
   the 22-commit session, or update `HANDOFF.md` to say this evening handoff
   overrides `progress.md` until progress is refreshed.

2. **Clarify HEAD wording.** The code snapshot commit is `74d704f`, but the
   current repo HEAD after the handoff-doc commit is `47861b3`. Best wording:
   "code snapshot: `74d704f`; docs HEAD after handoff: `47861b3`."

3. **Normalize the `src/lib/` module count.** Some wording says "four pure
   helper modules," while the tested pure-helper modules are three:
   `effective-settings`, `settings-transitions`, and `history-stack`. Better
   phrasing: "three tested pure-helper modules, plus API / preview / Tauri
   support wrappers."

4. **Keep Dan out of mechanical prewarm QA.** The pending listening list is
   useful, but the auto-prewarm dispatch paths should be covered by the
   deferred mock-API Vitest slice rather than assigned to Dan. Dan's listening
   batch should stay focused on subjective UX/audio feel: whether playback
   feels snappy and whether the audio behavior matches expectations.

Fresh quick-gate spot check during this review:

- `npm test`: 43/43 pass.
- `cargo test --lib --target-dir target-tests`: 144/144 pass.

# Album Mastering Studio Implementation Plan (Claude Build)

Last updated: 2026-05-19

This is the execution map for the Claude-build repo of Album Mastering Studio (`album-mastering-studio-claude-build`). `docs/PRODUCT.md` is the product canon; this file is the living implementation plan. If anything below conflicts with `docs/PRODUCT.md`, treat `PRODUCT.md` as authoritative and flag the drift.

This repo is now the active Tauri + Rust implementation. Older Codex/Python reference work may still be useful for historical comparison, but no source should be imported from it unless the user explicitly asks.

## Current Strategic Direction

Build a top-tier private cross-platform desktop mastering app — Mac and
Windows targeted; Linux deferred — around two modes:

1. Track Master.
2. Album Master.

Track Master ships first as the core vertical slice, but Album Master is a required near-term path because the user's personal project needs full-album mastering. Track Master is the first proof of the shared foundation, not the final destination.

The product must support real-time or near-real-time audition before Track Master can be called release-candidate. Non-real-time preview rendering is allowed only as temporary scaffolding while the real-time path is being built and proven.

## Non-Negotiable Product Gates

Track Master cannot be considered top-tier until it has:

- Drag/drop audio import.
- Analyze.
- Safe Universal settings.
- Large waveform.
- Waveform zoom.
- Region selection.
- Loop selected region.
- Original/Mastered toggle at the same playhead.
- Optional Volume Match, off by default.
- Functional preset tiles.
- Functional Intensity macro.
- Functional Low/Mid/High EQ.
- Whole-track mastered preview.
- Stale preview state when controls change.
- Real-time or near-real-time audition for basic ear-facing controls.
- One obvious Export Master action.
- Advisory post-render quality checks.
- Non-overwriting output.
- Autosave.
- Undo/redo for non-destructive state.

Album Master cannot be considered top-tier until it has:

- Track ordering.
- Analyze.
- Global album intent plus per-track adaptation.
- Track Roles / Story step after analysis.
- Editable role/character decisions.
- Individual masters.
- Continuous album WAV by default.
- Preserved source boundaries by default.
- Gap/crossfade/boundary primitives (preserve / direct / timed gap / equal-power crossfade / fade / ring-out). AI-generated transitions are out of scope per PRODUCT.md #42.
- Cue/split data when appropriate.
- Album dashboard/report.
- Album-level quality checks.

## Workstream Overview

1. Phase 0  — Workspace scaffold and architecture ADR.
2. Phase 1  — Rust/Tauri typed app foundation.
3. Phase 2  — Track Master frontend skeleton.
4. Phase 3  — Source playback, waveform, and A/B foundation.
5. Phase 4  — Offline universal mastering chain and stale-preview state.
6. Phase 5  — Real-time audition engine.
7. Phase 6  — Track Master export and quality checks.
8. Phase 7  — Presets, custom settings, autosave, undo.
9. Phase 8  — Album Master mode (sequence, intent, per-track adaptation).
10. Phase 9  — Album Story / Roles step.
11. Phase 10 — Transition primitives.
12. Phase 11 — DSP audit and modernization.
13. Phase 12 — Private real-audio fixture loop.
14. Phase 13 — Performance budgets.
15. Phase 14 — Release and installer hardening.

These streams can overlap, but every phase must end with a no-victory-lap check against `docs/PRODUCT.md` and an entry in `docs/progress.md`.

## Phase 0: Workspace Scaffold And Architecture ADR

**Status: DONE (2026-05-11).** ADR 0001 written, workspace scaffolded, baseline commands verified. See `docs/progress.md` Phase 0 entry.

Goal: take this repo from zero-state to a buildable Tauri app shell, with the architecture decision recorded.

Tasks:

- Write `docs/adr/0001-tauri-rust-stack.md`. Compare Tauri+Rust audio, JUCE, Rust-native UI/audio, and hybrid shells against `docs/PRODUCT.md` gates: real-time audition, export parity, desktop packaging, offline rendering quality, file safety, testability. Recommend Tauri 2.x shell + Rust audio engine (`cpal` for output, `symphonia` for decode, `hound` for WAV write, hand-rolled DSP initially). Note JUCE and Rust-native UI as reversible fallbacks if the Phase 5 real-time spike misses latency targets. List risks and what stays reversible.
- Scaffold workspace structure:
  - Root: `package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`.
  - Frontend in `src/`: React + TypeScript + Vite, with a placeholder `App.tsx` and a single CSS file.
  - Tauri app in `src-tauri/`: `Cargo.toml`, `tauri.conf.json`, `build.rs`, `src/main.rs`, `src/lib.rs`, empty `audio/`, `engine/`, `files/`, `project/`, `exports/`, `jobs/`, `settings/` module folders.
  - `docs/adr/` directory containing the ADR above.
  - `docs/progress.md` seeded with the Phase 0 session entry.
- Confirm baseline commands work on a clean machine.

Baseline verification commands:

```powershell
# PowerShell / Windows
npm install
npm run tauri dev        # opens a placeholder window
cd src-tauri
cargo check
cargo test               # zero tests, must pass
```

```bash
# Bash / macOS or Linux
npm install
npm run tauri dev        # opens a placeholder window
cd src-tauri
cargo check
cargo test               # zero tests, must pass
```

No-victory-lap check:

- An empty Tauri window is not the product; it is just a baseline.
- The architecture choice is reversible per the ADR; do not act as if Tauri-forever has been decided.

## Phase 1: Rust/Tauri Typed App Foundation

**Status: DONE (2026-05-11).** 16 typed `#[tauri::command]` handlers in `src-tauri/src/lib.rs`; contract tests in `src-tauri/tests/contracts.rs`; hand-written TS bindings in `src/bindings.ts`. See `docs/progress.md` Phase 1 entry.

Goal: make the backend speak product concepts, not raw shell strings, before any DSP work.

Desired modules in `src-tauri/src/`:

- `engine`: DSP entry points; in-process Rust initially. External sidecar deferred.
- `jobs`: analyze/render job queue with progress and cancel.
- `files`: import validation, source metadata, path safety, format-neutral handling.
- `project`: autosave, project files, recent sessions.
- `settings`: user presets and settings chains.
- `audio`: playback, A/B preview state, waveform prep.
- `exports`: output versioning and quality-check orchestration.

Typed commands to introduce as Tauri `#[tauri::command]` handlers (stubbed where DSP is not ready, returning typed errors):

- `analyze_tracks`
- `render_track_preview`
- `render_track_master`
- `render_album_master`
- `prepare_waveform`
- `run_export_checks`
- `save_project`
- `autosave_session`
- `load_recent_session`
- `save_user_preset`
- `list_user_presets`
- `open_output`

Rules:

- Define shared types in a `types` module and surface them to the frontend via TS bindings. Prefer `specta` (or equivalent) for codegen; hand-written bindings acceptable for the first slice.
- The frontend must never construct CLI argument arrays. Only typed command calls.
- Stubs must return realistic shapes so frontend phases can proceed without DSP being ready.
- Path safety: refuse to write into directories the user did not authorize; reject `..` traversal in any user-supplied path.

Verification:

- Each command is callable from the frontend via `@tauri-apps/api`.
- Schema/contract tests in `src-tauri/tests/` cover at least `analyze_tracks` and `prepare_waveform` response shapes.
- `npm run tauri dev` still launches; `cargo test` passes.

## Phase 2: Track Master Frontend Skeleton

**Status: DONE.** Track Master shell in `src/App.tsx` + `src/components/` (sidebar, waveform, transport, A/B toggle, VM toggle, preset tiles, intensity, 3-band EQ, advanced panel, export). State hook in `src/hooks/useTrackMaster.ts`. See `docs/progress.md` Phase 2 entry.

Goal: build the reference-style Track Master workstation as a UI shell, fed by stub backend data.

Required screen structure:

- Left rail of imported songs.
- Main waveform/audition area (placeholder canvas at this phase).
- Play/pause/seek controls.
- Loop control.
- Original/Mastered toggle.
- Optional Volume Match toggle, off by default.
- Preset tile row (Universal, Clarity, Tape, Spatial, Oomph, Warmth, Punch, Loud).
- Intensity macro.
- Low/Mid/High EQ.
- Preview-stale indicator slot.
- Export Master button.
- Advanced section, collapsed by default.

State behavior at this phase:

- Drag/drop adds tracks via `files.import` typed command.
- Analyze button calls `analyze_tracks` stub and renders dummy values.
- Export button is disabled until analyze has run.
- Changing any control marks the mastered preview stale.

No-victory-lap check:

- A pretty screen is not enough.
- Preset tiles must connect to typed state, even if the audio chain is not yet wired.
- The stale-preview indicator must change when controls change.
- The UI must not appear to play a master it cannot actually produce.

## Phase 3: Source Playback, Waveform, A/B Foundation

**Status: DONE.** Real playback + waveform + same-playhead A/B in `src-tauri/src/audio.rs`. Three-tier PCM resolution (decoded_cache → SharedDecodedCache prewarm → fresh decode); `prewarm_decode` fired fire-and-forget on selectTrack / loadRecentSession / importTracks / openProjectFromDisk; stale-prewarm-evicts-newer guard via `prewarm_target` check.

Goal: get real source audio playing and rendering as a waveform inside the Tauri app.

Required:

- Decode imported audio via `symphonia` (WAV, AIFF, FLAC, MP3, M4A/AAC, OGG, Opus).
- Render a downsampled waveform overview (multi-resolution peak cache) for the imported track.
- Cache decoded PCM and peak data per source in the Tauri app data directory.
- Source playback through `cpal` on a Rust audio thread; transport state surfaced to the frontend.
- Waveform zoom.
- Region selection by dragging.
- Region loop.
- Seek-on-click.
- Original/Mastered toggle as a state flag (initially toggles between source and a placeholder identity-master copy; real mastered playback arrives in Phase 4/5).
- Volume Match optional/off by default.
- Playhead preservation across A/B and Volume Match toggles.

Native audio requirement:

- Real-time playback uses the Rust audio thread, not browser audio.
- Web Audio is acceptable only for scaffolding visualization, not as the playback path.

Verification:

- A real WAV imports, renders a waveform, and plays without dropouts on the dev machine.
- Lossy formats (MP3, M4A, OGG, Opus, AAC) decode and play.
- Region loop holds the selected region cleanly without clicks.
- Source file is never modified.
- `cargo test` covers waveform peak generation and decode-error handling.

## Phase 4: Offline Universal Mastering Chain + Stale Preview

**Status: DONE.** DSP chain implemented in `src-tauri/src/dsp.rs`; preset coefficients + intensity macro; VM cap math bounds chain-push; stale-preview indicator wired in the Track Master shell.

Goal: produce a first credible mastered preview using an offline-rendered output, with safe-by-default behavior. This is temporary scaffolding for Phase 5's real-time engine; quality has to be honest, not toy-grade.

Initial chain (subject to DSP audit in Phase 11):

- Input gain.
- Low/Mid/High EQ; choose filter type (minimum-phase vs. linear-phase) per research docs.
- Optional soft saturation/glue (Tape preset only at this phase).
- Compression/density stage (Universal default: very light).
- Brick-wall limiter with true-peak detection and oversampling; document any simplification.
- Output ceiling.

Universal-first behavior:

- Analyze computes LUFS, true-peak, dynamic range, basic spectral balance, transient density.
- Safe Universal settings derive from analysis, not from genre guesses.
- Preset tiles can produce different chain settings, but Universal remains the confident default.
- "Apply to all" is supported but not required.

Stale-preview behavior:

- Any control change marks the preview stale.
- The UI visibly indicates stale state.
- A new offline render is triggered (manual button or auto-debounced); the master is audible only once the render completes.

Verification:

- Universal preset improves or at least does not obviously harm a `clean-full-mix` fixture, judged by ear and by post-render metering.
- Preset changes produce expected audible/measurable direction (e.g. Tape adds warmth; Punch raises transient impact).
- LUFS, true-peak, and dynamic range numbers reported after a render match an independent measurement on a known reference tone.

## Phase 5: Real-Time Audition Engine

**Status: DONE.** Live preview + same-playhead A/B + 4-layer perf defense: (1) 8 s preview window in `export_landing_gain_lin_for_preview`, (2) VM cap in `dsp.rs::from_settings`, (3) coalescer + playback barriers in `audio.rs::coalesced_command_sequence`, (4) `PreviewLandingCache` settings-hash-keyed landing-gain cache. ADR 0001's JUCE/native fallback was not triggered; Rust met the latency targets.

Goal: prove the app can support responsive controls by ear. Mandatory for release-candidate Track Master.

Targets:

- Gain, light EQ, width, and Volume Match changes audible in under ~150 ms.
- Heavier macro changes audible in under ~500 ms.
- No clicks, zipper noise, glitches, or unstable playback.
- Preview and export must match in audible intent.

Approach:

- Primary: the Rust audio thread runs a real-time chain that mirrors the offline DSP.
- Parameter smoothing for all audible controls.
- Block-based DSP with low-latency buffer sizes appropriate for the host.
- Hot-swap of preset/parameter state without restarting the audio thread.

Controls to prove first:

- Gain.
- 3-band EQ.
- Width.
- Volume Match.
- Basic Intensity subset.

Then extend to:

- Full Intensity macro.
- Preset parity (real-time matches offline export).
- Advanced controls.

Deliverable:

- `docs/adr/0002-realtime-audition.md` capturing:
  - Latency measurements.
  - CPU/memory observations.
  - Fidelity/export-parity risks.
  - Packaging implications.
  - Decision: stay with Rust real-time, add a JUCE/native lane, or supplement with a Python offline R&D lane.
  - What is temporary vs final.

No-victory-lap check:

- Basic real-time controls are a milestone, not the finish line.
- Do not stop after one slider works.
- Non-real-time preview is temporary scaffolding, not final quality.

## Phase 6: Track Master Export And Quality Checks

**Status: DONE.** WAV export with 16/24-bit symmetric-range integer quantization; ceiling-bounded LUFS landing on track + preview + album paths; post-render quality checks (TP / LUFS / DR / bit-depth / non-finite guards) in `src-tauri/src/exports.rs`.

Goal: make export safe, obvious, and honest.

Required:

- One Export Master button.
- Output folder: timestamped or versioned, never overwriting prior renders by default.
- Default delivery: 24-bit WAV; 16-bit dithered WAV available; sample-rate-converted variants optional.
- Post-render checks run automatically.
- Advisory warnings; Export Anyway allowed when the issue is non-fatal.
- Compact receipt or report with output paths.
- Open output action.

Quality checks:

- True-peak / ceiling risk.
- Clipping risk.
- Extremely loud/flat warning.
- Codec preview risk when enabled.
- Non-finite-value guards on analysis.
- Source/master sanity comparisons (energy delta, LUFS delta, spectral delta sanity).

Quality language:

- Plain language.
- No scare warnings for MP3/lossy source format alone.
- Warnings based on measured problems.

Verification:

- Export never modifies the source file.
- Export never overwrites a prior render by default.
- Risky settings produce advisory checks.
- Normal settings can pass quietly.

## Phase 7: Presets, Custom Settings, Autosave, Undo

**Status: DONE.** `save_user_preset` / `list_user_presets` commands; project autosave + explicit Save Project; undo/redo with Ctrl+Z / Ctrl+Shift+Z; `src/lib/history-stack.ts` + Vitest coverage (14 cases). B7 auto-flip-to-Custom on advanced edits; LoudnessTarget readout reflects effective target.

Goal: make experimentation safe and reusable.

Required:

- Custom user presets / settings chains.
- Shared presets across Track Master and Album Master, with mode-specific fields.
- Autosave session state (Tauri app data dir, JSON).
- Explicit Save Project action.
- Undo/redo for non-destructive state.

Undo/redo coverage:

- Presets.
- Intensity.
- EQ.
- Advanced tuning.
- Track order.
- Album roles.
- Transition settings.
- Metadata.

Shortcuts:

- Ctrl+Z undo.
- Ctrl+Shift+Z redo.

No-victory-lap check:

- Rendered files do not need undo.
- Source files are never changed.
- Autosave must not corrupt explicit project files.

## Phase 8: Album Master Mode

**Status: PARTIAL.** `src-tauri/src/album.rs` has album planning + character bias; per-track adaptation logic in place; album-simple + album-plan render paths share the ceiling-bounded landing helper. Track Master and Album Master destination pickers shipped 2026-05-18/19. Album UX (track reordering, the album dashboard/report, per-track override surface) is still thin in the frontend.

Goal: build the album workflow on the Track Master foundation.

Required:

- Album Master mode toggle.
- Track reorder.
- Analyze sequence as a whole, in addition to per-track analysis.
- Global album intent (master settings shared across the album).
- Per-track adaptation (each track can deviate from the album intent where needed).
- Export Album action.
- Individual masters.
- Continuous album WAV by default.
- Preserve original boundaries by default.
- Album dashboard / report.
- Album-level quality checks.

No-victory-lap check:

- Album Master is not batch Track Master with a different button.
- It must show sequence/story awareness.
- It must preserve distinct track identities.

## Phase 9: Album Story / Roles Step

**Status: NOT STARTED.** Listed in PRODUCT.md "Still Open" as one of the genuinely undecided product questions (album story / roles UX).

Goal: give the user a humble, reviewable view of inferred track roles.

Required:

- After analysis, present inferred roles (e.g. opener, single, interlude, ballad, closer) and rough character (e.g. dense, sparse, bright, dark).
- Roles use humble language: "likely role" or "appears to be", never "detected".
- Confidence labels: Strong, Moderate, Unsure. If confidence is low, stay with Universal album intent.
- User can accept all defaults and export without editing.
- User can edit any role/character per track.
- Roles influence per-track adaptation parameters where appropriate, with audible mappings documented.

No-victory-lap check:

- The step must be skippable but visibly reviewable.
- Edits must persist through undo/redo and autosave.

## Phase 10: Transition Primitives

**Status: NOT STARTED.** Scope is now locked by PRODUCT.md #42 to the primitive set only (preserve / direct / timed gap / equal-power crossfade / fade / ring-out). AI-generated transitions are explicitly out of scope. Any earlier task here that referenced generated interludes is superseded.

Goal: provide reliable album boundary tools. The primitive set below is the full scope of this phase per PRODUCT.md #42; generated interludes are not on the roadmap.

Default:

- Preserve source boundaries.
- No transition applied unless the user opts in.

Primitives to implement, in order of priority:

- Direct boundaries.
- Timed gaps.
- Equal-power crossfades.
- Fade out / fade in.
- Ring-out.
- Reverse swell only if it sounds genuinely useful.

## Phase 11: DSP Audit And Modernization

**Status: ONGOING.** Phase A4 mechanical-correctness fixes (B1–B7 audit queue + four perf concerns) ship as part of this stream. Preset subsonic HPF infrastructure and preset transient shaper infrastructure shipped 2026-05-18 with mechanical gates; per-preset cutoff/strength tuning remains in `docs/followups/listening-batch-2026-05-19.md`. PRODUCT.md's 10 DSP Correctness Commitments (BS.1770-5 LUFS, ≥4× oversampled true-peak, ISP awareness, lookahead+oversampled limiter, TPDF dither once at final reduction, polyphase SRC, canonical chain order, mastering-appropriate filters, corpus-grounded preset calibration, spectral-subtraction reference matching) are the audit checklist for this phase.

Goal: improve actual mastering quality, not just UI.

Audit topics:

- BS.1770-compliant LUFS measurement.
- True-peak detection and oversampled limiter behavior.
- Limiter design (lookahead, release, transient handling).
- EQ/filter phase behavior (minimum-phase vs. linear-phase trade-offs).
- Compression behavior (program-dependent release, knee, density).
- Saturation models.
- Stereo processing (M/S width, mono compatibility).
- Dither (TPDF, noise-shaped, configurable).
- Sample-rate conversion (high-quality SRC for delivery profiles).
- Codec preview (AAC, MP3, Opus simulation).
- Preset numeric mappings.

Use the research files in `docs/research/`:

- `audio-mastering-technical-research.md`
- `deep-research-report.md`
- `mastering-settings-reference.md`
- `compass-artifact-e83b62aa.md`

Modernization rule:

- Rewrite/migrate DSP when evidence shows better sound, speed, reliability, real-time behavior, or maintainability.
- Do not rewrite only because native code seems prestigious.
- Consider Rust-native crates (e.g. `fundsp`, `nih-plug`-derived primitives, custom biquads) before pulling C++ via FFI. C++/JUCE is acceptable if it materially improves a Phase 5 latency target.

## Phase 12: Private Real-Audio Fixture Loop

**Status: PARTIAL.** Fast/slow test lanes are wired in `CLAUDE.md` (default fast lane skips real-fixture tests; `AMS_RUN_REAL_FIXTURE=1` opts in to the slow lane). `private-audio-fixtures/` is in place locally. Broader fixture coverage and a documented per-fixture acceptance check still TBD.

Goal: test on music that matters.

Use the convention in `docs/PRIVATE_AUDIO_FIXTURES.md`:

- `private-audio-fixtures/` is ignored by git.
- `private-audio-fixtures/manifest.json` describes purpose, quick vs. slow test suitability, listening notes, known problem areas.

Rules:

- Automated local tests may use fixtures.
- Manual listening remains required for quality calls.
- Never commit private/copyrighted audio, rendered masters from private audio, waveform images derived from private audio, or fixture-specific generated artifacts.

Listening loop per session:

1. Pick a fixture from the manifest.
2. Run the current pipeline (Analyze → Universal → Preview → A/B → Export).
3. Note ear impressions.
4. Note measured deltas (LUFS, TP, DR, spectral balance).
5. Note bugs or weak points.
6. Feed observations into the next phase's adjustments.

## Phase 13: Performance Budgets

**Status: PARTIAL.** The 4-layer perf defense (Phase 5 status) closed the audible cliffs (audio-seek reply timeout + VM over-attenuation). Explicit numeric budgets (target ms per chain update, max decode time per minute of audio, etc.) and an automated regression watch are still TBD.

Goal: measure rather than guess.

Initial rough targets (refine with evidence on the dev machine):

- App launch: prompt; first window paint under ~2 s on warm boot.
- Import: does not block UI; visible progress for files over a few seconds to decode.
- Analyze: visible progress; cancellable.
- Waveform: first overview render under ~1 s for a 4-minute track on the dev machine.
- Lightweight real-time controls: under ~150 ms perceived latency.
- Heavier macro controls: under ~500 ms perceived latency.
- Preview/export: progress visible; cancellable when safe.
- 8-track album export: meaningful progress reporting.

Establish baselines, then refine budgets per phase.

## Phase 14: Release And Installer Hardening

**Status: PARTIAL.** Mac packaging shipped 2026-05-18 (`build:mac`, `.app` + DMG, ad-hoc signed). Windows packaging script shipped 2026-05-19 (`build:windows`, MSI + NSIS setup EXE targets) with a `bundle.windows` config block. Windows installer execution + Authenticode signing are deferred to `docs/followups/infrastructure-2026-05-19.md`. Apple Developer notarization is deferred to the same follow-up doc.

Goal: make the app usable outside the repo.

Required:

- Installed/release build launches.
- No user-managed Python required (audio engine is in-process Rust by default).
- FFmpeg/FFprobe or equivalent only if/when needed; bundle or detect.
- Sensible default render folder.
- Open output / report works.
- App handles missing/corrupt files gracefully.
- Startup overhead measured and documented.

Build command targets:

```powershell
# PowerShell / Windows
npm run build:windows
```

```bash
# Bash / macOS
npm run build:mac
```

The release build should produce platform installable artifacts: Windows MSI/NSIS setup EXE and Mac DMG. Signing/notarization are distribution decisions, not blockers for local private testing.

## Public Release Risk Notes

The app is private for now. Do not slow private development with public-product anxiety.

If it ever ships publicly, revisit:

- Branding.
- UX similarity to commercial products.
- Copied assets or icons.
- Claims about mastering quality.
- Metering/certification claims.
- Third-party library licenses.
- Codec/tool redistribution rights.
- Handling copyrighted audio fixtures.

## Agent Completion Rules

Every meaningful implementation pass must end with:

- What changed.
- What was verified.
- What failed.
- What remains partial.
- What should happen next.
- Whether `docs/PRODUCT.md` still matches the work.

Update locations:

- `docs/progress.md`: detailed evidence and session notes per `docs/CLAUDE_WORK_LOOP.md`.
- `docs/IMPLEMENTATION_PLAN.md`: concise phase status changes or plan changes.
- `docs/PRODUCT.md`: only after human-approved product canon changes.
- `docs/PRIVATE_AUDIO_FIXTURES.md`: fixture convention if it evolves.
- `docs/PARALLEL_BUILD_NOTES.md`: independence rules if the cross-repo posture changes.

No phase is complete just because something visually resembles the goal. It must satisfy the relevant product behavior and verification gates.

## Long-Running Claude Sessions

Follow `docs/CLAUDE_WORK_LOOP.md`:

1. Restate the current slice in one paragraph.
2. Identify which product requirement from `docs/PRODUCT.md` it serves.
3. Inspect relevant research or architecture docs before choosing an implementation.
4. Build one vertical slice, not a disconnected demo.
5. Add or update tests/smoke checks where behavior is testable.
6. Run verification.
7. Write a concise progress note in `docs/progress.md`.
8. List what remains partial or unproven.

Good session candidates:

- "Build the Phase 3 source-playback slice and verify on `clean-full-mix` fixture."
- "Implement Phase 1 typed commands `analyze_tracks` and `prepare_waveform` with stub data and contract tests."
- "Run the Phase 5 real-time audition spike and write ADR 0002."
- "Add private fixture manifest support to the Phase 12 listening loop."

Bad session candidates:

- "Make the app good."
- "Finish mastering."
- "Rewrite the engine" without acceptance criteria.
- Anything that requires subjective listening but provides no fixture or evaluation notes.

Use long-running sessions as an execution loop for a clear verified slice, not as a substitute for product planning.

## Immediate Next Questions For Humans

Genuinely open product questions. The earlier questions on this list (Python offline lane, JUCE/native benchmarking, real audio fixture choice, progress update cadence) have been resolved by ADR 0001, by Phase 5 landing on Rust, by the existence of `private-audio-fixtures/`, and by the mechanical-correctness-first workflow agreement respectively.

1. Does Album Master need to ship before any release-candidate claim, or is "Track Master release-candidate with Album Master near-complete" acceptable?

Additional open product questions live in `docs/PRODUCT.md` under "Still Open" (loudness/TP profile defaults, reference matching algorithm specifics, album story/roles UX, codec QC scope, installer/distribution polish, preset numeric mappings).

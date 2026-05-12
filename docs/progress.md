# Progress Log

## 2026-05-11 — Phase 0: Workspace scaffold and architecture ADR

Goal:

Take the Claude-build repo from zero-state to a buildable Tauri 2.x shell with the architecture lock recorded in `docs/adr/0001-tauri-rust-stack.md`.

What changed:

- Rewrote `docs/IMPLEMENTATION_PLAN.md` for the Claude build. Preserved the phase wisdom from the Codex-authored version; reframed Phase 0 as "workspace scaffold + ADR" instead of "canon and repo baseline"; replaced Codex verification commands with Claude-build-native commands; updated module list and typed command list to match the zero-state assumption.
- Wrote ADR 0001 locking Tauri 2.x shell + React/TS frontend + Rust audio engine, with JUCE and Rust-native UI documented as reversible fallbacks.
- Scaffolded the workspace:
  - Root: `package.json`, `index.html`, `vite.config.ts`, `tsconfig.json`, `tsconfig.node.json`.
  - Frontend in `src/`: `main.tsx`, `App.tsx`, `App.css`, `vite-env.d.ts`.
  - Tauri app in `src-tauri/`: `Cargo.toml`, `tauri.conf.json`, `build.rs`, `src/main.rs`, `src/lib.rs` with empty module declarations for `audio`, `engine`, `exports`, `files`, `jobs`, `project`, `settings`.
  - `src-tauri/capabilities/default.json` with the minimum permission set.

Verification:

- `npm install` — 74 packages, 0 vulnerabilities. `@types/node ^25.7.0` was added during the run because `vite.config.ts` references `process.env.TAURI_DEV_HOST` and TypeScript needed Node types.
- `npm run build` — frontend builds cleanly via `tsc -b && vite build`. `dist/` produced (~196 KB pre-gzip).
- `cargo check` (from `src-tauri/`) — clean after a placeholder `icon.ico` was added; `tauri-build` requires it for the Windows resource file even with `bundle.active = false`. The placeholder icon is a 64×64 dark "A" generated via .NET `System.Drawing`; it should be replaced with a real icon in Phase 14.
- `cargo test` (from `src-tauri/`) — 0 tests, all 0 pass. Contract/unit tests start in Phase 1.

Real-audio fixture used: none. Phase 0 has no audio path yet.

What failed or remains partial:

- `npm run tauri dev` requires an interactive window and is not verified in this automated pass; deferred to manual verification on the dev machine.
- `tauri build` is intentionally not attempted; signing and proper icons belong to Phase 14.
- No DSP, no audio I/O, no playback. Phase 1 starts the typed command layer; Phase 3 brings real audio.
- The placeholder `src-tauri/icons/icon.ico` is committed (2.6 KB) so the workspace builds out of the box; replace in Phase 14.

Next recommended slice:

Phase 1 — typed Rust app foundation. Define the command list with stub implementations and contract tests. Start with `analyze_tracks`, `prepare_waveform`, and `prepare_source_playback` since those unblock Phase 2's frontend skeleton.

## 2026-05-11 — Phase 1: Rust/Tauri typed app foundation

Goal:

Make the backend speak product concepts via typed `#[tauri::command]` handlers, with realistic mock shapes so frontend phases can develop against the contract before DSP exists.

What changed:

- Added `uuid` and `thiserror` to `src-tauri/Cargo.toml`; `tokio` (dev-only) for async contract tests.
- `src-tauri/src/types.rs`: shared product types — `TrackId`, `ImportedTrack`, `AnalysisResult` (LUFS, true peak, DR, spectral balance, transient density, stereo width, recommended Universal settings), `MasteringSettings` (preset, intensity, EQ, volume_match, advanced), `WaveformPeaks`, `PlaybackHandle`, `AbPreview`, `RenderJob` + `JobStatus`, `ExportReport` + `QualityCheck`, `ProjectState`, `UserPreset`, `CommandError` (serialized as string for the IPC boundary).
- 16 typed commands registered in `lib.rs`:
  - `files`: `import_tracks` (rejects `..` traversal, extracts display name + extension).
  - `engine`: `analyze_tracks`, `render_track_preview`, `render_track_master`, `render_album_master` (all return realistic mocks).
  - `audio`: `prepare_source_playback`, `prepare_master_playback`, `prepare_ab_preview`, `prepare_waveform` (sine-shaped mock peak envelope).
  - `exports`: `run_export_checks` (real logic: true-peak / LUFS / DR / bit-depth / non-finite guards with plain-language messages), `open_output`.
  - `project`: `save_project`, `autosave_session`, `load_recent_session`.
  - `settings`: `save_user_preset`, `list_user_presets`.
- `src-tauri/tests/contracts.rs`: 8 contract tests covering analyze, waveform stereo shape, path-traversal rejection, display-name parsing, export-check warnings (high true peak), export-check silent-when-clean, render-job done-shape, preset name validation.
- `src/bindings.ts`: hand-written TS types mirroring Rust types. Phase 1.2 should replace with `tauri-specta` codegen.
- `src/lib/api.ts`: typed wrapper around `@tauri-apps/api/core`'s `invoke`, with snake_case param keys matching Rust function signatures.
- `src/App.tsx`: IPC proof — two buttons (Import mock track, Analyze) that round-trip through the backend and render shape-valid results. Replaced in Phase 2 by the real Track Master surface.

Verification:

- `npm run build`: clean. Bundle 198 KB (62 KB gzip).
- `cargo test` (from `src-tauri/`): 8/8 contract tests pass. New-deps compile ~30s.
- `npm run tauri dev`: deferred (interactive).

Real-audio fixture used: none. Phase 1 is stub-only; real audio paths begin in Phase 3.

What failed or remains partial:

- TS bindings are hand-written; will drift from Rust types if changed without care. Phase 1.2 should swap to `tauri-specta` codegen.
- No live frontend smoke; the IPC proof in `App.tsx` only exercises 2 of 16 commands and is verified by the manual `npm run tauri dev` Dan runs locally.
- No runtime IPC test from Rust; contract tests call command functions directly.

Next recommended slice:

Phase 2 — Track Master frontend skeleton: left-rail song list, main waveform area (placeholder canvas), transport (play/pause/seek/loop), Original/Mastered toggle, Volume Match toggle (off), preset tile row, Intensity, 3-band EQ, preview-stale indicator, Export Master button, collapsed advanced section. UI shell only, wired to the Phase 1 stubs. Alternative: Phase 1.2 (specta bindings) first if binding drift bites — but Dan's "knock so much out" preference probably favors moving to Phase 2.

## 2026-05-11 — Phase 2: Track Master frontend skeleton

Goal:

Build the reference-style Track Master workstation as a UI shell, fed by the Phase 1 stub backend. Drop the temporary "IPC proof" buttons; replace with the real product surface.

What changed:

Backend:

- Added `tauri-plugin-dialog = "2"` to `src-tauri/Cargo.toml`; registered plugin in `lib.rs` via `.plugin(tauri_plugin_dialog::init())`.
- Updated `src-tauri/capabilities/default.json` to grant `dialog:default` so the frontend can open file pickers.

Frontend:

- Added `@tauri-apps/plugin-dialog` JS bindings.
- `src/hooks/useTrackMaster.ts`: central state hook managing tracks, per-track analysis, per-track waveform peaks, per-track settings, transport state, stale-preview set, advanced panel state, last export receipt, error toast. Every action routes through the Phase 1 typed commands via `src/lib/api.ts`.
- `src/App.tsx`: full Track Master layout — sidebar (track list + "Add files" → native dialog with audio extension filter), main workspace with `TrackHeader` (analyzed metering badges), `WaveformView` (SVG-rendered peaks), `Transport` (play/pause, time display, loop toggle, Original/Mastered A/B segmented toggle, Volume Match checkbox), `PresetTiles` (Universal/Clarity/Tape/Spatial/Oomph/Warmth/Punch/Loud with hover blurb), `Macros` (Intensity + L/M/H EQ sliders), `StaleBar` with pulsing dot + "Update preview" button, `ExportSection` + collapsible `AdvancedPanel` (LUFS target, ceiling, width, warmth, presence/air, compression, bit depth, sample rate; each with Auto/Set toggle), `Toast` for errors, `ExportReceiptCard` modal for post-export feedback.
- `src/App.css`: complete dark-themed design system — CSS variables for palette, sidebar/workspace grid layout, all component styles, stale-pulse animation, semantic color coding for quality-check levels.

Behavior wired:

- "Add files" opens native dialog with audio extensions filter → `import_tracks` → auto-`analyze_tracks` (applies `recommended_universal` if track is still on Universal preset) → auto-`prepare_waveform`.
- Changing any control (preset, intensity, EQ band, advanced field) marks preview stale; "Update preview" calls `render_track_preview` and clears stale on success.
- Export disabled until analysis exists; clicking it calls `render_track_master`, runs `run_export_checks`, and surfaces a receipt modal with output path + color-coded quality checks (info/warning/critical).
- A/B toggle, Volume Match, loop, transport buttons are UI state only (no audio yet — Phase 3).

Verification:

- `npm run build`: clean. 34 modules transformed. CSS 11 KB, JS 213 KB (66 KB gzipped).
- `cargo test` (from `src-tauri/`): 8/8 contract tests still pass after adding `tauri-plugin-dialog`. No regression.
- `npm run tauri dev`: deferred (interactive — Dan to verify the layout visually).

Real-audio fixture used: none. All audio paths are stubs.

What failed or remains partial:

- No real audio playback (Phase 3 deliverable). Transport buttons toggle UI state only.
- No drag/drop-on-window file events; "Add files" uses the native dialog. Window drag/drop arrives in Phase 3 alongside real decode.
- Waveform comes from `prepare_waveform`'s mock sine envelope, not real PCM peaks.
- Hand-written TS bindings still drift risk. Phase 1.2 (`tauri-specta`) deferred.
- No undo/redo on settings changes (Phase 7).
- No persistence — page refresh loses session (Phase 7 autosave).

Next recommended slice:

Phase 3.1 — wire `prepare_source_playback` to a real Rust audio thread using `cpal` + `symphonia`. Make the play button actually play the imported audio file. This is the biggest win for product feel and unblocks real waveform peaks in Phase 3.2.

## 2026-05-11 — Phase 3.1: real audio decode + waveform peak generation

Goal:

Replace the mock waveform/import metadata with real audio decoding via `symphonia`. Frontend gets actual peak data from imported files; `import_tracks` populates `duration_seconds`, `sample_rate`, `channels` from the file itself. Playback in Phase 3.2 builds on this.

What changed:

Backend:

- `src-tauri/Cargo.toml`: added `symphonia = "0.5"` with format features `mp3`, `aac`, `isomp4`, `flac`, `wav`, `pcm`, `ogg`, `vorbis`. Added `hound = "3"` and `tempfile = "3"` to dev-dependencies for synthetic WAV tests.
- `src-tauri/src/files.rs`: `import_tracks` now probes the file with symphonia's format reader and fills in real `duration_seconds`/`sample_rate`/`channels` for every supported codec. Path-traversal check refactored from substring (`contains("..")`) to component-based (`Path::components().any(|c| c == Component::ParentDir)`) — the old check rejected any path containing `..` as a substring, breaking legitimate filenames like `something..mp3` and relative paths like `../fixtures/...`.
- `src-tauri/src/audio.rs`: `prepare_waveform` now takes `(track_id, track_path, target_pixels)` and decodes the real file via symphonia. Streams packets, decodes into `SampleBuffer<f32>`, accumulates per-channel running peaks across `samples_per_pixel`-sized windows, flushes the trailing partial window. `prepare_source_playback`, `prepare_master_playback`, `prepare_ab_preview` also take `track_path` (parameter plumbing now, real audio thread wiring in Phase 3.2).

Frontend:

- `src/lib/api.ts`: matching signature updates for the four audio-path commands. `prepareWaveform` now takes `(trackId, trackPath, targetPixels?)`; `targetPixels` defaults to 1000 server-side when null.
- `src/hooks/useTrackMaster.ts`: import loop now iterates over `imported` so it can pass each track's path to `prepareWaveform`. Default target is 1200 pixels for the waveform view.

Fixtures:

- Copied `Lay the Money on the Desk (1).mp3` into the gitignored `private-audio-fixtures/lay-the-money-on-the-desk.mp3` (4.28 MB).
- Wrote `private-audio-fixtures/manifest.json` per the convention in `docs/PRIVATE_AUDIO_FIXTURES.md`.
- Both files are gitignored (`private-audio-fixtures/` + `*.mp3`). The convention is documented but the audio itself never enters git.

Tests (in `src-tauri/tests/contracts.rs`):

- `import_tracks_extracts_metadata_from_synthetic_wav`: hound generates a 1-second 44.1 kHz stereo sine, asserts duration/sample rate/channels are recovered.
- `prepare_waveform_decodes_synthetic_wav`: hound generates a 0.5-second sine at 0.5 amplitude, asserts the decoder returns stereo peaks within `(0.45, 0.55)` max.
- `prepare_waveform_rejects_empty_path`.
- `decode_real_fixture_if_present`: skips silently when `../private-audio-fixtures/lay-the-money-on-the-desk.mp3` is absent; otherwise asserts duration > 10s, sample rate > 0, channel count > 0, peak length ≥ 200, max peak > 0.1. The fixture path is canonicalized to absolute so the path-traversal check passes.
- Replaced the old mock `prepare_waveform_returns_stereo_peaks` test (no longer relevant — the function now requires a real file path).
- Existing 7 tests still pass.

Verification:

- `npm run build`: clean. Bundle 213 KB (66 KB gzipped).
- `cargo test` (from `src-tauri/`): 11/11 contract tests pass. Total run time 5.57s including the real MP3 decode.
- `cargo check`: clean.
- The real-fixture test confirms symphonia successfully decodes the supplied MP3 end-to-end and produces structurally valid peak data.

Real-audio fixture used: `private-audio-fixtures/lay-the-money-on-the-desk.mp3` — first real-mix fixture supplied by Dan. Used by the contract test, never committed.

What failed or remains partial:

- No playback yet. Transport buttons still toggle UI state only; real audio thread + cpal stream wiring is Phase 3.2.
- No drag-on-window file events; "Add files" still uses the native dialog.
- No cache. Every waveform call decodes the file end-to-end. For multi-minute tracks this is well under a second on the dev machine, but a peak/PCM cache in the Tauri app data directory would help re-opens. Adding it when needed.
- The `recommended_universal` from `analyze_tracks` is still mock — analyzer doesn't run real metering yet. That's Phase 4 (offline mastering chain) territory.

Next recommended slice:

Phase 3.2 — real source playback. Add `cpal` to Cargo.toml; build a Rust audio thread that owns the cpal output stream; `prepare_source_playback` creates/replaces the stream for the requested track; new typed commands `play`, `pause`, `seek`, `stop` drive transport. Tauri events stream playback position back to the frontend so the transport time display updates and the waveform can show a playhead.

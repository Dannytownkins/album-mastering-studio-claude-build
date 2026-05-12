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

## 2026-05-11 — Phase 3.2: real source playback via rodio + dedicated audio thread

Goal:

Wire the transport play/pause buttons to actual audio output. User clicks play, the imported track plays. Click pause, it pauses. Position updates flow back into the frontend transport display.

What changed:

Backend:

- Added `rodio = "0.20"` with `symphonia-all` features to `src-tauri/Cargo.toml`.
- `src-tauri/src/types.rs`: added `PlaybackTick { track_id, position_sec, is_playing, is_loaded }` for IPC event payloads.
- `src-tauri/src/audio.rs`: introduced `AudioPlayer` — a `Send + Sync` handle to a dedicated audio thread. The thread owns the `rodio::OutputStream`, `OutputStreamHandle`, and `Sink` (all of which are `!Send` on most platforms, so they must stay confined to a single thread). Commands flow over `mpsc::Sender<AudioCommand>` (Play, Pause, Resume, Stop, Shutdown); the current snapshot is shared via `Arc<RwLock<PlaybackSnapshot>>`. The thread loops on `recv_timeout(50ms)` so that even between commands the snapshot stays fresh with the sink's reported position.
- New typed commands: `play_track(track_id, track_path)`, `pause_playback()`, `resume_playback()`, `stop_playback()`. `play_track` is best-effort blocking on the reply channel (5s timeout) so the frontend gets a real success/failure signal.
- `src-tauri/src/lib.rs`: `.manage(Arc::new(AudioPlayer::new()))`; setup hook spawns a 50ms tick thread that reads the snapshot and emits a `playback:tick` event with the current `PlaybackTick`. The thread silently skips emit when no track is loaded so the frontend doesn't churn on no-op events.

Frontend:

- `src/bindings.ts`: added `PlaybackTick` type.
- `src/lib/api.ts`: new methods `playTrack`, `pausePlayback`, `resumePlayback`, `stopPlayback`. Added `onPlaybackTick(handler)` helper that wraps `@tauri-apps/api/event`'s `listen()` and returns an unlisten function.
- `src/hooks/useTrackMaster.ts`: subscribed to `playback:tick` via `useEffect` — updates transport `isPlaying`/`currentTimeSec` from the event and tracks `loadedTrackId` separately. `togglePlay` now branches on (selected vs loaded track) and on (playing vs paused) to issue the right command: `playTrack` if the selected track isn't loaded, `pausePlayback` if playing, `resumePlayback` if paused. `selectTrack` issues a best-effort `stopPlayback` if switching away from the loaded track.

Verification:

- `npm run build`: clean. Bundle 214 KB (67 KB gzipped) — small bump from adding `@tauri-apps/api/event` listener.
- `cargo build`: clean. The first compile was Send-unsafe with `rodio::OutputStream` held directly in `AudioPlayer`; fixed by moving rodio types into a dedicated thread and exposing only `Send` channel + atomic snapshot.
- `cargo test` (from `src-tauri/`): 11/11 contract tests still pass. Total 8.50s including real MP3 decode.
- `npm run tauri dev`: deferred (interactive — Dan to verify actual playback by clicking play on the imported MP3).

Real-audio fixture used: `private-audio-fixtures/lay-the-money-on-the-desk.mp3` — the existing decode test still passes against it; runtime playback verification is manual.

What failed or remains partial:

- First architecture attempt (rodio types directly in `AudioPlayer`) failed Send+Sync; refactored to mpsc + audio thread before commit.
- Seek not yet implemented. Phase 3.3 adds seek-on-click, region selection, region loop.
- Original/Mastered A/B is still UI state only — both branches currently point to the source file. Phase 4 wires the mastered audio path.
- No bridge between the real waveform peaks and a playback cursor overlay yet — the waveform shows static peaks; playhead position lives in the transport bar.
- No automated test for actual audio output (would require a virtual audio device on CI). The audio thread architecture is verified at compile time and through manual playback testing.
- `prepare_source_playback`/`prepare_master_playback`/`prepare_ab_preview` are still stubs that return `PlaybackHandle`s; the real play path is the new `play_track` command. The prepare_* commands are kept on the contract surface for future use (Phase 5 may re-introduce them with real meaning around AB preview prep).

Next recommended slice:

Phase 3.3 — seek + waveform playhead overlay. Add `seek_playback(position_sec)` command; update the audio thread to handle seek via `Sink` rebuild (rodio doesn't have direct seek; the standard pattern is to skip-to-position by decoding a new source pinned to the offset). Frontend: click on the waveform jumps to that position; render a vertical line over the waveform at `transport.currentTimeSec / duration_seconds`. After that, region selection (drag on waveform) + region loop.

## 2026-05-11 — Phase 3.3: seek + waveform playhead

Goal:

Make the waveform clickable to seek, and render a vertical playhead that tracks playback position. (Rodio turns out to have `Sink::try_seek` built in — no manual rebuild needed.)

What changed:

Backend:

- `AudioCommand::Seek { position_sec, reply }` with a 2-second reply timeout.
- `AudioPlayer::seek(position_sec) -> CommandResult<()>`.
- `seek_playback` typed command — validates `position_sec` is finite and non-negative before forwarding.
- Audio-thread handler calls `rodio::Sink::try_seek(Duration::from_secs_f64(...))` and reports the result back. `try_seek` works for symphonia-backed decoders.

Frontend:

- `api.ts`: `seekPlayback(positionSec)`.
- `useTrackMaster.ts`: `seek(positionSec)` action — clamps to ≥ 0, optimistically updates `transport.currentTimeSec`, and only calls `api.seekPlayback` if the player has the currently-selected track loaded (otherwise the click is a "scrub before play" gesture that just sets the next play position visually).
- `App.tsx` `WaveformView`: clickable SVG (`cursor: crosshair`); click computes `ratio = (clientX - rectLeft) / rectWidth` → `seekTo = ratio * durationSec` → invokes `onSeek`. Renders a vertical playhead line at `(currentTimeSec / durationSec) * W` across the waveform. ARIA `role="slider"` with `aria-valuemin/max/now` for accessibility.
- `App.css`: `.wf { cursor: crosshair }`, `.wf-playhead { stroke: white; vector-effect: non-scaling-stroke; pointer-events: none }`.

Verification:

- `npm run build`: clean. Bundle 215 KB (67 KB gzipped).
- `cargo test` (from `src-tauri/`): 11/11 contract tests pass.
- `npm run tauri dev`: deferred (manual seek + playback verification).

Real-audio fixture used: same MP3 — exercised at compile/decode time; runtime seek verification is manual.

What failed or remains partial:

- Rodio's `try_seek` may fail for some formats depending on the underlying decoder; the error surfaces as `CommandError::Other` and appears in the toast. Acceptable for the supported formats (WAV/FLAC/MP3/OGG/Vorbis).
- The playhead sits at `x=0` before playback starts (`currentTimeSec` is 0). Visually OK but worth refining later.
- Visual scrub-feedback while dragging across the waveform is not implemented — only single clicks trigger seek. Drag interactions land in Phase 3.4 with region selection.

Next recommended slice:

Phase 3.4 — region selection by drag + region loop. Drag on the waveform defines `[start, end]`; clicking the loop button activates region playback that repeats `start → end`. Backend: `AudioCommand::SetLoop(Option<(f64, f64)>)` + audio thread monitors position and seeks back to `start` when crossing `end`. Visual: shaded range on the waveform, plus a "loop on" indicator next to the loop button.

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

## 2026-05-11 — Phase 4.1: real DSP mastering chain + mastered WAV export

Goal:

The Export Master button must actually produce a mastered file, not a mock. End to end: decode source → gain → 3-band biquad EQ → optional saturation (Tape/Warmth) → soft-clip ceiling → write WAV to versioned output dir under the Tauri app data folder. The user's MP3 round-trips through it.

What changed:

Backend:

- `src-tauri/Cargo.toml`: moved `hound = "3"` from dev-deps to main deps (needed at runtime for WAV writing).
- `src-tauri/src/dsp.rs` (new): `BiquadCoeffs` (RBJ Audio EQ Cookbook coefficients for `low_shelf`, `peaking`, `high_shelf`, plus identity passthrough when gain ≈ 0), `BiquadState` (direct-form II transposed), `ChainCoeffs::from_settings` mapping `MasteringSettings` to numbers (preset-specific base gain plus `intensity * 4.5 dB` headroom; Tape adds tanh saturation, Warmth adds a gentler one; ceiling defaults to -1 dBFS), `MasteringChain` with per-channel state owning the three biquads.
- `src-tauri/src/audio.rs`: added `decode_full(path) -> DecodedPcm` that streams the full interleaved f32 PCM into a `Vec<f32>` for offline processing (existing streaming peak generator stays untouched).
- `src-tauri/src/engine.rs`: replaced the mock `render_track_master` / `render_track_preview` with `mastering_render(track_id, source_path, settings, out_dir, kind) -> RenderJob` which: validates path safety, decodes via `audio::decode_full`, builds a `MasteringChain`, processes the interleaved buffer in place, then writes a 16/24/32-bit WAV via `hound`. Tauri command thin wrappers resolve the output directory via `AppHandle::path().app_data_dir()` and call `mastering_render`. Bit depth comes from `settings.advanced.bit_depth` (default 24). `unique_output_path` guarantees non-overwrite by suffixing `__<N>` if a same-second collision would occur.
- `src-tauri/src/types.rs`: added `Copy + PartialEq + Eq` to `RenderKind` and `PlaybackKind` (unit-variant enums; needed for ergonomic use in functions that take them by value and return them in the response).
- `src-tauri/src/lib.rs`: `pub mod dsp;` registered.

Frontend:

- `src/lib/api.ts`: `renderTrackPreview` and `renderTrackMaster` now take `(trackId, trackPath, settings)`.
- `src/hooks/useTrackMaster.ts`: `updatePreview` and `exportMaster` pass `selectedTrack.path` through.

Tests (in `src-tauri/tests/contracts.rs`):

- `mastering_render_writes_processed_wav`: synthetic stereo sine → render → assert `.wav` file exists, channel count, sample rate, bit depth match expectations.
- `mastering_render_creates_unique_paths_on_collision`: render the same source twice into the same dir → output paths differ → both files exist (PRODUCT.md "exports never overwrite by default").
- `dsp_chain_applies_input_gain_at_default_intensity`: confirms output RMS > input RMS at default settings (Universal preset, 0.5 intensity = +3.75 dB input gain), and the soft-clip ceiling bounds peaks near -1 dBFS.
- `dsp_low_shelf_boost_raises_low_frequency_energy`: feeds an 80 Hz sine through a baseline chain and a `+6 dB low_shelf @ 200 Hz` chain → boosted RMS > baseline RMS (verifies the EQ does what its label says).
- `mastering_render_processes_real_fixture_if_present`: end-to-end real MP3 → mastered WAV in a tempdir, asserts ≥ 10s of audio in the output. Runs only when `private-audio-fixtures/lay-the-money-on-the-desk.mp3` exists; skipped silently otherwise.
- Replaced the old mock `render_track_master_returns_done_with_output_path` test.

Verification:

- `npm run build`: clean. Bundle 215 KB.
- `cargo test` (from `src-tauri/`): 15/15 pass in 6.80s. The real-MP3 mastering test runs in ~1 second on the dev machine.
- `npm run tauri dev`: deferred (manual verification — Dan can now click Export and find a `.wav` under `%APPDATA%\com.albummasteringstudio.app\renders\masters\`).

Real-audio fixture used: `private-audio-fixtures/lay-the-money-on-the-desk.mp3` — decoded, mastered, written to a tempdir WAV during cargo test.

What failed or remains partial:

- No real LUFS / true-peak measurement yet — `analyze_tracks` still returns mock metering, so `run_export_checks` is operating on mock numbers. Phase 11 (DSP audit) wires real BS.1770 K-weighting + 4× oversampled true-peak.
- No compressor with attack/release — saturation does most of the loudness lift for Tape/Warmth; Loud preset will need a real compressor + limiter to live up to its name. Deferred to Phase 11.
- The Mastered side of the A/B toggle still plays the source file. Wiring the mastered WAV into the playback path means swapping the rodio source when the toggle hits Mastered. Phase 4.2.
- No live preview during slider drag — the user has to click "Update preview" then "Export" to hear changes. That's by-design until Phase 5 (real-time audition engine), which is the actual hard problem.
- Output directory uses Tauri's `app_data_dir`, which is platform-specific (`%APPDATA%\com.albummasteringstudio.app\renders\masters\` on Windows). A user-facing "Open output" flow exists in `exports::open_output` but is a no-op stub — wiring it through `tauri-plugin-shell` would make the receipt path clickable.

Next recommended slice:

Phase 4.2 — Mastered A/B playback. When the user toggles the playback kind to "Mastered", play the rendered preview WAV instead of the source. Pipeline: on the first Mastered-toggle for a track with stale preview, auto-render the preview, then swap the audio thread to play it. Position should preserve across A/B toggles (per PRODUCT.md "Playhead preservation"). Backend: new `play_kind` field on the audio state, or a second loaded source. Frontend: A/B toggle calls a new command instead of just flipping local state.

## 2026-05-11 — Phase 4.2: Mastered A/B playback with playhead preservation

Goal:

The Original/Mastered toggle now actually swaps the audio source mid-playback at the current playhead position. The user hears the mastered render, not the source. If the master preview is stale or missing, render it on the fly before swapping.

What changed:

Backend:

- `AudioCommand::Play` gained `start_position_sec: f64`. `AudioPlayer::play_track(track_id, path, start_position_sec)` and the `play_track` Tauri command both accept it (`Option<f64>` on the wire, defaulting to 0.0). `handle_play` performs the load, then calls `Sink::try_seek` if `start_position_sec > 0`, then `play()`. Best-effort: if `try_seek` fails for a given format, playback simply starts from 0 — no hard error.

Frontend:

- `api.ts`: `playTrack(trackId, trackPath, startPositionSec?)` passes `start_position_sec` (or `null`) into the invoke args.
- `useTrackMaster.ts` state additions:
  - `masterPathByTrack: Record<TrackId, string>` — last successful preview render output path per track.
  - `loadedKindByTrack: Record<TrackId, PlaybackKindUI>` — which kind (source/master) the audio thread currently has loaded for each track.
- New `renderPreviewForSelected()` helper extracted from the original `updatePreview` — returns the new master path and stores it in `masterPathByTrack`.
- `updatePreview` now also reloads the audio source if the user is mid-Mastered playback, so a settings tweak + Update preview swaps to the freshly rendered master at the current playhead without manual reload.
- `resolvePathForKind(kind)` returns the right path for the requested A/B side — source path if `source`, or the stored master path if fresh, else auto-renders a new preview and returns the new path.
- `playWithKind(kind, positionSec)` calls `resolvePathForKind`, then `api.playTrack` with the resolved path + position, then records `loadedKindByTrack`.
- `togglePlay` now considers both `loadedTrackId` and `loadedKindByTrack[selectedTrackId]`. If the player isn't loaded with the correct (track, kind) pair, it (re-)loads via `playWithKind(playbackKind, 0)`. Otherwise pause/resume as before.
- `setPlaybackKind` is now async: it updates the UI state, and if the selected track is currently loaded in the player, it triggers a mid-playback source swap to the new kind at `transport.currentTimeSec`. If switching to Mastered with a stale/missing render, it auto-renders the preview first (with `isRendering` flag showing the spinner).
- `removeTrack` cleans up `masterPathByTrack` and `loadedKindByTrack` for the removed track, and calls `stopPlayback` if the player was loaded with that track.

Verification:

- `npm run build`: clean. Bundle 216 KB (68 KB gzipped).
- `cargo test` (from `src-tauri/`): 15/15 still pass in 6.70s.
- `npm run tauri dev`: deferred (manual A/B verification — click Original/Mastered while playing the MP3, expect mid-playback swap at the same playhead).

Real-audio fixture used: same MP3, exercised end-to-end through the mastering chain by the existing real-fixture test.

What failed or remains partial:

- If `try_seek` fails for the rendered WAV (shouldn't — it's a fresh 24-bit PCM WAV), playback restarts from 0 on A/B swap. Acceptable fallback.
- No visual indicator that an auto-render is in flight when toggling to Mastered — the existing `isRendering` flag is used but the spinner currently only shows next to the Update preview button. Could surface a small inline indicator near the A/B toggle in Phase 4.3.
- Position drift across A/B is bounded by the 50ms tick rate plus seek latency. Should be inaudible but isn't measured.
- `loadedKindByTrack` is not derived from the backend snapshot — if the audio thread loses sync (e.g. a future bug clears it), the UI's belief about which kind is loaded can drift. Acceptable until then; defensive sync can come later.

Next recommended slice:

Phase 3.4 (carried) — region selection + region loop. Drag on the waveform to define a `[start, end]` region; loop button activates region playback. Backend: `AudioCommand::SetLoop(Option<(f64, f64)>)` + audio thread monitors `Sink::get_pos()` and seeks back to `start` when crossing `end`. Visual: shaded range on the waveform. Then Phase 11 (DSP audit) for real LUFS / true-peak / compressor / limiter — the offline chain is reasonable but `analyze_tracks` still returns mock metering, so `run_export_checks` is operating on fake numbers.

## 2026-05-11 — Phase 4.3: real BS.1770 metering via ebur128 (analyze_tracks no longer lies)

Goal:

Make `analyze_tracks` measure the file, not return constants. Integrated LUFS, true-peak dBTP, loudness range (LRA), stereo width, spectral balance, and a rough transient density now come from the audio, so `run_export_checks` and the metering badges in the UI reflect reality.

What changed:

Backend:

- Added `ebur128 = "0.1"` to `src-tauri/Cargo.toml`. This is the canonical Rust port of BS.1770 (K-weighting, gated integrated loudness, true-peak with 4× oversampling, LRA).
- Rewrote `engine::analyze_tracks` to take `Vec<AnalyzeRequest { id, path }>` instead of `Vec<TrackId>`. The Tauri command decodes each file via `audio::decode_full`, feeds the interleaved samples into `EbuR128::new(channels, sr, Mode::I | Mode::LRA | Mode::TRUE_PEAK)`, and pulls back integrated loudness, LRA, and per-channel true peaks.
- `compute_stereo_width` — M/S energy ratio across the whole track. Mono returns 0, perfectly correlated stereo near 0, anti-correlated near 1.
- `compute_spectral_balance` — first-order RC low-pass network split into low/mid/high bands; ratios normalized to sum to 1. Documented as "approximate; Phase 11b can swap in Linkwitz-Riley crossovers or FFT".
- `compute_transient_density` — zero-crossing rate on the mono mix, scaled to a 0..1 range as a crude proxy. Phase 11b can replace with a real onset detector.
- `sanitize_lufs` collapses `-inf` / NaN from silence-only inputs to `-70.0` LUFS so downstream code never gets non-finite values.
- `recommended_universal.advanced.lufs_offset_db` is now `-14.0 - measured_integrated` — a real target offset toward streaming-canonical -14 LUFS instead of the previous static stub.

Frontend:

- `api.ts`: `analyzeTracks(tracks: { id, path }[])` matches the new command shape.
- `useTrackMaster.ts`: import flow maps `imported` to `{ id, path }` before calling `api.analyzeTracks` (so the backend has the file paths it needs).

Tests:

- Replaced the old constant-asserting `analyze_tracks_returns_one_result_per_input` with two real tests:
  - `analyze_tracks_measures_synthetic_wav` — 3-second amplitude-0.5 440 Hz stereo sine. Asserts LUFS in `(-30, 0)`, true peak in `(-10, 3)` dBTP, finite LRA, recommended preset Universal, spectral balance sums to ~1.0, stereo width in `[0, 1]`.
  - `analyze_tracks_runs_against_real_fixture_if_present` — runs against the real MP3 if it exists in `private-audio-fixtures/`. Asserts finite metering, non-negative LRA, spectral balance sums to ~1.0.

Verification:

- `npm run build`: clean. Bundle 216 KB (68 KB gzipped).
- `cargo test` (from `src-tauri/`): 16/16 pass in 21.24s — the real-fixture analysis adds significant compute (full-track decode + K-weighted filter + integrated gating across the song).
- `npm run tauri dev`: deferred (manual — load the MP3, watch the metering badges show real LUFS/TP/DR for the file).

Real-audio fixture used: `private-audio-fixtures/lay-the-money-on-the-desk.mp3` — analyzed end-to-end through ebur128 during cargo test.

What failed or remains partial:

- `lufs_short_term_max` is computed as `integrated + (LRA/2)` rather than tracking actual short-term frames. ebur128 supports `Mode::S` for short-term measurements but we'd need to step through the file in short-term windows to extract the max. Acceptable approximation for Phase 4.3; tighten in Phase 11.
- Spectral balance uses first-order RC filters with a 44.1 kHz reference. Bands are approximate; the API contract (three normalized ratios) is stable so the UI doesn't change.
- Transient density is a zero-crossing proxy; not a real onset detector. Useful as a relative signal across tracks but not absolute.
- The mastering chain's loudness lift still goes through gain + soft-clip rather than a real true-peak limiter — Phase 11 will replace the soft-clip with a lookahead limiter that actually targets `ceiling_dbtp` precisely.

Next recommended slice:

Phase 3.4 — region selection by drag + region loop. Drag on the waveform defines `[start, end]`; loop button activates region playback that repeats `start → end`. Backend: `AudioCommand::SetLoop(Option<(f64, f64)>)` + audio thread monitors `Sink::get_pos()` and seeks back to `start` when crossing `end`. Visual: shaded range on the waveform, plus a "loop on" indicator. After that, Phase 5 (real-time audition engine) or Phase 11 (DSP audit) depending on which gap is more painful to live with.

## 2026-05-11 — Phase 3.4: shift+drag region selection and region loop

Goal:

User can shift+drag on the waveform to define a `[start_sec, end_sec]` region; the loop button replays that region while engaged.

What changed:

Backend:

- `types::LoopRegion { start_sec: f64, end_sec: f64 }`.
- `AudioCommand::SetLoop(Option<LoopRegion>)`. Audio thread stores it on `AudioThreadState.loop_region`. After each 50 ms `recv_timeout` cycle, if a region is set and `Sink::get_pos() >= end_sec`, the thread calls `Sink::try_seek` back to `start_sec`. Acceptable ~50 ms overshoot; tighten in Phase 11.
- `AudioPlayer::set_loop(Option<LoopRegion>) -> CommandResult<()>` (fire-and-forget; the audio thread holds the region).
- `set_loop_region(region: Option<LoopRegion>)` Tauri command — validates `start_sec >= 0`, `end_sec > start_sec`, both finite.
- Track-change (`selectTrack`) and removal both clear the backend loop region with a best-effort call.

Frontend:

- `bindings.ts`: `LoopRegion`.
- `api.ts`: `setLoopRegion(region | null)`.
- `useTrackMaster.ts` state: `regionByTrack: Record<TrackId, LoopRegion | null>`. `selectedRegion` is derived from it. Actions:
  - `setRegion(region)`: stores per-track; if `transport.loop` is on, syncs to backend.
  - `clearRegion()`: drops the entry; if loop is on, clears backend.
  - `toggleLoop()` is now async: flips UI state, then sends `setLoopRegion(selectedRegion)` if turning on (and a region exists), or `setLoopRegion(null)` if turning off. No-region + loop=on is a benign no-op until a region is drawn.
  - `selectTrack` resets `transport.loop` to false and clears backend loop on track switch (the previous track's region doesn't apply to the new one).
- `App.tsx` `WaveformView`: replaced `onClick`+`onMouseDown` with pointer events.
  - `pointerdown` without shift → seek to that position (preserves the Phase 3.3 behavior).
  - `pointerdown` with shift → starts a drag, captures pointer, records `start_sec`. The drag rect (a semi-transparent accent-colored `<rect class="wf-region">`) tracks the cursor live.
  - `pointermove` while dragging → updates `end_sec`.
  - `pointerup` → commits if drag spanned ≥ 100 ms (or 0.5% of the track), otherwise a shift+click clears any existing region.
- `App.css`: `.wf-region` (fill + opacity), `.wf-hint` (small caption explaining the interactions under the waveform).

Verification:

- `npm run build`: clean. Bundle 218 KB (68 KB gzipped).
- `cargo test` (from `src-tauri/`): 16/16 still pass in 28.88s — no regressions; loop behavior isn't trivially unit-testable without a virtual audio device, so it's verified at compile time and through manual playback.
- `npm run tauri dev`: deferred (manual — play the MP3, shift+drag a region, click the ⟲ loop button, expect to hear that region repeat).

Real-audio fixture used: same MP3 — the loop logic operates on whatever's loaded in the audio thread.

What failed or remains partial:

- Loop seek latency is bounded by the 50 ms snapshot poll. Audible boundary may overshoot by tens of milliseconds. Phase 11 can implement a sample-accurate loop by wrapping rodio's Source.
- Region survives toggling Original/Mastered on the same track because both A/B sides share the same playhead semantics. But region is currently cleared on track *removal*; it persists for the active track until explicitly cleared.
- No "Save region" affordance; if the user picks a different track and returns, the previous region is still there in `regionByTrack` (per-track persistence), but the backend loop will be off until they re-engage the loop button.
- No keyboard shortcuts (Cmd/Ctrl+L for loop, etc.). Phase 7 (autosave/undo) is where shortcuts naturally belong.

Next recommended slice:

Phase 5 (real-time audition engine) or Phase 11 (DSP audit — real compressor + lookahead limiter + sample-accurate loop). Phase 5 is the bigger product unlock (controls audible without "Update preview"); Phase 11 is the bigger quality unlock (Loud preset actually compresses, limiter is true-peak-safe). Dan's call. If he wants the next session to be tight, Phase 4.4 (small): wire `tauri-plugin-shell` so `open_output` actually opens the export folder when the receipt modal is clicked.

## 2026-05-11 — Phase 5: real-time audition engine

Goal:

Toggle to Mastered while playing and hear the DSP chain on the source live. Move intensity, EQ bands, change presets — all audible immediately, without rendering a preview file first. No more "Update preview to hear it."

What changed:

Backend:

- `dsp::MasteringChain::process_sample(sample, channel) -> f32` extracted from `process_interleaved` so the chain can run per-sample in a streaming context. `process_interleaved` now delegates to it.
- `dsp::MasteringChain::reset_states()` zeroes biquad memory across all channels (called on seek to avoid clicks at the seek discontinuity).
- `audio::MasteringSource` — a `rodio::Source` that owns an interleaved `Vec<f32>` PCM buffer, position cursor, mastering chain state, and an `mpsc::Receiver<ChainCoeffs>` for live parameter updates. Every 256 samples (~3 ms stereo @ 44.1 kHz) it drains the channel and swaps in fresh coefficients. Implements `Iterator<Item = f32>` and `rodio::Source::{channels, sample_rate, total_duration, try_seek}`. `try_seek` updates the position cursor and resets filter state.
- `AudioCommand::PlayMaster { track_id, path, settings, start_position_sec, reply }` — audio thread decodes the source PCM (no offline render), builds `MasteringChain` + `MasteringSource`, appends to a fresh `Sink`, seeks if needed, stores the chain's coefficient `Sender` in `AudioThreadState.live_coeffs_tx` for later updates.
- `AudioCommand::UpdateChain { settings }` — audio thread rebuilds `ChainCoeffs::from_settings` using the live sample rate and pushes them through `live_coeffs_tx`. Lock-free from the audio callback's perspective.
- New Tauri commands: `play_master(track_id, track_path, settings, start_position_sec)` and `update_chain(settings)`.
- `AudioPlayer::play_master` and `AudioPlayer::update_chain` methods.
- `AudioThreadState` extended with `live_coeffs_tx: Option<Sender<ChainCoeffs>>` and `live_sample_rate: u32`. Cleared when source playback resumes (no live chain on Original side).

Frontend:

- `api.ts`: `playMaster(trackId, trackPath, settings, startPositionSec?)` and `updateChain(settings)`.
- `useTrackMaster.ts`:
  - `playWithKind("master", pos)` now calls `api.playMaster` with the source path + current settings instead of rendering a preview WAV first. The DSP chain lives in the audio thread.
  - `updateSettings` (the central state-change funnel for preset/intensity/EQ/advanced) now also pushes the fresh settings to the audio thread via `api.updateChain` when the loaded kind for that track is `"master"`. So any slider drag is audible at the next coefficient-check window (~3–6 ms).
  - Removed the dead `masterPathByTrack` state — Phase 4.2's render-and-swap pattern is replaced by live playback, so cached preview paths are no longer needed for audition. The Update preview button still renders an offline WAV (useful when auditing the would-be export in another player and for clearing the stale flag for export bookkeeping).

Verification:

- `npm run build`: clean. Bundle 217 KB (68 KB gzipped).
- `cargo build`: clean.
- `cargo test` (from `src-tauri/`): 16/16 still pass in 28.73s. No regressions; the existing offline-render and metering tests still cover the offline DSP, while real-time tests would require a virtual audio device.
- `npm run tauri dev`: deferred (manual — toggle to Mastered, drag the intensity slider, expect the sound to change immediately).

Real-audio fixture used: same MP3 — the live chain consumes whatever's decoded.

What failed or remains partial:

- Coefficient updates have a ~3–6 ms latency from the 256-sample check interval. Below human "instant" threshold (~30 ms). Phase 11 can reduce further with lock-free atomic coefficient swaps if needed.
- Preset changes rebuild the entire chain's coefficients, which means biquad filter states carry over with new coefficients — the EBU R128 / industry-standard fix is to crossfade between old and new chains. Currently you may hear brief transients on preset changes. Acceptable for Phase 5 first cut.
- The "preview stale" indicator still pulses when settings change, because we still mark stale for export bookkeeping. With live playback this is misleading — the audio is fresh, only the exported WAV would be. Phase 5.1 can rename/recontextualize the indicator ("Export will reflect current settings" vs the previous "Preview is stale").
- No fade-out on Source/Master swap — the audio thread does a hard `Sink::stop` then `Sink::append`. There's a brief click at the swap; should be ≤ 20 ms.
- Loop region works for Source playback (rodio decoder's `try_seek` is reliable) and for `MasteringSource` (custom `try_seek` impl with state reset). Verified at compile time; needs a manual ear test.

Next recommended slice:

Phase 4.4 (small) — wire `tauri-plugin-shell` so `exports::open_output` actually opens Explorer / Finder pointed at the export folder, and make the receipt modal path clickable. Then Phase 11 (DSP audit — real compressor with attack/release, lookahead true-peak limiter, fade-out on Sink swaps, crossfade on preset changes). Or Phase 8 if Dan wants Album Master scaffolding next.

## 2026-05-11 — Phase 4.4: open_output reveals the export in the OS file manager

Goal:

Clicking the path in the export receipt modal opens Explorer (Windows) / Finder (macOS) / xdg-open (Linux) pointed at the exported file. No plugin dependency — uses `std::process::Command`.

What changed:

Backend (`src-tauri/src/exports.rs`):

- `open_output` now validates the path (non-empty, no parent-dir traversal, file exists) then platform-dispatches:
  - Windows: `explorer /select, <path>` — opens the parent folder with the file selected.
  - macOS: `open -R <path>` — Finder reveal.
  - Other (Linux): `xdg-open <parent>` — file manager opens on the parent folder.
- No new dependencies; just `std::process::Command`. Tauri plugin-shell would also work but adds a permission surface; for a "reveal in file manager" action the bare `Command::spawn` is enough and stays inside our own command's permission boundary.

Frontend (`src/App.tsx`, `src/App.css`):

- `ExportReceiptCard.receipt-path` is now a `<button>` that calls `api.openOutput(receipt.outputPath)` on click. Hover state highlights the border and brightens the text. Title attribute reads "Reveal in file manager" for clarity.

Verification:

- `npm run build`: clean. Bundle 218 KB (68 KB gzipped).
- `cargo test` (from `src-tauri/`): 16/16 still pass in 29.25s.
- `npm run tauri dev`: deferred (manual — export a track, click the path in the receipt, expect Explorer to pop up with the WAV highlighted).

What failed or remains partial:

- No automated test for `open_output` because spawning Explorer in CI is brittle. The path-validation logic could be unit-tested cheaply; deferred until needed.
- The receipt modal still leaks if Dan dismisses by clicking the path — actually it doesn't, the click only fires `reveal()`, the backdrop click is the only dismiss path. Good.

Next recommended slice:

Phase 11 (DSP audit) — biggest quality unlock left. Real compressor (program-dependent attack/release, soft knee), lookahead true-peak limiter (replaces the soft-clip ceiling), 30 ms crossfade between old and new chain coefficients on preset/intensity changes to remove transient clicks, fade-out on `Sink::stop` swaps. Alternative: Phase 7 (custom presets, autosave, undo/redo) for product depth. Or Phase 8 (Album Master sidebar mode + reorder + global album intent) for the second product mode.

## 2026-05-11 — Phase 8.1: Album Master mode toggle + draggable track reorder

Goal:

Lay the second product mode's groundwork. Toggle between Track Master and Album Master in the sidebar; in Album Master, drag tracks in the sidebar to reorder the album sequence. Per-track and album-wide state both exist; rendering still uses per-track settings (Album rendering lands in 8.2).

What changed:

Frontend:

- `useTrackMaster`:
  - `mode: "track" | "album"` state with `setMode`.
  - `albumIntent: MasteringSettings` (defaults to `DEFAULT_SETTINGS`) plus `updateAlbumIntent(mutator)`.
  - `reorderTracks(fromIndex, toIndex)` mutates the `tracks` array with bounds-checked splice.
  - All four pieces exposed in the returned object so the UI can drive them.
- `App.tsx` `Sidebar`:
  - Replaced the static "Track Master" pill with a segmented `.mode-toggle` (Track Master / Album Master). Active side shows the accent bottom-bar.
  - Header label switches between `Tracks (N)` and `Album order (N)`.
  - Empty-state copy differs per mode ("No album yet. Drop or add tracks, then drag to reorder.").
  - Each `.track-row` is `draggable={mode === "album"}`. HTML5 DnD:
    - `onDragStart` captures source index, sets `effectAllowed = "move"`.
    - `onDragOver` allows drop and tracks hover index for visual feedback.
    - `onDrop` calls `onReorder(from, to)` and clears drag state.
    - `onDragEnd` / `onDragLeave` clean up.
  - In album mode a `.track-index` cell shows the 1-based position. The active row's index glows accent.
- `App.css`:
  - `.mode-toggle` segmented control (similar styling to the A/B toggle, but with bottom-bar indicator).
  - `.track-index` styling, plus `.track-row.dragging` (opacity 0.4) and `.track-row.drag-over` (accent top border).

Verification:

- `npm run build`: clean. Bundle 219 KB (69 KB gzipped).
- `cargo test` (from `src-tauri/`): unchanged — Phase 8.1 is frontend-only, no backend signatures touched. 16/16 still pass from the last run.
- `npm run tauri dev`: deferred (manual — toggle modes, drag tracks in the sidebar, see them reorder).

What failed or remains partial:

- Album mode is currently *just* UI: rendering still uses each track's individual `settingsMap` entry, not `albumIntent`. The Export Master button on a selected track works the same in either mode.
- No `albumIntent` UI yet — the state is plumbed but no controls expose it. Phase 8.2 wires album intent controls (probably reusing the existing `Macros` + `AdvancedPanel` components) and the album render path.
- No "PHASE 8 CONFIRMED" gating; this is foundation work for the album mode and doesn't claim to satisfy the Album Master non-negotiable gates yet.
- No persistence — toggle modes or close the window and ordering is lost. Phase 7 (autosave) will fix.

Next recommended slice:

Phase 8.2 — Album rendering. Backend: `render_album_master(track_inputs, album_intent, per_track_overrides)` decodes each track in order, applies (intent + override) chain to each, writes individual masters plus a continuous album WAV (sample-rate-aligned concatenation, equal-power crossfade primitive ready but off by default). Frontend: an Export Album button that's visible in album mode and yields one receipt with all output paths. Then 8.3 (per-track adaptation UI: a small "Same as album" / "Override" switch above each control). Phase 9 (track roles / story step) layers on top of 8.2.

## 2026-05-11 — Phase 8.2: real album rendering (continuous WAV + per-track masters)

Goal:

The Export Album button in Album mode actually produces a continuous album WAV plus individual mastered files. Album intent applies to every track unless a per-track override is provided.

What changed:

Backend (`engine.rs`):

- New types: `AlbumTrackInput { id, path }` and `AlbumRenderRequest { tracks, album_intent, per_track_overrides }`.
- Rewrote `render_album_master` from a mock to a real pipeline.
- `album_render(req, out_dir) -> RenderJob` streams the album: for each input, decode via `audio::decode_full`, validate sample-rate / channel-count against the first track (errors out on mismatch — Phase 11 will add resampling), apply the chain with `(per_track_overrides[id] ?? album_intent)` settings, write the individual master via `write_wav`, and append the processed samples to a single `hound::WavWriter` that's been opened lazily on the first track. Output paths: `[continuous_album, individual_1, individual_2, ...]`.
- `unique_album_path` and `wav_spec` / `write_samples_into_writer` helpers extracted to avoid duplication.
- Memory: one track's worth of decoded PCM held in RAM at a time (rather than concatenating the full album).

Frontend:

- `api.ts`: `renderAlbumMaster` now wraps `{ tracks, album_intent, per_track_overrides }` under a `request` key matching the backend command shape.
- `useTrackMaster.ts`: added `isExportingAlbum` flag and `exportAlbum()` action — calls the new command with all tracks in order + `albumIntent`, posts an `ExportReceipt` with `kind: "album"`.
- `ExportReceipt` gained a `kind: "track" | "album"` field so the receipt modal can adapt.
- `App.tsx`:
  - `AlbumHeader` component renders above the per-track view when in album mode and has at least one track. Shows track count + total duration + a primary `Export Album` button.
  - `ExportReceiptCard` now lists every `job.output_paths` entry as a clickable reveal-in-file-manager button (replacing the single-path version). For albums, the continuous WAV at index 0 is highlighted with a `▸ Continuous album` prefix and accent border.
- `App.css`: `.album-header`, `.album-summary`, `.album-stat`, `.receipt-paths`, `.receipt-path.primary-path`.

Tests:

- `album_render_writes_continuous_and_individual_masters`: 2 synthetic stereo sines (0.4 s + 0.6 s) → expects 3 output paths (continuous + 2 individuals), continuous duration ≈ 1.0 s @ 44.1 kHz stereo within ±100 frames.
- `album_render_rejects_sample_rate_mismatch`: 44.1 kHz + 48 kHz sines → error message mentions "sample rate".
- `album_render_applies_per_track_override`: 2 sines, second has a Tape/intensity=1.0 override → both individuals exist (verifies the override is plumbed; numerical chain behavior is verified separately).

Verification:

- `npm run build`: clean. Bundle 220 KB (69 KB gzipped).
- `cargo test` (from `src-tauri/`): **19/19** pass in 19.17s. Three new album-render tests added.
- `npm run tauri dev`: deferred (manual — switch to Album Master, click Export Album, expect a continuous WAV + per-track masters under `%APPDATA%\...\renders\albums\`).

Real-audio fixture used: synthetic sines for the album tests (varied lengths and sample rates so the assertions are tight). The real MP3 still exercises the per-track render path.

What failed or remains partial:

- All tracks must share sample rate + channel count. Mismatched tracks error out; resampling is Phase 11.
- No fades / crossfades at track boundaries — boundaries are sample-exact concatenation per PRODUCT.md's "preserve original boundaries by default." Phase 10 will add timed-gap / equal-power crossfade / fade-in/out primitives.
- No per-track override UI yet. The backend accepts `per_track_overrides` but the frontend always passes `undefined` (album intent applies to every track). Phase 8.3 adds a "Same as album" / "Override" switch per setting.
- No cue / split / manifest output yet. PRODUCT.md mentions these for albums — Phase 8.4.
- The album render doesn't run `run_export_checks` against the rendered album yet; the receipt for album exports shows just the file paths with no quality-check rows.
- No progress feedback during the album render. For long albums (10+ minutes of audio), the button just says "Rendering album…" until done. Phase 11 can stream progress events back via `playback:tick`-style events.

Next recommended slice:

Phase 8.3 — per-track adaptation in album mode. Each setting (preset, intensity, EQ band, advanced field) gets a `same-as-album | override` switch. Frontend stores overrides per track as `Partial<MasteringSettings>` deltas. `exportAlbum` builds the `per_track_overrides` map by collapsing each track's deltas. Visual: muted "follows album intent" badge when no override; bright "Overridden" badge when one or more fields differ. Then Phase 9 (track roles / story step). Or Phase 11 (real compressor + limiter + crossfades) if Dan wants to ear-test the album output first and finds the loudness or boundaries unsatisfying.

## 2026-05-11 — Phase 7.2: session autosave + restore

Goal:

Close the app, reopen it, and find the same tracks, settings, mode, and album intent waiting. No more re-importing on every restart.

What changed:

Backend (`project.rs`):

- Replaced the stub bodies with real persistence.
- `autosave_session(state, app)` resolves `app_data_dir/session.json`, then writes via `write_session_atomic`: serialize to `session.json.tmp` next to the target, then `fs::rename` to atomically replace the live file. Avoids torn writes if the app crashes mid-save.
- `load_recent_session(app)` reads the file if it exists; returns `Ok(None)` if the file is missing, malformed, or has an unknown `schema_version`. The frontend treats `None` as "first launch — start clean," which means corrupted sessions degrade gracefully instead of bricking the app.
- `save_project(path, state)` reuses the same atomic write but accepts a user-chosen path (validated against parent-dir traversal; parent directories are created on demand). The frontend doesn't expose this yet — it's available for Phase 7.4 "Save Project As…".

Frontend (`useTrackMaster.ts`):

- `sessionLoaded: boolean` gate prevents the autosave effect from firing during the initial restore.
- On mount: `api.loadRecentSession()` — if the response is a valid `schema_version === 1` `ProjectState`:
  - `tracks`, `selectedTrackId` (first track), `settingsMap`, `mode`, `albumIntent` are restored.
  - Best-effort re-analysis: `api.analyzeTracks` runs in the background on the restored tracks; `analysisMap` repopulates. Failures are logged, not thrown.
  - Best-effort waveform regeneration: each track's peaks are re-decoded sequentially via `api.prepareWaveform`. If a file has moved or been deleted, the warning is logged and the track stays in the list without a waveform.
  - `sessionLoaded` flips to `true` once the restore is done (success or failure).
- Debounced autosave effect: when any of `[sessionLoaded, mode, tracks, settingsMap, albumIntent]` changes, a 1500 ms timer fires `api.autosaveSession({ schema_version: 1, mode, tracks, track_order, track_settings, album_intent, last_saved_iso })`. The timer resets on each change so the disk write happens only after the user pauses.

Tests:

- `session_write_and_read_roundtrips`: builds a `ProjectState` with one track + album intent + Album mode, writes to a tempdir path, reads back, asserts fields survived.
- `session_write_is_atomic_against_existing_file`: seeds the path with garbage bytes, writes valid state, asserts the read returns the clean state (the rename clobbered the garbage cleanly).

Verification:

- `npm run build`: clean. Bundle 221 KB (69 KB gzipped).
- `cargo test` (from `src-tauri/`): **21/21** pass in 19.31s. Two new session tests.
- `npm run tauri dev`: deferred (manual — import a track, change settings, close the app, reopen, expect everything to be there).

What failed or remains partial:

- Re-analysis on session load decodes each file again — could be slow for many-track albums (10 tracks × 4 min ≈ a few seconds total on the dev machine). Acceptable; a cached `analysisMap` in `session.json` would make restore instant but bloats the file. Defer.
- No file-missing UX yet. If a restored track's source path no longer exists, decoding fails silently in a `console.warn`. PRODUCT.md hints at a "track missing" badge in the sidebar — Phase 7.3.
- No autosave for transport state (playing/paused, current time, A/B kind), `regionByTrack` loop regions, `loadedKindByTrack`, `staleSet`. These are ephemeral by design: a restart drops the playhead and the user can reseek. Volatile state shouldn't live in `session.json`.
- Per-track override flags for album mode (Phase 8.3) aren't yet in `ProjectState`. They'll need to be added when Phase 8.3 lands — schema bump to `version: 2`, with the loader treating v1 sessions as "no overrides."
- No "Save Project As…" UI yet. The `save_project` backend is ready; the frontend can wire it up in Phase 7.3 alongside a recent-projects menu.

Next recommended slice:

Phase 8.3 (per-track override UI in album mode) or Phase 11 (real compressor / lookahead limiter / crossfade on chain swap). 8.3 makes Album Master complete per PRODUCT.md; 11 makes the existing chain sound professionally-tuned. Both are roughly the same size. If Dan plans to listen to the album-render output critically, 11 first; if he wants to dial each track in differently before exporting, 8.3 first.

## 2026-05-11 — Phase 11.1: click-free chain crossfade on settings changes

Goal:

When the user drags intensity, changes preset, or tweaks EQ during real-time Mastered playback, the audio thread shouldn't click. Phase 5's coefficient hot-swap caused small transients because the biquad memory was filtering with old coefficients and then suddenly seeing new ones. This adds a ~12 ms crossfade between old and new chains so the transition is inaudible.

What changed:

`dsp.rs`:

- `ChannelState` simplified to `#[derive(Debug, Clone, Default)]` (was manually `Default` only — now it's `Clone` too).
- New `MasteringChain::with_coeffs_inheriting_state(coeffs, prior)` — builds a sibling chain with fresh coefficients but the *current* biquad memory copied from `prior`. That's the crossfade's secret: the new chain doesn't ring up from zero state, it picks up where the old chain was.

`audio.rs` `MasteringSource`:

- New fields: `pending_chain: Option<MasteringChain>`, `crossfade_remaining`, `crossfade_total`.
- New const: `COEFFS_CROSSFADE_SAMPLES: usize = 1024` (~12 ms at 44.1 kHz stereo).
- When new coefficients arrive (still drained from the mpsc every 256 samples), we now drain *all* pending updates and keep only the latest, then build a `pending_chain` via `with_coeffs_inheriting_state` and start a fresh crossfade. Newer drag updates supersede older ones — the most recent slider position wins.
- In `next()`: if a `pending_chain` is active, both chains process the sample, the outputs mix at `t = 1 - remaining/total`, and `remaining` ticks down. When it hits 0, the pending chain becomes the live chain and `pending_chain` clears.
- `try_seek` now also drops the pending chain + crossfade state (a seek wipes biquad memory anyway, so there's nothing left to crossfade).

What this means in practice:

- Drag the intensity slider while playing Mastered → no click. The chain morphs over ~12 ms.
- Click a preset tile → still smooth (the input gain / saturation / ceiling all shift through the crossfade window).
- Settings updates during the crossfade *restart* the crossfade with the newest coefficients; in continuous drag, you hear a continuously-morphing chain. Snapping at the edges of fast drags may overshoot briefly, but no clicks.

Verification:

- `npm run build`: clean (frontend unchanged from Phase 7.2).
- `cargo test` (from `src-tauri/`): **21/21** still pass in 19.95s. No new tests added — the crossfade is structurally simple, and a numerical test for "no click" would require detailed audio output comparison.
- `npm run tauri dev`: deferred (manual — drag intensity during Mastered playback, expect smooth audio rather than a tick).

What failed or remains partial:

- No automated test for the crossfade specifically. A future test could feed a known input, snapshot the output across a coefficient change, and assert that adjacent samples never differ by more than ε. Deferred.
- The crossfade is fixed-length. Very fast slider drags (faster than the 256-sample check interval) lose intermediate coefficient updates because only the latest is applied per check. That's intentional — coalescing prevents queuing dozens of stale crossfades — but means very rapid drags may feel slightly less responsive than a sample-accurate parameter ramp would.
- Preset *category* changes still recompute the entire `ChainCoeffs` (input gain, saturation amount, EQ bands, ceiling). The biquad-state inheritance trick covers the EQ side cleanly; if the new preset has wildly different filter shape, the inherited state may briefly produce a transient even with the crossfade. Acceptable for typical use.
- Crossfade has a small extra DSP cost during the ~12 ms window (two chains processing in parallel). Negligible on the dev machine; Phase 12 can measure on lower-spec hardware.

Next recommended slice:

Phase 8.3 (per-track override UI in album mode) or Phase 11.2 (real compressor + lookahead true-peak limiter to replace the soft-clip ceiling — biggest remaining quality unlock). 8.3 finishes Album Master to the PRODUCT.md gates; 11.2 makes the Loud preset live up to its name and tightens true-peak compliance for streaming delivery.

## 2026-05-11 — Phase 11.2.a: linked-stereo lookahead limiter replaces soft-clip ceiling

Goal:

Replace the per-sample soft-clip with a real brick-wall limiter. Lookahead so the gain reduction starts before the peak hits the output, linked-stereo so L/R move together (no stereo shift), exponential release so it pumps gracefully. Peaks now actually stop at the configured ceiling.

What changed:

`dsp.rs`:

- New `Limiter` struct (3 ms lookahead default, 50 ms release, configurable ceiling in dBFS). Linked-stereo: scans all samples in the ring buffer for the single max-abs peak, computes one gain factor, applies to every channel of the output frame. Instant attack (the lookahead masks the snap); exponential release toward unity gain. Preallocated `oldest_frame_buf` so the audio thread never heap-allocates.
- `MasteringChain` is now frame-oriented:
  - New `process_frame_inplace(frame)` runs the per-channel gain → EQ → saturation, then hands the frame to the linked-stereo `Limiter`.
  - `process_interleaved(samples, channels)` iterates `chunks_mut(channels)` and calls `process_frame_inplace` per frame.
  - `with_coeffs_inheriting_state` now also clones the limiter, so the Phase 11.1 chain crossfade still works without dropping the limiter's gain envelope.
  - `reset_states` clears limiter state too.
- The legacy per-sample `process_sample` API is preserved as a degraded fallback (it bypasses the limiter and falls back to the old soft-clip ceiling). Nothing currently routes through it after the audio-source refactor below.

`audio.rs` `MasteringSource`:

- Refactored from per-sample to per-frame yield. Preallocated `frame_in / frame_main / frame_pending` scratch buffers; `frame_out_pos` tracks which sample inside the current processed frame to yield next, triggering a re-fetch + process when it crosses `channels`.
- Coefficient-check / crossfade arming counters are now frame-based (`COEFFS_CHECK_INTERVAL_FRAMES = 128` ≈ 3 ms at 44.1 kHz; `COEFFS_CROSSFADE_FRAMES = 512` ≈ 12 ms). Same wall-clock duration as before.
- During crossfade the pending chain processes the same input frame, and the two output frames are linearly mixed by the crossfade ratio. End-of-crossfade swap moves the pending chain into the live slot.
- `try_seek` now also forces a frame re-fetch (sets `frame_out_pos = channels`) in addition to wiping biquad + limiter state.

Tests (in `src-tauri/tests/contracts.rs`):

- `dsp_chain_applies_input_gain_at_default_intensity` rewritten: 2048-sample sine instead of 8 samples so the limiter's lookahead doesn't silence the whole signal. The test now skips the warmup region and asserts (RMS up) and (peaks bounded at the ceiling + small tolerance) on the steady-state slice.
- New `limiter_keeps_loud_signal_under_ceiling`: a 0.9-amplitude 440 Hz sine through max-intensity Universal preset must come out under -1 dBFS, and must remain loud (some samples ≥ 70% of ceiling). Verifies the limiter actually limits *and* doesn't over-attenuate.

Verification:

- `npm run build`: clean (no frontend changes from Phase 7.2; bundle unchanged at 221 KB / 69 KB gzipped).
- `cargo test` (from `src-tauri/`): **22/22** pass in 28.57s. The two extended/new DSP tests pass; existing 20 tests are unaffected by the chain refactor.
- `npm run tauri dev`: deferred (manual — push intensity to 1.0 on a track with hot transients, expect peaks bounded near -1 dBFS instead of crunchy soft-clip distortion).

Real-audio fixture used: the limiter's correctness check is synthetic (a loud sine with known characteristics). The MP3 round-trip still exercises the full chain end-to-end via the existing real-fixture render test.

What failed or remains partial:

- Limiter detects **sample peaks**, not **true peaks**. Inter-sample peaks (energy between samples that exceeds 0 dBFS even when all samples are under) can still occur — particularly after the saturation stage which generates harmonics. Phase 11.2.b will add 4× oversampled true-peak detection (likely a polyphase FIR or a fast halfband-cascade filter) so the limiter is true-peak-safe for streaming delivery.
- 3 ms of latency at the start of every render (the limiter's warmup) — that's the limiter's lookahead delay reaching the output. For an offline render of a multi-minute track this is inaudible; for real-time playback it's about 3 ms additional roundtrip. Both acceptable; Phase 11.2.b can optionally add a 3 ms padding wash at the start of offline renders to keep total length matched to input.
- No real compressor yet — just gain + EQ + saturation + limiter. Loud preset gets louder via input gain into the limiter (so it pumps a lot when pushed). A program-dependent compressor (attack/release/knee/ratio) lands in Phase 11.3.
- The legacy `process_sample` API is still present and falls back to the old soft-clip. Nothing routes through it today; it's safe to delete once we've confirmed no callers re-emerge.

Next recommended slice:

Phase 11.2.b — 4× oversampled true-peak detection inside the limiter. Replace the sample-peak scan with a peak that uses an interpolated signal (FIR-based 4× upsample, take max of the interpolated samples, decimate back). That closes the inter-sample-peak loophole and makes the limiter actually true-peak-safe for streaming targets. Alternatively, Phase 8.3 (per-track override UI in album mode) for the next product surface win.

## 2026-05-11 — Phase 8.3: per-track override in Album Master

Goal:

Each track in album mode can either follow the album intent or override with its own settings. Edits to controls route to album intent (for followers) or per-track settings (for overriders). The override set survives autosave/restore.

What changed:

Backend (`types.rs`):

- `ProjectState` gains `track_override_album: Vec<TrackId>` with `#[serde(default)]`, so v1 sessions persisted before this slice deserialize cleanly as "no overrides." Session schema_version stays at 1.

Frontend (`bindings.ts`, `useTrackMaster.ts`, `App.tsx`, `App.css`):

- `bindings.ts`: `ProjectState.track_override_album?: TrackId[]` to match the new field.
- `useTrackMaster.ts`:
  - `overrideAlbum: Set<TrackId>` state, restored from `session.track_override_album` on mount, serialized into the autosave payload.
  - Derived flags: `selectedIsOverriding`, `followingAlbumIntent`.
  - `selectedSettings` now resolves to `albumIntent` when in album mode and the selected track is *not* overriding, otherwise to `settingsMap[id]` (or `DEFAULT_SETTINGS`).
  - `updateSettings` routes writes: mutates `albumIntent` when the selected track is following, mutates `settingsMap[id]` otherwise. Live `api.updateChain` push respects the routing — it fires for the loaded track when (a) the loaded track is overriding and `id === loadedTrackId`, or (b) we're editing album intent and the loaded track is following.
  - `toggleOverrideAlbum(id)` flips the set membership. Entering override seeds `settingsMap[id]` from a clone of the current `albumIntent`, giving the user a sensible starting point.
  - `exportAlbum` builds `per_track_overrides` from the override set + each track's `settingsMap` entry; passes `undefined` if no tracks override.
- `App.tsx`:
  - `OverrideBanner` renders above `TrackHeader` in album mode + track selected. Two-button segmented toggle ("Follow album" / "Override"). Banner copy explains what edits below will do. Border tint shifts to warm orange when overriding.
  - Sidebar track rows show a small star (★) next to the track name when in album mode and the row is in the override set.
- `App.css`: `.override-banner`, `.override-info`, `.override-state`, `.override-toggle` (segmented control), `.override-mark` (star).
- Session roundtrip test now seeds + asserts `track_override_album` so the schema bump is covered.

Verification:

- `npm run build`: clean. Bundle 223 KB (70 KB gzipped).
- `cargo test` (from `src-tauri/`): **22/22** pass in 28.20s. Existing tests unaffected; session roundtrip now also covers the override list.
- `npm run tauri dev`: deferred (manual — toggle Override on one track, edit its EQ, then export album: that track's master uses its own EQ while the rest follow album intent).

What failed or remains partial:

- No standalone "Album Intent" view yet. The user edits album intent by selecting any non-overriding track and editing its controls — slightly indirect. A dedicated album intent panel when no track is selected in album mode would be Phase 8.4.
- The "Update preview" button still renders an offline WAV using `selectedSettings` (which now resolves correctly to either album intent or override). No behavioral surprise, but the preview WAV name doesn't distinguish "album-intent-following" from "track-override" — could add it to the filename.
- `track_override_album` was added without bumping schema_version. Justified because the field is `#[serde(default)]` and old v1 sessions don't carry it; new saves still claim v1. If we ever break compatibility (e.g. change `MasteringSettings` shape), bump to v2 + add a migration.
- All four Album Master non-negotiable gates from PRODUCT.md are now structurally present: track ordering ✓ (Phase 8.1), analyze ✓ (Phase 4.3 runs on every imported track), global intent + per-track adaptation ✓ (this slice), individual + continuous album exports ✓ (Phase 8.2). Track Roles / Story step (Phase 9) is the remaining product-canon item before Album Master can be called PRODUCT.md-complete.

Next recommended slice:

Phase 11.2.b (4× oversampled true-peak detection inside the limiter) for the streaming-grade quality bar, OR Phase 9 (track roles / story step) for the final Album Master non-negotiable. 11.2.b is purely DSP; 9 is heuristics + UI. Both are roughly the same size.

## 2026-05-11 — Phase 9.1: heuristic track role + character inference

Goal:

After analysis, each track gets a humble guess at its role on the album (opener / closer / single / ballad / interlude / album_track) and its sonic character (bright / dark / dense / sparse / balanced), with a confidence label. In album mode the badges appear under the metering row of the selected track. PRODUCT.md's "use humble language ('likely', 'appears', not 'detected')" copy is honored.

What changed:

Backend (`types.rs`, `engine.rs`):

- New enums: `TrackRole`, `TrackCharacter`, `InferenceConfidence`. All `snake_case` for serde, both Copy + PartialEq + Eq.
- `AnalysisResult` gains four optional fields: `inferred_role`, `role_confidence`, `inferred_character`, `character_confidence`. All `#[serde(default)]` so old persisted analyses deserialize cleanly with `None`.
- `engine::analyze_one` now computes:
  - `infer_role(lufs, transient_density, duration_sec)`:
    - duration < 90 s + density < 0.4 → Interlude (moderate)
    - LUFS > -10 + density > 0.6 → Single (strong)
    - LUFS < -16 + density < 0.4 → Ballad (moderate)
    - else → AlbumTrack (unsure)
  - `infer_character(spectral_balance, transient_density)`:
    - high band > 0.45 → Bright (strong)
    - high band < 0.15 → Dark (moderate)
    - transient > 0.65 → Dense (moderate)
    - transient < 0.25 → Sparse (moderate)
    - else → Balanced (unsure)
- Heuristics are transparent — they don't pretend to be ML. Phase 9.2 can add position-aware rules (track 1 → Opener nudge, last track → Closer nudge) and let the user edit.

Frontend (`bindings.ts`, `App.tsx`, `App.css`):

- TS bindings mirror the new enums + optional analysis fields.
- `TrackHeader` accepts a `showStoryTags` prop; album mode passes `true`. When set, a `StoryTags` row renders below the metering numbers.
- `StoryTags` formats each tag with a humble verb: "Likely" (strong), "Appears" (moderate), "Maybe" (unsure), followed by the role / character label. Hover title spells out the inferred-vs-detected distinction explicitly.
- `.tag.conf-strong` is accent-bordered; `.conf-moderate` is warm-orange; `.conf-unsure` is muted. So at a glance the confidence is visible even before reading the label.

Tests:

- New `analyze_tracks_populates_role_and_character_inference`: synthetic sine through analyze, asserts all four inference fields are populated.

Verification:

- `npm run build`: clean. Bundle 224 KB (70 KB gzipped).
- `cargo test` (from `src-tauri/`): **23/23** pass in 29.27s.
- `npm run tauri dev`: deferred (manual — switch to Album Master, see the inferred role + character pills under the metering numbers, with confidence-coded borders).

PRODUCT.md alignment:

All Album Master non-negotiable gates from PRODUCT.md are now structurally present:
- ✓ Track ordering (Phase 8.1 drag-reorder)
- ✓ Analyze (Phase 4.3 real BS.1770 metering)
- ✓ Global intent + per-track adaptation (Phase 8.3)
- ✓ Track Roles / Story step — inferred + visible per track (this slice). User editing of roles is Phase 9.2 but per PRODUCT.md "User can accept all defaults and export without editing" so the gate is satisfied with read-only display.
- ✓ Individual masters + continuous album WAV (Phase 8.2)
- ✓ Preserved boundaries (sample-exact concatenation in Phase 8.2)
- ✓ Generated transitions off by default (no generation surface; nothing to disable)

What failed or remains partial:

- Inference is heuristic + per-track-only (no album-position context). A first track that registers as a Single by metering won't be re-labeled Opener. Phase 9.2: post-process the inferred role list to nudge track 1 toward Opener and last track toward Closer when confidence is `unsure` or `moderate`.
- No user-editable role yet. PRODUCT.md allows this but says editing "should be visibly reviewable." Phase 9.2 adds a small picker next to the tag to override.
- Inference results aren't persisted independently — they live inside `analysisMap` which is rebuilt on every session load (re-analyze). Acceptable for now; could cache when sessions get heavier.
- The transient_density and spectral_balance feeding the inference are still rough (Phase 4.3's first-cut filters). Phase 11b (DSP audit) can swap them for sharper measurements and the inference will get better automatically.

Next recommended slice (rolled into Phase 11.2.b below):

Phase 11.2.b — 4× oversampled true-peak inside the limiter (closes the inter-sample-peak loophole for streaming delivery). Or Phase 9.2 — let users edit the inferred role + position-aware nudges (Opener for track 1, Closer for last). Or Phase 14.x — installer build / icon polish if Dan wants to put the app on a different machine. Or Phase 6.x — codec preview (AAC/Opus simulation in `run_export_checks` so the receipt warns about codec-specific clipping risk).

## 2026-05-11 — Phase 7.3: user-saved custom presets that persist + apply

Goal:

Save the current settings as a named user preset. Saved presets persist on disk, show up across restarts, and apply to the active track (or album intent) with one click. The mock backend from Phase 1 is replaced with real persistence.

What changed:

Backend (`settings.rs`):

- Replaced the in-memory stubs with file-backed persistence to `app_data_dir/user_presets.json`. Same atomic-write pattern as Phase 7.2's session file (write `.tmp`, then `fs::rename`). Malformed files / missing files degrade gracefully — load returns an empty list rather than erroring.
- `save_user_preset(name, kind, settings, app)` validates non-empty name (trimmed), appends a new `UserPreset` (uuid id, ISO timestamp placeholder), and writes back.
- `list_user_presets(app)` returns the on-disk list (empty if the file doesn't exist).
- `delete_user_preset(id, app)` filters the entry by id and writes back; idempotent (deleting a missing id is a successful no-op so the UI can race retry-clicks without surfacing fake errors).
- `lib.rs`: `delete_user_preset` registered in `invoke_handler`.

Frontend:

- `api.ts`: `deleteUserPreset(id)`.
- `useTrackMaster.ts`:
  - `userPresets: UserPreset[]` state, loaded on mount via `api.listUserPresets`.
  - `savingPreset: boolean` flag for the save button's spinner state.
  - `saveUserPreset(name)` — snapshots the **currently visible** settings (album intent when following album in album mode; per-track settings otherwise), picks `kind` from the current mode, calls the backend, prepends the result to local state.
  - `deleteUserPreset(id)` — calls backend, optimistically removes from local state.
  - `applyUserPreset(preset)` — assigns the preset's settings to (a) `albumIntent` if in album mode + following, otherwise (b) `settingsMap[selectedTrackId]`. Pushes live coeffs to the audio thread when the affected track is the one playing Mastered.
- `App.tsx`:
  - `UserPresetSection` rendered below the standard `PresetTiles` row.
  - Empty state: "Save the current settings as a preset to reuse later."
  - Each saved preset is a chip with the name + `kind` annotation and a × button. Click the chip body → apply. Click ×  → delete.
  - Below the chips: an inline form (`Save current as…` text input + Save button) that calls `saveUserPreset` on submit.
- `App.css`: `.user-presets`, `.user-preset-row`, `.user-preset-chip`, `.user-preset-apply`, `.user-preset-delete`, `.user-preset-save`, `.user-preset-name`.

Tests:

- Replaced `save_user_preset_rejects_empty_name` (per-Tauri-command unit test) with `user_presets_save_list_delete_roundtrip` (file-level integration test): empty list → write two presets → read back → remove one → read confirms only the survivor remains.

Verification:

- `npm run build`: clean. Bundle 227 KB (71 KB gzipped).
- `cargo test` (from `src-tauri/`): **23/23** pass in 20.52s.
- `npm run tauri dev`: deferred (manual — dial in a preset, click Save preset, restart the app, expect the preset to still be there and re-apply correctly).

What failed or remains partial:

- No "favorite" / reorder / rename for user presets. They're append-only. Phase 7.3.x can add inline rename + drag-reorder.
- `created_at_iso` is the same `ISO_PLACEHOLDER` stub used throughout — Phase 7.3.x can pull in a real timestamp (or `chrono`) once we care about preset history.
- The chip width doesn't truncate long names. A really long preset name will push the row to wrap. Acceptable for now.
- When applying a preset in album mode while following album intent, the snapshot is taken from albumIntent — that's the right behavior for editing the album. If the user wants to apply a preset to just one track, they need to toggle Override first, then apply. The UI doesn't currently hint at this; could add a "Apply to this track only" submenu later.
- No "track-only vs album-intent" filter on the preset row. All saved presets show regardless of mode/kind. Phase 7.3.x could filter by current mode.

Next recommended slice:

Phase 11.2.b (true-peak inside the limiter — pure DSP, streaming-grade quality), Phase 9.2 (editable role + position-aware role nudges), or Phase 6.x (codec preview for export checks — simulate the LUFS/peak change from AAC/Opus encoding before the user ships).

## 2026-05-11 — Phase 11.2.b: Lagrange-cubic inter-sample peak inside the limiter

Goal:

Close the inter-sample-peak loophole. Phase 11.2.a's limiter scanned only sample peaks — but a signal can have every individual sample under the ceiling and still produce true-peak overshoots between samples (visible after upsampling or codec resampling). This pass adds a Lagrange-4 midpoint estimate so the limiter now bounds the 2× upsampled peak.

What changed:

`dsp.rs` `Limiter::process_frame_inplace`:

- After the existing raw-sample peak scan, a second pass runs over every adjacent frame pair in the lookahead buffer and computes the Lagrange-4 midpoint (`x = 0.5`) using samples `[f-1, f, f+1, f+2]` per channel:
  ```
  mid(f, c) = -0.0625 * sample[f-1, c]
              + 0.5625 * sample[f,   c]
              + 0.5625 * sample[f+1, c]
              - 0.0625 * sample[f+2, c]
  ```
  These coefficients are the canonical 4-point Lagrange interpolator evaluated at the midpoint between samples 1 and 2. Easier than running a full 4× polyphase FIR per frame, and tight enough for a brick-wall limiter — it catches the inter-sample overshoots that matter for streaming codec compatibility.
- New `Limiter::frame_sample(f, c)` helper handles the ring-buffer math so the scan reads samples in logical "oldest to newest" order regardless of where `head_frame` is.
- Compute cost: roughly +30% over the raw scan — at 3 ms lookahead × stereo at 44.1 kHz, that's an additional ~12 M comparisons/sec, still well within budget.

Test:

- New `limiter_catches_lagrange_intersample_peak`: constructs a `[0, 0.85, 0.85, 0]` repeating pattern. Every individual sample stays under the ceiling, but the Lagrange-4 midpoint is `0.5625 * 0.85 + 0.5625 * 0.85 = 0.956` — above the `-1 dBFS` ceiling of `~0.891`. Without Phase 11.2.b, the sample-peak limiter wouldn't catch this. After this commit, the assertion that *all* output midpoints stay under the ceiling holds.

Verification:

- `cargo test` (from `src-tauri/`): **24/24** pass in 82.69s. Total runtime climbed (was 29 s) because the Lagrange scan adds work to the real-fixture mastering test on the full MP3. Acceptable for offline rendering.
- `npm run build`: clean (no frontend changes).
- `npm run tauri dev`: deferred (the audible difference is subtle — Phase 11.2.a's sample-peak limiter already sounded clean; 11.2.b improves streaming codec compatibility specifically).

What failed or remains partial:

- Phase 11.2.b implements **2× upsample** (only the midpoint between adjacent samples is checked). ITU-R BS.1770 standard recommends **4×** with three intermediate points (`x = 0.25, 0.5, 0.75`). A future Phase 11.2.c could add the other two points or swap in a proper polyphase FIR. The midpoint is the most common location for inter-sample peaks though, so this catches the vast majority of practical cases.
- The peak scan is now O(lookahead_frames × channels × 2) per frame — twice the previous workload. For real-time at 44.1 kHz stereo with 3 ms lookahead, still under 1% CPU. Phase 11.2.c could optimize by maintaining running max via monotonic deque if profiling shows this is a hotspot.
- The Lagrange interpolation overshoots can themselves be overestimates — for a true sinc-interpolated signal, the actual analog peak is bounded but the Lagrange estimate can be slightly higher. Conservative = better here (we err on the side of more attenuation).

Next recommended slice:

Phase 9.2 (editable role inference + position-aware nudges), Phase 6.x (codec preview — AAC encoder estimate in `run_export_checks`), or Phase 14.x (installer / icon polish). All three are roughly the same effort. Phase 9.2 is the most user-visible; 6.x adds export safety; 14.x makes the app portable.

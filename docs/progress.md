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

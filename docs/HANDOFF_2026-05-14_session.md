# Handoff — YES Master — 2026-05-14 evening session

> **One-paragraph snapshot.** App renamed Album Mastering Studio → **YES Master**. All seven UI restyle slices + the post-restyle UX restructure landed earlier in the day; Codex then shipped four more passes that stuck: a console-layout pass (`a3fcc25`) that locks Track Master to a 5-row CSS grid + adds a real-PCM Original-playback path with matched metering / FFT spectrum, the rename itself (`6a441d9`), a default-window bump to 1920×1080 logical pixels with minWidth 1440 / minHeight 860 (`1ea2fa5`), and a zoom-reset (`4878140`) that forces 100% webview zoom on every launch and reverts the variable-zoom keybindings from `4f1e53d`. Codex also tried a Windows-DPI runtime workaround at `a09fe28` (force physical window to 1920×1080 + `set_zoom(1/scale_factor)`) and **reverted it one minute later** at `fc7fa9f`; treat that path as rejected for now, not an open fix to re-land. The settled decision is a 1920×1080 logical app at 100% webview zoom. Dan's ~2880×1620 screenshots are expected Windows 150% display scaling, not an app scale bug, unless Dan explicitly reopens that decision. A second-opinion DSP audit ran against this code; verified items are in the new "DSP Debt — Audit findings" section below. The **next workstream is preset character retuning** per `docs/PRESET_REFERENCE_ANALYSIS_2026-05-14.md` — the missing piece that makes presets feel like creative directions instead of tonal cousins. Everything else is queued behind it.

## Read first (in order)

1. `CLAUDE.md` — repo non-negotiables + the fast/slow test-lane workflow added in audit slice 6.
2. `docs/PRODUCT.md` — product canon. Title is now **YES Master** (renamed). Do not modify without explicit ask.
3. **This file** (`docs/HANDOFF_2026-05-14_session.md`).
4. `docs/PRESET_REFERENCE_ANALYSIS_2026-05-14.md` — the calibration analysis driving the next workstream. Read top-to-bottom; the Suggested Claude Task List at the bottom is what we're executing.
5. `docs/UI_LAYOUT_REVISION_1600x940.md` — the closed UI spec. **Layout work is Codex's lane this session**; only read if a preset change forces a layout edit.
6. Tail entry of `docs/progress.md` for the latest slice-level state.

Do not read by default: `docs/reference/`, `docs/research/`. They're context for the original port, not active work.

## Current branch state

- **HEAD**: this handoff has been refreshed after `b977a23` / `fc7fa9f` to reflect the settled DPI decision: keep the 1920×1080 logical canvas and 100% webview zoom; do not re-land the physical-window workaround unless Dan explicitly asks.
- **Remote**: pushed to `origin/master`
- **Local tree**: clean after this handoff refresh commit.
- **Tests** (stale full-suite memory, last fully recorded at `6a441d9`): `cargo test --lib` 80/80; `cargo test` 138/138 with the real-fixture tests skipping by default (set `AMS_RUN_REAL_FIXTURE=1` for the slow lane). `npm run build` clean. Later UI/window/zoom work landed after that full run, so **rerun both as a first move next session**. `a3fcc25` added a new `MeteredPcmSource` constructor signature, `1ea2fa5` rewrote CSS for the higher-resolution canvas, `4878140` simplified the zoom hook. `a09fe28` added `configure_main_window_physical_target` to `lib.rs` but was reverted by `fc7fa9f`, so `lib.rs` is back to its pre-DPI shape.

### What just landed (this session)

| Commit | What |
|---|---|
| `e803e83` `fbefe54` `e1e71ad` `7447435` `cf3fcf8` `a368d02` | UI layout revision L1–L5 + L4b (live FFT spectrum). The whole `UI_LAYOUT_REVISION_1600x940.md` spec is shipped. |
| `47c8bb0` | Codex audit slice 6 — `cargo test` fast/slow lane split via `AMS_RUN_REAL_FIXTURE` env var. |
| `4f1e53d` | In-app zoom keybindings (`Ctrl+=` / `Ctrl+-` / `Ctrl+0`) so the canvas can render larger physically without changing Windows scaling. |
| `cc360e4` | `docs/PRESET_REFERENCE_ANALYSIS_2026-05-14.md` tracked. |
| `a3fcc25` | **Codex** — console-mode layout pass. Adds `@media (min-width: 1280px) and (min-height: 820px)` CSS grid that locks the workspace to the viewport (no main-canvas scroll, rail-only scroll). New `MeteredPcmSource` for Original playback so peak / LUFS / spectrum populate during A/B (not just Mastered). `MasterOutPanel` keeps the deck meters column; `LevelsPanel` + `StereoWidthGauge` are `display: none` in console mode. |
| `6a441d9` | **Codex** — rename Album Mastering Studio → YES Master in `productName`, window title, brand name, README, PRODUCT.md, etc. Tauri `identifier` stays `com.albummasteringstudio.app` (changing it would break installs); the Cargo package name and repo folder also stay for the same reason. |
| `2ba39b2` | This handoff doc itself — queues preset retuning as the next workstream and documents Codex's file-ownership lane. |
| `1ea2fa5` | **Codex** — Tauri window default bumped to **1920×1080**, `minWidth` 1440 / `minHeight` 860. App.css gained 177 lines retuned for the higher-resolution canvas. Closes the "1920×1080 canvas decision" open-queue item from the previous handoff revision. |
| `4878140` | **Codex** — `useWebviewZoomShortcuts` simplified to **force 100% zoom on every launch**. Reverts the variable-zoom behavior shipped at `4f1e53d`; `Ctrl+=` / `Ctrl+-` keybindings are removed. `Ctrl+0` still resets to 100% (effectively a no-op now). |
| `a09fe28` / `fc7fa9f` | **Codex — DPI workaround, attempted and reverted within a minute.** `a09fe28` added `configure_main_window_physical_target` in `src-tauri/src/lib.rs` that at app setup forced the **physical** window to 1920×1080 and applied `set_zoom(1.0 / scale_factor)` so the CSS design canvas would stay 1920×1080 regardless of OS scaling. It also lowered the `tauri.conf.json` logical-pixel values to 1280×720 / 960×573. `fc7fa9f` reverted it one minute later. Net effect on the tree is zero (lib.rs back to pre-DPI shape, tauri.conf.json back to 1920×1080 / 1440×860 logical pixels). **Current decision:** do not re-land this. The app target is 1920×1080 logical pixels at 100% webview zoom. Dan's ~2880×1620 screenshots are normal Windows 150% display scaling, not an app-side scale bug. Only revisit physical sizing if Dan explicitly asks for a separate smaller-window mode or a true DPI-aware physical-pixel target. |

### Codex's parallel lane

Codex owns the **UI layout / CSS** files for the foreseeable future. If you need to touch them for a DSP-driven reason, **pull first** and coordinate with Dan to avoid clobbering Codex's in-flight work.

Files Codex is touching:

- `src/App.tsx`
- `src/App.css`
- `src/components/RightRail.tsx`
- `src/components/AlbumPanel.tsx` (likely; Album mode UI polish is still open)

Files **safe to edit** for this preset workstream:

- `src-tauri/src/dsp.rs` — preset calibration table, `ChainCoeffs::from_settings`, MasteringChain compressor wiring
- `src-tauri/src/types.rs` — only if adding new preset-compressor fields to `MasteringSettings::advanced`
- `src-tauri/tests/preset_signature.rs` — the existing per-preset character regression test (extend with the new distinctness assertions)
- `src-tauri/tests/preset_loudness_balance.rs` — already passes; revisit after retune to confirm spread stays <4 LU
- `src-tauri/tests/contracts.rs` — only if adding a new preset-distinctness contract test
- `src/bindings.ts` — only if new advanced fields need TS mirroring

## Primary workstream — Preset character retuning

### Why this is next

Per `docs/PRESET_REFERENCE_ANALYSIS_2026-05-14.md` (lines 104–122): the Rust preset table already declares fields like `compressor_threshold_dbfs`, `compressor_ratio`, `transient_punch`, and `target_lufs`, but **most of them are captured and not applied** in the live chain. The user-audible result is that presets feel like minor EQ variations instead of distinct creative directions. The analysis doc compared the app's presets against real online-mastered versions of Dan's track and produced concrete starting values for an 8-preset retune (see lines 252–259) along with a paired dynamics map (lines 265–274).

The product framing (lines 213–215) is the lock:

> Presets are not just safe technical defaults. They are creative direction buttons.

Preset retuning is the slice that delivers on that promise. It is **higher impact than the remaining Codex audit findings** because it changes what every user hears every time they switch presets, not just first-click latency on a long file.

### Required reading inside the analysis doc

- **Lines 104–122** — current implementation gap (which fields are captured but not applied)
- **Lines 252–259** — Conservative Target Table (8 presets × 8 fields, the starting values to land on)
- **Lines 265–274** — Dynamics Targets per preset (threshold/ratio/attack/release direction)
- **Lines 322–349** — Suggested Claude Task List (the six slices)
- **Lines 365–371** — Good Enough Target (acceptance phrasing)
- **Lines 277–321** — Inferred Preset Notes for Spatial / Warmth / Punch / Loud (they weren't in the measured set; these are the design intents)

### Implementation plan

The work breaks into six sub-slices (P1–P6) that map 1-to-1 onto the analysis doc's task list. Ship each as its own commit on master (audio-thread changes; the autonomy rules still allow direct-to-master, per the AMS autonomy memory).

#### P1 — Wire preset compressor into the live chain

Files: `src-tauri/src/dsp.rs`, possibly `src-tauri/src/types.rs`.

Inspect the existing `PresetCalibration` table: does it already carry `compressor_threshold_dbfs` and `compressor_ratio` per preset, or are those user-overridable advanced fields only? Decide on the data shape:

- **Option A**: extend `PresetCalibration` with `compressor_threshold_dbfs` / `compressor_ratio` / attack / release.
- **Option B**: keep one user-side advanced field, drive it from the preset at chain-build time.

Whichever shape lands, `ChainCoeffs::from_settings` must apply the preset's compressor identity by default (so a user who never touches Advanced still hears the preset's dynamics signature). The user's `compression_density` should scale the preset's base, not replace it (per analysis doc line 185: "Treat preset compressor settings as the base. Let user `compression_density` scale the base rather than replace it.").

#### P2 — Define preset/user interaction model

Document and implement: at default `compression_density`, preset behavior is fully applied. At lower densities, scale toward bypass. At higher densities, push slightly past the preset baseline (but bounded by safety). Pure bypass only when the user explicitly disables compression (analysis doc line 187).

Same rule for EQ: user EQ sliders **add to** preset EQ (already the case for `eq_low_db` etc. — preserve this).

Same rule for width: user `width` override replaces preset width when set; otherwise scale.

#### P3 — Retune the calibration table

Use the conservative-target values from `PRESET_REFERENCE_ANALYSIS_2026-05-14.md` lines 252–259 as starting points. The Codex source numbers currently in `dsp.rs::PRESETS` should be replaced with these, NOT the more aggressive measured-reference values (lines 148–153). The conservative set keeps the presets broadly usable while still making each immediately legible.

Specific values to target (table at line 252):

```
                  low_shelf low_mid presence  air saturation width gain compression
Universal           +0.2    -0.1    +0.0     +1.1   0.03    1.04  +1.2 light transparent
Clarity             +0.2    -1.0    -0.8     +1.7   0.025   1.02  +0.8 light, transient
Tape                -0.2    +0.3    -1.4     +2.0   0.10    0.99  +1.5 glue, crest reduction
Spatial             +0.1    -0.8    -0.3     +1.3   0.04    1.16  +1.0 light, clean sides
Oomph               +2.4    -3.0    -2.6     -0.8   0.045   0.95  +1.8 medium, low/mid control
Warmth              +0.8    +0.7    -1.8     -0.8   0.08    0.98  +1.0 soft glue
Punch               +0.8    -1.8    +1.6     +0.8   0.035   1.04  +1.6 faster attack/release
Loud                +0.4    -1.6    +1.8     +1.2   0.055   1.03  +2.5 strongest density/limiting
```

The dynamics direction per preset is at line 265 — read in tandem with this table.

#### P4 — Preset-distinctness contract test

New file: `src-tauri/tests/preset_distinctness.rs` (or extend `preset_signature.rs`).

Render the same source (synthetic or the existing test fixture) through each preset and assert:

- **Universal vs Clarity**: 1.5–4 kHz band of Clarity is at least 1.0 dB below Universal's; 8–16 kHz band is at least 0.8 dB above.
- **Universal vs Oomph**: 20–60 Hz band of Oomph is at least +1.8 dB above Universal's; 250 Hz–2 kHz region is at least 2.0 dB below.
- **Universal vs Tape**: crest factor (true peak − integrated LUFS) of Tape is at least 0.8 dB lower than Universal's.
- **Punch vs Loud**: Punch's crest factor is at least 0.4 dB higher than Loud's (Punch preserves more transient movement).
- All 8 presets share their integrated LUFS within 4 LU at default intensity (this is the existing `preset_loudness_balance.rs` assertion — verify it still passes after retune).

If any assertion fails, **adjust the calibration table, do not weaken the assertion** — the test is the spec.

#### P5 — Render the private fixture through the app + rerun the band-delta analysis

Use the private fixtures in `tests for presets/` (gitignored; analysis doc line 5 references this folder). Render `It's a coat-original-test.wav` through each retuned preset via the app's render path, then run the same band-delta analysis the doc used. Compare the resulting deltas against the acceptance check at lines 191–198:

| Preset | Expected App Result |
|---|---|
| Universal | mostly neutral, 8–16k about +0.8 to +1.2 dB after match |
| Clarity | mids clearly reduced, air clearly lifted, not much louder than Universal |
| Oomph | strongest tonal contrast; sub/low lift and mid scoop obvious within 5 seconds |
| Tape | crest factor reduced more than Universal/Clarity; quiet sections feel denser |

If the analysis script isn't checked in, ask Dan or work from the band definitions in the analysis doc (line 32) and the Rust `engine::measure_integrated_lufs_at_path` + Goertzel helpers in the existing test files.

#### P6 — Safety checks

New contract test or extension to `preset_distinctness.rs`:

- For each factory preset at default intensity (0.5), render a known-loud source (e.g. the 1 kHz sine fixture or pink noise) through the chain and assert the rendered WAV's measured true peak is ≤ −0.1 dBTP (no clipping).
- Loud / Punch / Oomph / Tape cannot rely on EQ + input gain alone to pass — the compressor / limiter must engage. Assert the rendered peak is below the user's effective ceiling.

Reference: analysis doc lines 220–231 for the safety principle.

### Acceptance check

After all six slices ship, the workstream is done when **all of these hold simultaneously**:

1. `cargo test --lib`: 80/80 (or N/N — count may grow from P4/P6).
2. `cargo test`: 138+/138+ (fast lane) with the new preset-distinctness contract test passing.
3. `AMS_RUN_REAL_FIXTURE=1 cargo test` (slow lane) still passes if a fixture is present.
4. `npm run build`: clean.
5. Listening pass on Dan's actual track (`It's a coat`) — Universal, Clarity, Oomph, Tape are immediately distinguishable in a 5-second A/B with Volume Match on. Preset differences survive volume matching. Oomph and Tape feel dynamically different, not just tonally different (analysis doc lines 365–371).

### Constraints

- **Codex owns UI**: do not edit `src/App.tsx`, `src/App.css`, `src/components/RightRail.tsx`, or `src/components/AlbumPanel.tsx` unless a preset change forces a UI surface update AND you've pulled latest first.
- **Dev binary lock**: if Dan has `npm run tauri dev` running, the standard `cargo test` build can fail with "cannot remove file `target/debug/album-mastering-studio.exe`" (the executable name hasn't been renamed even though the app is now YES Master). Workaround: `cargo test --lib` (no integration tests) or `cargo test --tests --target-dir target-tests` for a scratch build dir. Used twice this session, works reliably.
- **`MasteringSource::new` signature**: gained a 9th argument (`spectrum_ring: Arc<SpectrumRing>`) in L4b. Codex's `a3fcc25` introduced a sibling `MeteredPcmSource` for Original playback. Any new audio-thread integration test needs both source types.
- **Preset calibration is the source of truth**: do not retune via the user-facing advanced sliders. Update `PresetCalibration` values directly so the audible identity follows the preset, not the user override.

## DSP Debt — Audit findings (verified 2026-05-14)

A second-opinion DSP / security audit (separate Opus session) was run against the code at HEAD `4878140`. Every claim was checked against source. Verified items are real and slot-able; overstated items are clarified; refuted items are listed so they don't show up in a future audit and re-trigger the same investigation. The headline item is item 1 — a literal correctness bug. Items 2–8 are refinements, not bugs.

### Verified — slot into a future slice

1. **engine.rs:1188 — `let energy_density = 0.5_f32` hardcoded in `render_album_plan_impl`.** The album EXPORT path discards per-track energy density and passes neutral 0.5 to `apply_album_shadow`. The plumbing exists everywhere else: `compute_energy_density_score` at engine.rs:670, `AnalysisResult.energy_density_score` populated at engine.rs:243, `album.rs` threads `analyses[i].energy_density_score.unwrap_or(0.5)` through its planner at album.rs:425, and `apply_album_shadow` at album.rs:345 accepts `energy_density: Option<f32>` and uses it in `source_comp = (0.5 - energy_density) * 0.45` (album.rs:336). The album export render loop is the one place that throws it away. Net effect: the album-arc character bias's presence-band energy-gate is dead in the export path. **Priority: real correctness bug, separate slice (item 1 in the open queue).** Fix shape: the PCM is already decoded in the loop at engine.rs:1152 — call `crate::engine::compute_energy_density_score` against `&pcm` and pass that instead of the literal. ~10 lines.

2. **engine.rs:1437-1438 — Dither quantization range asymmetric.** `INT16_SCALE = 32_767.0`, `INT24_SCALE = 8_388_607.0`. TPDF dither is symmetric around 0; int16 range is `[-32_768, 32_767]`; the most-negative value is unreachable. Audibly inconsequential — <1 LSB DC offset, well under -90 dB — but technically incorrect. Fix is one line per scale: use `32_768.0` / `8_388_608.0`, keep the existing `clamp(-1.0, 1.0)` to prevent positive overflow.

3. **dsp.rs:1042-1065 — Limiter peak scan is O(lookahead × channels) per output frame.** Every frame, the limiter does a linear pass over the full lookahead ring (line 1042), then runs Lagrange-4 ISP interpolation across every adjacent frame pair × 3 positions × channels (line 1049). At 192 kHz / 5 ms lookahead / stereo that's ~960 frames × 3 positions × 2 channels of work per output sample. A monotonic-queue max would amortize the raw-peak pass to O(1); the Lagrange pass can fold into the same windowed-max structure. Pure perf, not correctness.

4. **dsp.rs:992-996 / 1081 — Limiter release is plain one-pole, single time constant.** `release_coef = exp(-1 / (release_ms * sample_rate / 1000))`. No program-dependent attack/release, no dual time constants. Releases sound the same regardless of material. DSP refinement, not a bug — Dan can tune `release_ms` per-preset if a specific character needs it.

5. **engine.rs:1403-1422 — Dither RNG is a single shared xorshift32 across L/R.** `DitherRng` carries one `u32`; the offline render loop draws sequentially for L then R, so left/right TPDF samples are one xorshift32 step apart — mathematically correlated. Audibly inconsequential at -90 dB FS; textbook practice is per-channel RNG or 64-bit state.

6. **No noise-shaped dither.** Pure TPDF at the LSB (grep confirms zero `noise_shap` / `shaped_dither` symbols anywhere in `src-tauri/src`). Comments at engine.rs:1390-1395 confirm this is intentional — flat noise floor, ~3 dB extra noise, "inaudible at 16-bit; below hearing at 24-bit." F-weighted or SBM-family shaping would push quantization energy above ~16 kHz. Optional refinement.

7. **engine.rs:1162-1171 — Album mode hard-stops on SR / channel mismatch.** "(resampling not yet supported)" is in the error string. For Dan's own album where every track is the same SR + channel count this is fine; for a generic user it's a hard fail. Lower priority than item 1.

8. **engine.rs:1130-1152 — Album mode loads all decoded PCM in RAM.** `decode_full(path)` per track, results held through the render loop. For Dan's own album (10 tracks × ~4 min × 44.1 kHz × 24-bit stereo decoded to f32) this is roughly ~840 MB — comfortable on Dan's machine. Pre-emptive concern, not current.

### Overstated by the audit — clarified

9. **"Limiter has no inter-sample-peak detection."** Refuted by dsp.rs:1049-1065. The limiter does Lagrange-4 polynomial interpolation at x ∈ {0.25, 0.5, 0.75} between every adjacent frame pair — a real ISP estimator, just not a polyphase ×4 oversampler. Phase 11.2.b's x=0.5-only check was expanded specifically to catch sign-asymmetric patterns where the true peak falls near x=0.25 / x=0.75 (see the comment block at dsp.rs:1033-1040). A textbook ×4 oversample would catch ~0.1 dB more on pathological waveforms; otherwise the Lagrange estimator is within a fraction of a dB. Not worth re-architecting.

10. **"files.rs `has_parent_dir_component` is cosmetic."** Partial. It DOES reject the `..` traversal pattern (`Component::ParentDir` match at files.rs:15) — that's real path-traversal defense, not cosmetic. What it doesn't do is restrict reads to a specific allowlist root. In Tauri's threat model (frontend = trusted source code, paths come from the user's native file picker), this is acceptable defense-in-depth. Worth a one-line clarification in a docstring; not a slice.

11. **"`open_output` is a command injection risk."** Refuted by behavior. Windows: `explorer /select,` highlights the file — doesn't execute it. macOS: `open -R` reveals in Finder. Linux: `xdg-open` is called on the **parent directory**, not the file (exports.rs:150), so even a `.desktop` payload wouldn't auto-execute. `path.exists()` and `has_parent_dir_component` are also pre-checks. An extension allowlist would add defense-in-depth but the active surface is small.

### Frontend debt — Codex's lane, do not refactor from Claude side

- **`src/App.tsx`** — 2,438 lines (verified). Should split into `src/components/AlbumColumn.tsx`, `src/components/DeckMeters.tsx`, `src/components/TransportBar.tsx`, etc. Codex is actively iterating this file; any refactor from Claude would clobber in-flight work.
- **`src/hooks/useTrackMaster.ts`** — 1,501 lines, ~88 hook calls (verified). Single mega-hook returns the full app state; every consumer re-renders on every state change. Splitting into `useTransport`, `useAnalysis`, `usePresets`, `useAlbum` would localize re-renders.
- **No `React.memo` anywhere** — grep returns zero matches in `src/`. High-traffic display components (meters, waveform, spectrum) re-render on every snapshot tick. Memo-ing them is a low-risk win.

### Tauri config — `tauri.conf.json` (verified)

- **`security.csp: null`** (line 24). Acceptable for development with `frontendDist: ../dist` and Tauri 2's same-origin model. A production build for anyone other than Dan should land a proper CSP first.
- **`bundle.active: false`** (line 28). **Important context:** no installer is being built. `npm run tauri build` produces the `.exe` only — no `.msi` / `.app` / `.dmg` / `.deb`. Fine for Dan running locally; the gate for ever distributing the app.

## Open queue (NOT for this session)

Listed in priority order. Each is a self-contained slice; pick up in this order once preset retuning is closed.

1. **Album export `energy_density` bug** — engine.rs:1188 fix per DSP Debt item 1 above. Real correctness bug: the album EXPORT path hardcodes `let energy_density = 0.5_f32` instead of using the per-track analysis value, dead-coding the album-arc character bias's energy-gate. Fix shape: the PCM is already decoded at engine.rs:1152; call `crate::engine::compute_energy_density_score` against it and pass that to `apply_album_shadow`. New regression test in `src-tauri/tests/album_*.rs` asserting that two tracks with different spectral energy distributions get different `presence_db` shadow values through the same album plan.

2. **Codex audit slice 7** — first-Mastered-click decode stall (~1–2 s freeze on long WAVs). With Codex's `a3fcc25` introducing `MeteredPcmSource` + the decode cache for Original playback, the architecture for this slice is now mostly there — `handle_play_master` already uses the decode cache. The remaining work: kick `decode_full` on the audio thread when a track is selected (not on first play) so the cache is warm before the user clicks Mastered.

3. **Album-master export receipt** — Slice 1 left `RenderJob.measurements = None` for the album path. Album exports still surface source-analysis numbers, not rendered. The fix needs a multi-segment EbuR128 collector that spans the per-track segments the album writer emits, then publishes the aggregate on the returned `RenderJob`.

4. **Album-mode UI polish** — per-transition Gap-seconds spinner, drag-to-reorder inside the album panel, "Album: –1.05 LUFS / ×0.94 intensity" workspace badge per track. Codex's lane (UI).

5. **"New project" menu action** — demoted from Codex audit P1 #2 (was: auto-restore → make clean-boot default). The agreed approach was to leave auto-restore in place and add a Tools-menu "New project (close current)" action. Small slice, ~30 min.

6. **Limiter perf — monotonic-queue max** — DSP Debt item 3. Replace the O(N) per-frame peak scan + Lagrange ISP loop with a monotonic-queue windowed-max so the limiter cost is O(1) per output frame. Existing limiter tests must still pass (this is a perf refactor, not a behavior change). Add a microbench under `src-tauri/benches/` to confirm the speedup.

7. **Dither correctness pass** — DSP Debt items 2, 5, 6 batched. (a) Bump `INT16_SCALE` / `INT24_SCALE` to the correct power-of-two so the most-negative integer value is reachable. (b) Split `DitherRng` into per-channel state so L/R draws are independent. (c) Optional: F-weighted noise-shaping behind a `MasteringSettings.advanced.noise_shaping: bool` flag (default false for now; Dan can A/B against TPDF). All three are tiny edits; group them so the existing dither tests get touched once.

8. **Preset PNG optimization** — each preset PNG is 1.0–1.8 MB. Converting to WebP / AVIF could cut ~70% off the ~13 MB bundle weight. Cosmetic, low priority.

9. **Top-bar version string + user avatar** — mockup at `docs/UI_LAYOUT_REVISION_1600x940.md` shows "YES Master v1.2.0" + an "SM" avatar circle. Cosmetic; skipped in L5. Codex's lane.

10. **Tone-shape per-knob frequency dropdowns** — mockup shows "120 Hz / 2.5 kHz / 10.0 kHz" under each Low/Mid/High knob. DSP frequencies are currently fixed at 200/400/1500/6000 Hz; the UI labels could clarify this without enabling user-adjustable frequencies (the Visual EQ comment already documents why frequency drag is deferred — DSP doesn't support variable band frequency yet).

## Memory carried forward

These are durable preferences from the AMS autonomy memory and the session's accumulated feedback. They apply to all sessions, not just this workstream.

- **High autonomy** on this repo: install deps, run tests, commit + push to master when work is verified. Do NOT merge feature branches to master without explicit ask.
- **No check-in chatter**: when Dan says "dive in" / "keep going" / "go", chain commits. Don't `AskUserQuestion` between every slice.
- **Hold evidence under pressure**: if the test or the data backs the call, hold it. Don't capitulate to social pressure.
- **No under-building**: Dan needs features day-one. Do not v1-then-v2-stage when the spec is clear.
- **Lock in when shipping**: drop strategy docs, estimates, "this is hard" language when Dan signals shipping pressure.
- **Listening calls are Dan's**: sound-quality decisions only happen when Dan signals "I listened to it." Don't fake them. This applies hard to preset retuning P5/P6 — the numeric assertions are the gate; the final "does this feel right" call is Dan's listening pass.
- **`PRODUCT.md` is canon**: read-only without explicit ask. The YES Master rename is already in there; do not modify further without Dan's go-ahead.

## Commit shape

Match the established pattern from this session's commits (see `git log --oneline -20`):

```
<Slice tag>: <slice name>

<one-paragraph what + why>

- <bullet 1>
- <bullet 2>
...

Verification:
- cargo test --lib: N/N pass
- cargo test: M/M pass (or skipped — note env var if relevant)
- npm run build: clean / untouched
- AMS_RUN_REAL_FIXTURE=1 cargo test: passing (if the slice touches anything safety-critical)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

Subject under 70 characters. Push to `origin/master` after every passing slice.

## When to stop and ask

- The slice requires a product decision `docs/PRODUCT.md` doesn't answer (e.g., "should Universal at intensity 0 be true-bypass or still apply +1.2 dB push?").
- Two consecutive slices fail their own contract tests.
- The retune's listening verification step (P5) needs Dan's ears.
- You would need to touch `src/App.tsx` / `src/App.css` / `src/components/RightRail.tsx` for any reason — coordinate with Dan first to avoid clobbering Codex's in-flight work.
- The preset distinctness contract test (P4) reveals a structural DSP issue rather than a calibration miss (e.g., a band the chain doesn't actually have).

When you stop, append a `progress.md` entry that clearly states the blocker — same convention as every other session-end.

## Review checkpoint findings (2026-05-14)

First checkpoint of this build. Full report at `docs/checkpoints/checkpoint-2026-05-14-pre-preset-retune.md`. Build verified read-only (`cargo test --lib` 81/81; handoff said 80 — count drifted by one, refresh on next edit). No drift between handoff claims and source; every verified DSP-debt item still holds against current code.

### Real bugs (carried into next session as-is)

- **B1.** `engine.rs:1188` — album-export discards per-track `energy_density` and passes literal `0.5` to `apply_album_shadow`. Already in open queue as item #1; reaffirmed.
- **B2.** `engine.rs:1437–1438` — `INT16_SCALE` / `INT24_SCALE` use `32_767.0` / `8_388_607.0`; with `clamp(-1.0, 1.0)*scale` the most-negative integer is unreachable. Audibly inconsequential. Already in open queue as item #7a.

No new broken-broken bugs found this pass.

### Top priorities for next session

1. **Continue with P1–P6 preset retune as planned** — checkpoint reaffirms the workstream is well-targeted and the data shows `compressor_threshold_dbfs` / `compressor_ratio` truly are not consumed by `ChainCoeffs::from_settings`.
2. **Suggested ordering refinement: write P4 (`preset_distinctness.rs`) first as a failing test, then retune until it passes.** Makes the contract the gate exactly per "the test is the spec."
3. **Resolve before P4: does the Punch-vs-Loud crest-factor assertion ship in this pass (with only compressor movement available), or does it wait until a transient shaper exists?** `transient_punch` has been "captured but not applied" alongside `compressor_*` since Phase A2; P4's transient-distinctness assertion can't fully land without a shaper. Make the call up front so the contract is honest.

(Push-back item PB1 — routing PlaybackTick through a smaller subscriber to break the `useTrackMaster` mega-rerender — is filed in the checkpoint but is Codex's lane to act on, not Claude's.)


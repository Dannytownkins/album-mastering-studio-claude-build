# Album Mastering Studio Implementation Plan

Last updated: 2026-05-11

This is the execution map for the current Codex/Tauri/Python repo. Read `docs/PRODUCT.md` first. `PRODUCT.md` is the product canon; this file is the living implementation plan.

Do not treat this as a promise that the current architecture is permanent. The plan starts from the existing repo because it already has working engine, Tauri, sidecar, reports, and smoke-test proof. It also includes early architecture research and engine modernization gates so the product is not trapped inside the first implementation.

## Current Strategic Direction

Build a top-tier private desktop mastering app around two modes:

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
- Gap/crossfade/boundary primitives.
- Generated transitions off by default.
- Cue/split data when appropriate.
- Album dashboard/report.
- Album-level quality checks.

## Workstream Overview

1. Current repo stabilization and docs.
2. Competitive and architecture research spike.
3. Rust/Tauri typed app foundation.
4. Track Master frontend rebuild.
5. Playback, waveform, and A/B foundation.
6. Real-time audition spike and engine decision record.
7. Track Master export and quality checks.
8. Presets, custom settings, autosave, undo.
9. Album Master workflow.
10. Transition primitives.
11. DSP audit and engine modernization.
12. Real-audio fixture testing and listening loop.
13. Installer/release hardening.

These streams can overlap, but every phase must end with a no-victory-lap check against `docs/PRODUCT.md`.

## Phase 0: Canon And Repo Baseline

Goal: make sure every agent starts from the same product reality.

Tasks:

- Keep `docs/PRODUCT.md` as the canonical product record.
- Keep this file as the living execution plan.
- Keep `docs/progress.md` as detailed session evidence.
- Update `docs/codex-active-handoff.md` before handoff or compaction when current work is mid-flight.
- Confirm current repo commands still run before large refactors.

Verification:

```powershell
python -m compileall -q src tests
python -m unittest discover -s tests
python -m album_mastering_studio.cli smoke --output test-output\codex-plan-baseline-smoke
cd desktop
npm run build
npm run test:integration
```

No-victory-lap check:

- Product canon exists.
- Implementation plan exists.
- Known untracked/private research files are not accidentally deleted.

## Phase 1: Competitive And Architecture Research Spike

Goal: do not guess the app shell or audio engine path.

This phase runs in parallel with early Track Master prototyping. It should not freeze all product work.

Compare:

- Tauri UI plus Rust native audio engine.
- Tauri UI plus Python/offline engine plus Rust real-time preview.
- JUCE/native app.
- Rust native UI/audio stack.
- Hybrid route where Tauri remains UI and audio/DSP engine becomes native underneath.

Benchmark products:

- Waves Online Mastering: A/B, Volume Match, Add Reference, simple mastering controls.
- iZotope Ozone: assistant-driven mastering, preset/product language, metering.
- Steinberg WaveLab: professional montage, loudness, DDP/export discipline.
- LANDR/eMastered/BandLab: one-click expectations and quick export path.

Research questions:

- What does each product do that validates our desired workflow?
- What should remain private-reference-only if this ever goes public?
- What app shell best supports serious low-latency audio audition?
- Can Tauri plus native Rust audio meet the latency and fidelity bar?
- Does JUCE materially simplify real-time audio, DSP, waveform, and export parity?
- What architecture minimizes rewrite risk while maximizing audio seriousness?

Deliverable:

- `docs/ARCHITECTURE_SPIKE.md`

It must include:

- Options compared.
- Evidence gathered.
- Latency observations.
- Build/package implications.
- Audio quality implications.
- Recommendation.
- Risks.
- What decision remains reversible.

No-victory-lap check:

- The recommendation is evidence-based.
- It does not choose Tauri forever by inertia.
- It does not choose native/JUCE just because it sounds serious.

## Phase 2: Rust/Tauri App Foundation

Goal: make the backend talk in product concepts, not raw CLI arrays.

Current Rust is a useful bridge but too generic. Refactor toward typed commands while preserving working behavior.

Desired modules/responsibilities:

- `engine`: wraps engine commands and sidecar process execution.
- `jobs`: analyze/render job queue, progress, cancel.
- `files`: import validation, source metadata, path safety.
- `project`: autosave, project files, recent sessions.
- `settings`: user presets and settings chains.
- `audio`: playback cache, A/B preview assets, waveform prep.
- `exports`: output versioning and quality-check orchestration.

Typed commands to introduce:

- `analyze_tracks`
- `render_track_preview`
- `render_track_master`
- `render_album_master`
- `prepare_source_playback`
- `prepare_master_playback`
- `prepare_ab_preview`
- `prepare_waveform`
- `run_export_checks`
- `save_project`
- `autosave_session`
- `load_recent_session`
- `save_user_preset`
- `list_user_presets`
- `open_output`

Rules:

- Do not duplicate DSP in Rust just to move code.
- Do move app state, file safety, job control, and playback infrastructure into Rust where it helps.
- Keep the frontend insulated from raw CLI argument construction.

Verification:

- Existing integration tests still pass or are replaced with equivalent typed-command tests.
- Analyze and render still work through the app.
- Cancel/progress still work.
- Dev fallback and release sidecar behavior still work unless intentionally changed with evidence.

## Phase 3: Track Master UI Rebuild

Goal: build the reference-inspired Track Master workstation.

Agents may fully replace the current `desktop/src/App.tsx` structure if that better serves the spec. Preserve useful logic, not the current layout.

Required screen structure:

- Left imported songs rail.
- Main waveform/audition area.
- Play/pause.
- Loop.
- Original/Mastered toggle.
- Optional Volume Match toggle, off by default.
- Preset tile row.
- Intensity control.
- Low/Mid/High EQ.
- Update Preview or live state indicator.
- Export Master button.
- Advanced section collapsed by default.

Initial simple controls:

- Universal.
- Clarity.
- Tape.
- Spatial.
- Oomph.
- Warmth.
- Punch.
- Loud/Energy if needed.

State behavior:

- Import adds tracks quickly.
- Analyze computes values and safe universal settings.
- Export is enabled after analysis.
- Changing controls marks mastered preview stale unless real-time audition is live and export-faithful.
- The UI must not play an old master as if it reflects current controls.

Temporary scaffolding:

- Whole-track preview rendering may be used before real-time audition is solved.
- Region selection can initially control playback/loop while preview remains whole-track.

Verification:

- Drag/drop works.
- Analyze works.
- Whole-track preview works.
- Original/Mastered same-position toggle works.
- Volume Match exists and defaults off.
- Changing a control marks preview stale.
- Export creates a non-overwriting output.

No-victory-lap check:

- A pretty screen is not enough.
- Preset tiles must actually affect audio or be clearly disabled.
- A/B must preserve playhead.
- Stale state must be visible and honest.

## Phase 4: Playback, Waveform, A/B

Goal: make listening reliable enough for musical decisions.

Required:

- Waveform rendering for source and mastered audio.
- Zoom.
- Seek.
- Region selection.
- Region loop.
- Original/Mastered toggle.
- Volume Match optional/off by default.
- Playhead preservation.
- No source file mutation.

Acceptance:

- User can select a chorus, loop it, toggle Original/Mastered, and judge what the app did.
- User can move around the song without exporting.
- Playback controls are responsive and visually obvious.

Native audio requirement:

- Investigate native audio playback/control early.
- Browser audio is allowed for scaffolding but must not be assumed final.
- Serious real-time audition probably needs a native audio layer.

## Phase 5: Real-Time Audition Spike

Goal: prove the app can support responsive controls by ear.

This is mandatory for final Track Master release quality.

Targets:

- Gain, lightweight EQ, width, and Volume Match changes audible in under about 150 ms.
- Heavier macro changes audible in under about 500 ms.
- No obvious clicks, zipper noise, glitches, or unstable playback.
- Preview and export must match in audible intent.

Spike approaches:

- Rust native audio plus DSP subset.
- Web Audio only as a baseline comparison, not assumed final.
- Python process/service if it can meet latency.
- JUCE/native proof if Tauri path struggles.
- Hybrid engine where offline export and realtime preview share the same DSP definitions.

Controls to prove first:

- Gain.
- 3-band EQ.
- Width.
- Volume Match.
- Basic intensity subset.

Then continue toward:

- Full Intensity macro.
- Preset parity.
- Advanced controls.

Deliverable:

- `docs/ENGINE_DECISION_RECORD.md`

It must include:

- Latency measurements.
- CPU/memory observations.
- Fidelity/export parity risks.
- Packaging implications.
- Recommendation: continue Python sidecar, add Rust audio layer, migrate DSP, use JUCE/native, or other.
- What is temporary vs final.

No-victory-lap check:

- Basic real-time controls are a milestone, not the finish line.
- Agents may not stop after one slider works.
- Non-real-time preview is temporary scaffolding, not final quality.

## Phase 6: Track Master Export And Quality Checks

Goal: make export safe, obvious, and honest.

Required:

- One obvious Export Master action.
- Non-overwriting output folder/file.
- Post-render checks.
- Advisory warnings.
- Export Anyway when technically possible.
- Report or compact receipt.
- Open output action.

Quality checks:

- True-peak/ceiling risk.
- Clipping risk.
- Extremely loud/flat warning.
- Codec preview risk when enabled.
- Non-finite analysis guard.
- Source/master sanity comparisons.

Quality language:

- Plain-language.
- No scare warning for MP3/lossy format alone.
- Warn based on measured problems.

Verification:

- Export does not alter source file.
- Export does not overwrite prior render.
- Risky settings produce advisory checks.
- Normal settings can pass quietly.

## Phase 7: Presets, Settings, Autosave, Undo

Goal: make experimentation safe and reusable.

Required:

- Custom user presets/settings chains.
- Shared presets with mode-specific fields.
- Autosave session state.
- Explicit Save Project.
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

## Phase 8: Album Master Near-Term Path

Goal: build the user's required album workflow on the Track Master foundation.

Required:

- Album Master mode.
- Track reorder.
- Analyze sequence.
- Track Roles / Story step.
- Global album intent.
- Per-track adaptation.
- Editable roles/overrides.
- Export Album.
- Individual masters.
- Continuous album WAV by default.
- Preserve original boundaries by default.
- Album dashboard/report.

Track Roles / Story:

- Skippable.
- Visibly reviewable.
- Humble language: likely role, not magical detection.
- Important for wildly varied albums.

No-victory-lap check:

- Album Master is not batch Track Master with a different button.
- It must show sequence/story awareness.
- It must preserve distinct track identities.

## Phase 9: Transition Primitives

Goal: provide reliable album boundary tools before generated musical transitions.

Default:

- Generated transitions off.
- Preserve source boundaries.

Implement primitives:

- Timed gaps.
- Direct boundaries.
- Equal-power crossfades.
- Fade out/in.
- Ring-out.
- Reverse swell only if it sounds useful.

Generated interludes:

- Optional later.
- Must not be default until genuinely good.
- Should not be marketed as core quality until listening tests support it.

## Phase 10: DSP Audit And Modernization

Goal: improve actual mastering quality, not just UI.

Audit:

- LUFS measurement.
- True-peak detection.
- Limiter design.
- EQ/filter phase behavior.
- Compression behavior.
- Saturation.
- Stereo processing.
- Dither.
- SRC.
- Codec preview.
- Preset mappings.

Use research:

- `audio-mastering-technical-research.md`
- `deep-research-report.md`
- `mastering-settings-reference.md`
- `compass_artifact_wf-...markdown.md`
- `docs/research-implementation-notes.md`

Modernization rule:

- Rewrite/migrate DSP when evidence shows better sound, speed, reliability, real-time behavior, or maintainability.
- Do not rewrite only because native code seems prestigious.

## Phase 11: Private Real-Audio Fixtures

Goal: test on music that matters.

Create ignored folder:

```text
private-audio-fixtures/
```

Suggested files:

- One clean full mix.
- One rough/problem track.
- One acoustic/quiet track.
- One heavy/dense track.
- One bass-heavy track if available.
- Two or three adjacent album tracks.
- Eventually the full target album.

Add:

```text
private-audio-fixtures/manifest.json
```

Manifest should describe:

- File path.
- Purpose.
- Safe for quick automated tests: true/false.
- Safe for slow full render tests: true/false.
- Listening notes.
- Known problem areas.

Rules:

- Do not commit private/copyrighted audio.
- Do not snapshot rendered audio into git.
- Automated local tests may use fixtures.
- Manual listening remains required.

## Phase 12: Performance Budgets

Goal: measure rather than guess.

Initial rough targets:

- App launch feels prompt.
- Import does not block UI.
- Analyze progress is visible.
- Waveform appears quickly enough to keep trust.
- Lightweight real-time controls respond under about 150 ms.
- Heavier macro controls respond under about 500 ms.
- Preview/export shows progress and can be canceled when safe.
- 8-track album export gives meaningful progress.

Agents must establish baselines on the actual machine and refine these budgets with evidence.

## Phase 13: Release And Installer Hardening

Goal: make the app usable outside the repo.

Required:

- Installed/release build launches.
- No user-managed Python required unless architecture changes explicitly.
- FFmpeg/FFprobe or replacement audio tooling available to the app.
- Sensible default render folder.
- Open output/report works.
- App handles missing/corrupt files gracefully.
- Sidecar/startup overhead measured.

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
- Use of private research text.
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

- `docs/progress.md`: detailed evidence and session notes.
- `docs/IMPLEMENTATION_PLAN.md`: concise phase status changes or plan changes.
- `docs/PRODUCT.md`: only after human-approved product canon changes.

No phase is complete just because something visually resembles the goal. It must satisfy the relevant product behavior and verification gates.

## Immediate Next Questions For Humans

These are not blockers for creating the plan, but they should be answered before or during early execution:

1. Which architecture spike candidates should be tested first: Tauri+Rust audio, JUCE, or both immediately?
2. What real audio fixtures will be provided first?
3. What is the minimum Track Master feature set required before Album Master begins?
4. Should the current repo remain the Codex path while the new Claude build repo starts from zero?
5. How often should long-running agents update progress: every phase, every day, or every meaningful verified slice?

# Album Mastering Studio Product Canon

Last updated: 2026-05-11

This is the canonical product mission and decision record for Album Mastering Studio. Future agents must read this file before doing product, UI, DSP, architecture, or planning work. If another document conflicts with this one, treat this file as the product source of truth unless the user explicitly changes direction.

This file is not a detailed implementation plan. It captures what the product is, what quality means, what decisions are locked, what remains open, and how long-running agents should keep the work coherent.

## Mission

Album Mastering Studio is a local desktop mastering app for real tracks and real albums. It should be something a musician or producer would be proud to run their audio through.

The product is not a toy normalizer, not a certified mastering lab, and not a replacement for a skilled mastering engineer. It is a serious, standards-aware, listening-first mastering assistant that helps users improve finished mixes, export credible masters, and understand the result.

The quality bar is release-candidate for the user's own albums and for capable musicians/producers who want a trustworthy local workflow. It should be accessible enough that non-musicians can use the safe default path without being walled off by jargon.

## Core Promise

The app should let a user:

1. Drop in ready-made tracks.
2. Analyze them.
3. Get safe universal mastering settings.
4. Preview and compare original vs mastered audio.
5. Adjust simple musical controls if desired.
6. Export without damaging source files.
7. Receive plain-language quality checks and a useful report.

For albums, the app should also help the user:

1. Reorder tracks.
2. Review the album story and track roles.
3. Master each track into a coherent album intent without flattening wildly different material.
4. Export individual masters and a continuous album WAV.
5. Optionally shape boundaries, gaps, crossfades, or generated transitions.
6. Produce cue/split data, manifest, dashboard, and delivery checks.

## Product Bar

Top-tier quality means:

- The installed Tauri desktop app is the primary product surface.
- The app works without repo knowledge, Python knowledge, CLI comfort, or documentation spelunking.
- The fast path is obvious: Drop audio, Analyze, Export.
- The app is honest about local metering and mastering limits.
- The user can listen deeply before trusting an export.
- Source files are never destructively edited.
- Exports never overwrite prior renders by default.
- Weak optional features do not poison the default workflow.
- Reports support confidence, but listening remains central.

The app should not claim certification. Actual certification would require industry-standard validation, external conformance, and formal metering guarantees. The app should instead be transparent: it can get users very close and improve many tracks substantially, but it does not replace expert judgment.

## Target User

Primary user:

- A capable musician or producer on Windows.
- Comfortable making taste decisions.
- Not necessarily technical.

Secondary user:

- A non-musician or less experienced user who wants safe, tested presets and a quick path to better audio.

Implications:

- Plain language beats engineering theater.
- The app should guide workflow without forcing learning up front.
- Warnings should explain what matters and what the user can do.
- The user should be able to ignore advanced controls and still get a useful result.

## Two Product Modes

The app should be organized by intent, not by "basic" and "advanced":

- Track Master
- Album Master

Track Master is built first. Album Master builds on it.

### Track Master

Track Master is for one or more independent songs. Its job is to make each track better on its own.

Default workflow:

1. Drop or add audio.
2. Analyze.
3. Apply safe universal settings.
4. Preview original vs mastered.
5. Export master.

Multiple tracks in Track Master are independent by default. "Apply to all" or batch consistency can exist, but Track Master should not try to turn a list of songs into an album unless the user enters Album Master.

Reference UI direction:

- Left rail with imported songs.
- Large waveform as the main focus.
- Play, pause, seek, and loop controls.
- Original/Mastered toggle at the same playhead.
- Preset tiles.
- Intensity macro.
- Simple Low/Mid/High EQ.
- One obvious export button.

Required Track Master upgrades beyond the reference:

- Waveform zoom.
- Region selection by dragging across the waveform.
- Loop selected region.
- Toggle Original/Mastered while preserving playhead and selection.
- Optional Volume Match toggle, off by default.
- Optional Add Reference.
- Quality check after export.
- Advanced controls tucked away.

Track Master should feel like a fast mastering workstation, not a wizard and not a DAW.

### Album Master

Album Master is for a sequence of songs that should become a coherent record.

Default workflow:

1. Drop or add tracks.
2. Reorder tracks.
3. Analyze.
4. Review inferred album story and track roles.
5. Keep safe album settings or adjust them.
6. Export individual masters plus a continuous album WAV.

Album Master should use one global album intent plus granular per-track adaptation. This matters because the user's album may vary wildly in genre. The app must not flatten acoustic, heavy, transition, and return material into one texture.

Album-only capabilities:

- Track order as story.
- Track Roles / Story step.
- Album arc.
- Per-track role and character overrides.
- Per-track mastering decisions inside an album plan.
- Gap, boundary, and crossfade tools.
- Optional transition generation.
- Continuous album WAV.
- Cue/split data.
- Album dashboard.
- Album-level quality checks.

The Track Roles / Story step should be skippable but visibly reviewable. After analysis, the app can show inferred roles already filled in, but the user should be able to export without editing. The UI should encourage review when the album changes style or mood sharply.

## Universal-First Workflow

The default path is not "choose a genre preset before you can continue."

Locked workflow principle:

Drop audio -> Analyze -> Universal safe settings -> Export

On import, the app should add files quickly. It should not run expensive full analysis silently unless there is a clear background design later.

On Analyze, the app should compute useful audio values and prepare safe universal settings. Preset recommendations can exist, but they are optional and must not be mandatory.

Recommendation language:

- Use "recommended from analysis" or "likely fit."
- Do not say "detected genre" as if the app has magical certainty.
- Use confidence labels only when making a recommendation: Strong, Moderate, Unsure.
- If confidence is low, stay with Universal.

Universal is a confident default, not a weak fallback.

## Preset Model

The main preset row should stay limited to about 6 to 8 choices.

Preferred main preset vocabulary:

- Universal: well-rounded safe default.
- Clarity: upper-mid/high detail, vocal intelligibility, definition.
- Tape: saturation, glue, softened top, fuller low-mid body.
- Spatial: width/depth enhancement and careful stereo shaping.
- Oomph: low-end weight and punch for bass-forward material.
- Warmth: fuller, smoother, less harsh, musical low-mid and softer top.
- Punch: transient impact, drums/guitars forward.
- Loud or Energy: more density and level, with safety checks.

The visible names should be short and engineering-adjacent. Detailed descriptions can appear below, on hover, or in an expanded detail panel.

Avoid making the main row overly genre-specific. Genre-specific or specialty presets can live in a later "More presets" drawer.

Future specialty preset examples:

- Acoustic Natural.
- Heavy Rock / Metal.
- Djent / Modern Metal.
- Bright / Air.
- Dark / Smooth.
- CD-safe.
- Vinyl premaster.
- Platform or delivery-specific profiles.

Recommendations can point into the specialty drawer eventually, but the core row stays fast and scannable.

## Simple Controls

The first-layer Track Master controls are:

- Universal/preset tiles.
- Intensity.
- Low/Mid/High EQ.
- Original/Mastered toggle.
- Loop.
- Export.

Everything else belongs in an expandable advanced section:

- LUFS offset.
- Ceiling.
- Width.
- Warmth.
- Presence/air.
- Compression/density.
- Limiter behavior.
- Bit depth.
- Delivery profile.
- Codec QC.
- Reference details.

The advanced section should be powerful, but the top surface should remain usable in 30 seconds.

## Intensity

Intensity is a macro, not a volume knob.

It should change how hard the preset works across multiple parameters:

- Loudness push.
- Compression density.
- Saturation/warmth.
- Transient shaping.
- Width or brightness when the preset calls for it.

Intensity should not simply make the output louder. If it does, A/B comparison becomes misleading.

## A/B And Listening

Listening workflow is central to the product.

Required behavior:

- Compare Original and Mastered at the same playhead.
- Preserve playhead when toggling.
- Support selected region loop.
- Support waveform zoom.
- Let users inspect specific sections.

Volume Match:

- Optional.
- Off by default.
- Export level is unchanged.
- Tooltip should explain: "Aligns playback loudness for fair tone comparison. Export level is unchanged."

The default audition should represent the real exported result. If mastering made the track louder by design, the user should hear that by default. Volume Match is for judging tone and punch without the loudness advantage.

Reference track:

- Optional.
- Visible enough to discover.
- Non-blocking.
- Useful for comparison now and possible reference matching later.

## Source Formats

The app should be format-neutral by default.

Do not warn just because a file is MP3, M4A, AAC, Opus, OGG, or another lossy format. The user may only have that file, and the app should not make the import feel second-class.

Mastering logic should respond mostly to measured audio values:

- Loudness.
- Peaks.
- Dynamics.
- Spectral balance.
- Stereo width.
- Transient density.
- Harshness or artifact-like measured issues.
- Clipping or technical decode problems.

If a file is unreadable or corrupt, that is a hard stop. If a file is lossy but workable, process it.

The app can quietly record source format in details or reports. It should show plain-language notes only for actual measured problems, not for the file extension alone.

## Export Behavior

The app should provide one obvious export action with smart defaults.

In Track Master:

- One track: Export Master.
- Multiple tracks: Export mastered songs independently.

In Album Master:

- Export Album.
- Produce individual masters.
- Produce a continuous album WAV by default.
- Produce manifest/report/dashboard.
- Produce cue/split data when appropriate.
- Preserve normal boundaries unless the user opts into gaps, crossfades, or transitions.

Export should be allowed immediately after analysis. Preview/listening should be strongly encouraged but not required.

Post-render quality checks are required for every export.

Quality checks are advisory, not blocking:

- If clipping or true-peak risk is detected, explain it.
- If the master is extremely loud or flat, explain it.
- If a codec preview suggests clipping risk, explain it.
- If measurable checks suggest the output may be worse, ask for review.
- Allow Export Anyway when technically possible.

The app should not silently hand the user a bad master with a proud face.

## Output Safety

Locked rules:

- Never destructively edit source files.
- Never overwrite previous exports by default.
- Use timestamped or versioned output folders/files.
- Rendered files do not need undo because they can be regenerated.
- User settings and projects do need undo and autosave protection.

Render history can come later. Early product should support saving settings/custom presets before building a full render library.

## Projects, Autosave, Undo

The app should quietly autosave session/project state and also support explicit Save Project.

Undo/redo is required for non-destructive user state:

- Ctrl+Z = undo.
- Ctrl+Shift+Z = redo.
- Keep enough history for real experimentation.
- Cover presets, intensity, EQ, tuning, track order, roles, transitions, metadata, and settings.

Undo does not need to delete rendered files.

## Reports And Dashboard

Reports are a core confidence layer, not the main experience.

Reports should answer:

- What changed?
- What settings were used?
- Were there technical risks?
- Where are the files?
- What did the track or album measure after export?
- What warnings or codec issues were found?

Reports should not clutter the main mastering screen or replace playback judgment.

## Transitions And Boundaries

Generated transitions are off by default.

The release-candidate album workflow should prioritize reliable transition primitives before ambitious generated interludes:

- Preserve original boundaries by default.
- Direct boundaries.
- Timed gaps.
- Equal-power crossfades.
- Fades.
- Ring-outs.
- Possibly reverse swell as an optional creative primitive.

Generated musical interludes can become better over time, but they should not be central until they sound genuinely good.

Album Master should still create a continuous album WAV by default when there are multiple tracks, even if generated transitions are off.

## Not A DAW

The app is not a general audio editor for now.

Out of scope for the release-candidate direction:

- Cutting tracks.
- Moving clips on a timeline.
- Destructive trimming.
- Multitrack arrangement.
- Full DAW-style editing.

In scope because it supports mastering:

- Waveform zoom.
- Region selection.
- Loop selected region.
- Source/Master toggle.
- Jump to loudest, quietest, intro, outro, or high-energy sections.

The right boundary is: bring ready-made tracks, then master and audition them deeply.

## Architecture Direction

The Tauri desktop app is the primary product.

The existing app proved important plumbing:

- Python mastering engine.
- Tauri shell.
- Rust subprocess bridge.
- Sidecar packaging.
- FFmpeg resources.
- Playback cache.
- Reports and smoke tests.

The next product surface can start over cleanly on the frontend while preserving useful backend and engine work.

### Rust

Rust/Tauri should become the desktop application platform, not a stitched-on command bridge.

Rust should own:

- App shell reliability.
- Typed product commands.
- File import and metadata.
- Project state and autosave.
- User presets/settings storage.
- Render/analyze job control.
- Progress/cancel handling.
- Playback cache and A/B preview assets.
- Output folder versioning.
- Post-render checks orchestration.

The frontend should talk to product concepts, not raw CLI arrays.

Desired typed commands include:

- analyze_tracks.
- render_track_master.
- render_album_master.
- prepare_ab_preview.
- prepare_waveform.
- save_project.
- autosave_session.
- save_user_preset.
- run_export_checks.
- open_output.

### Python Engine

The current Python CLI/sidecar is the incumbent working engine. It should remain useful while product work proceeds.

Python is strong for:

- Existing DSP implementation.
- NumPy/SciPy iteration.
- Current tests and smoke renders.
- Fast algorithm experimentation.
- PyInstaller sidecar packaging.

Python is not guaranteed forever. It should not be treated as a sacred permanent boundary.

### Engine Modernization

Engine Modernization is a first-class workstream.

DSP should be rewritten or moved into Rust/C++/native audio libraries when that demonstrably improves:

- Sound quality.
- Metering correctness.
- Render speed.
- Real-time audition.
- Startup time.
- Reliability.
- Maintainability.
- Native playback/control.

Do not rewrite DSP merely because native code feels more serious. Language choice does not make a better master by itself. Better audio comes from better algorithms, metering, calibration, listening tests, and workflow.

## Deep Research Inputs

The deep research should be used after the product structure is clear and during DSP/preset calibration.

Known research inputs:

- audio-mastering-technical-research.md
- deep-research-report.md
- compass_artifact_wf-0dd25647-771b-4682-8d9d-4d900af5f667_text_markdown.md
- docs/research-implementation-notes.md

Use these to inform:

- Delivery profiles.
- LUFS and true-peak behavior.
- Dither and bit depth.
- Codec QC.
- Preset behavior.
- Signal-chain order.
- Limiter requirements.
- Reference matching.
- Album-mode loudness.
- Platform export choices.

Do not blindly implement research claims without checking them against current code, scope, and product goals. The research is a technical input, not a replacement for product judgment.

Already implemented according to current notes:

- Delivery profiles.
- Integer/dithered WAV exports.
- Codec QC.
- Cue sheets.
- Metadata.
- Normalization preview.
- Richer metering.
- Report/dashboard improvements.

Still important to revisit:

- BS.1770-compliant loudness.
- True-peak measurement and oversampled limiter behavior.
- Minimum-phase or otherwise mastering-appropriate filter choices.
- Preset calibration.
- Reference matching.
- Real-time audition engine.
- Native engine opportunities.

## Workstreams

Future work should be organized into these streams:

1. Product canon and planning.
2. Track Master UI rebuild.
3. Rust/Tauri app foundation.
4. Track Master listening workflow.
5. Export and quality checks.
6. Presets and user settings.
7. Album Master workflow.
8. Album story/roles and per-track adaptation.
9. Transition primitives.
10. DSP audit and correctness.
11. Engine modernization.
12. Deep research application.
13. Verification, installer, and real-audio testing.

Track Master should come first because it proves the core quality loop:

Drop -> Analyze -> Universal settings -> Waveform audition -> Original/Mastered -> Tune -> Export -> Quality check

Album Master should reuse that foundation and add sequence-specific depth.

## Verification Gates

Do not claim readiness without fresh verification.

Baseline code gates:

```powershell
python -m compileall -q src tests
python -m unittest discover -s tests
python -m album_mastering_studio.cli smoke --output test-output\codex-product-smoke
cd desktop
npm run build
npm run test:integration
```

Release/package gate when relevant:

```powershell
cd desktop
npm run build:sidecars
& cmd.exe /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 && set "PATH=%USERPROFILE%\.cargo\bin;%PATH%" && npm run tauri:build'
```

Product gates:

- App launches as installed or release build.
- Drag/drop works.
- Analyze works.
- Waveform renders.
- Source/Mastered playback works.
- Same-position A/B works.
- Region loop works when implemented.
- Export creates new non-overwriting output.
- Quality checks run.
- Report opens.
- No source file is modified.
- Real user audio can be mastered and auditioned.

Album gates:

- Track reorder works.
- Roles/story step appears after analysis.
- Per-track overrides persist.
- Continuous album WAV exists.
- Individual masters exist.
- Cue/split outputs exist when expected.
- Generated transitions remain off by default.
- Boundary/gap behavior preserves source intent unless changed.

DSP gates:

- LUFS behavior is honest and documented.
- True-peak/ceiling behavior is tested.
- Limiter does not globally destroy quiet material because of isolated spikes.
- Codec preview warnings are meaningful.
- No NaN/inf in analysis or output manifests.
- Preset changes produce expected audible/measurable direction.

Human listening gates:

- A/B does not lose playhead.
- Volume Match off by default.
- Volume Match on is clearly labeled and does not change export level.
- Universal preset improves or at least does not obviously harm representative real tracks.
- Aggressive settings warn when they create obvious risk.

## Agent Loop Rules

Every future agent should:

1. Read this file before changing product behavior.
2. Read AGENTS.md for repo rules.
3. Check docs/codex-active-handoff.md and docs/progress.md for current status.
4. Inspect current code before assuming a decision is implemented.
5. Work in one clear workstream at a time unless the user asks for parallel work.
6. Preserve user changes and do not reset the repo.
7. Prefer product-complete vertical slices over isolated refactors.
8. Add tests for behaviorally testable changes.
9. Run appropriate verification before claiming completion.
10. Update docs/progress.md after meaningful work.
11. Update this file only when product decisions change or new canonical decisions are made.

## Codex Goal Mode

Codex `/goal` can be useful for long-running work after this product canon and a concrete implementation plan exist.

Current verified docs describe `/goal` as an experimental long-running objective mode:

- https://developers.openai.com/codex/use-cases/follow-goals
- https://developers.openai.com/codex/cli/slash-commands

Use `/goal` only when:

- The objective is bigger than one prompt but smaller than an open-ended backlog.
- The stopping condition is verifiable.
- The agent knows which files to read first.
- The agent knows which commands prove progress.
- The agent is told when to pause or stop.

Do not use `/goal` as a substitute for a plan. Use it to execute a sharp plan.

## Locked Decisions From The 2026-05-11 Grill Session

1. The installed Tauri desktop app is the primary product surface.
2. The quality bar is release-candidate for real personal albums, not certified mastering replacement.
3. The target operator is a capable musician/producer, with non-technical accessibility.
4. The default workflow is Universal-first, not forced preset-first.
5. Preset recommendations are optional and should use humble confidence language.
6. The app supports both Track Master and Album Master.
7. Track Master and Album Master are separate intent-based modes.
8. Track Master should be rebuilt first.
9. The frontend/product surface can be substantially rebuilt from clean.
10. The Rust/Tauri layer should become app groundwork with typed commands.
11. The Python engine remains useful but is not assumed permanent.
12. Engine Modernization is a first-class track.
13. The app should be format-neutral and not warn just because input is MP3/lossy.
14. File decisions should mostly respond to measured audio issues.
15. One obvious export action should exist, with smart defaults by mode.
16. Track Master uses the reference-style layout: song list, waveform, A/B, presets, intensity, EQ, export.
17. Track Master simple controls are preset tiles, intensity, and 3-band EQ.
18. Advanced controls should be expandable, not front-loaded.
19. Intensity is a macro over mastering behavior, not just volume.
20. Every export gets post-render quality checks.
21. Quality checks are advisory and should allow Export Anyway when possible.
22. Volume Match is optional and off by default.
23. Original/Mastered audition defaults to real output levels.
24. Add Reference is optional and non-blocking.
25. Export is allowed immediately after analysis, with preview strongly encouraged.
26. Track Master processes multiple imported songs independently by default.
27. Apply-to-all or batch consistency is optional in Track Master.
28. Album Master uses global album intent plus per-track adaptation.
29. Album Master needs granular control for wildly varied albums.
30. Album Master gets a Track Roles / Story step after analysis.
31. The story step is skippable but visibly reviewable.
32. Generated transitions are off by default.
33. Transition work should prioritize reliable primitives before musical generated interludes.
34. Album Master preserves original boundaries by default.
35. Album Master exports a continuous album WAV by default for multiple tracks.
36. Reports are core confidence layers but secondary to listening.
37. The app should autosave state and also support explicit Save Project.
38. Undo/redo is required for non-destructive state, with Ctrl+Z and Ctrl+Shift+Z.
39. Exports never overwrite previous outputs by default.
40. Render history/library is later; saving settings/custom presets comes earlier.
41. Users should be able to save custom settings chains.
42. Custom presets should be shared across the app, with mode-specific fields.
43. Preset vocabulary should be short, engineering-adjacent, and limited to about 6 to 8 main choices.
44. A specialty preset drawer can come later.
45. The app is for ready-made tracks; it should not become a full audio editor/DAW in the release-candidate path.
46. Waveform zoom, region selection, loop, and A/B are core mastering audition features.
47. docs/PRODUCT.md is the canonical compaction-proof mission and decision record.

## Coverage Audit

This session covered:

- Product mission.
- Quality bar.
- Target user.
- Main workflows.
- Track Master vs Album Master.
- Universal-first behavior.
- Preset philosophy.
- Simple vs advanced controls.
- A/B behavior.
- Volume Match behavior.
- Reference behavior.
- Export behavior.
- Source format treatment.
- Output safety.
- Undo/redo.
- Autosave.
- Reports.
- Transitions.
- Album story/roles.
- Frontend rebuild direction.
- Rust app foundation.
- Python engine boundary.
- Engine modernization.
- Deep research usage.
- Long-running agent loop process.

Still not solved and should be handled in later planning:

- Exact screen layouts and component hierarchy.
- Exact visual design system.
- Exact Rust command schema.
- Exact state model for autosave and undo.
- Exact Track Master implementation milestones.
- Exact Album Master implementation milestones.
- Exact DSP changes and numeric preset mappings.
- Exact loudness/true-peak conformance strategy.
- Exact reference matching algorithm.
- Exact transition primitive UI.
- Exact test plan for real audio fixtures.
- Whether to adopt native DSP libraries, and which ones.
- Whether to migrate parts of the engine to Rust/C++ and in what order.
- Installer and distribution polish beyond current build proof.

Do not treat the unsolved items as permission to drift. They are planning targets under the mission above.

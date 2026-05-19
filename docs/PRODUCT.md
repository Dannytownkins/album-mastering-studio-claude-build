# YES Master Product Canon

Last updated: 2026-05-19

This file is the canonical product mission and locked decisions for YES Master. Future agents must read this before doing product, UX, DSP, or planning work. If another document conflicts with this one, treat this file as the product source of truth.

This file is intent, not a build status report. Implementation status lives in `docs/HANDOFF.md` and `docs/progress.md`. Architecture decisions live in `docs/adr/`. Do not treat features described here as built; check the code or the handoff before claiming a feature exists.

This file changes only through deliberate grill sessions with the user, not through incidental edits made during implementation work. If a decision needs revisiting, raise it explicitly and update this file as part of the same session.

## Mission

YES Master is a local desktop mastering app for real tracks and real albums. It should be something a musician or producer would be proud to run their audio through.

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
5. Optionally shape boundaries with reliable primitives (gaps, crossfades, fades, ring-outs).
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

- A capable musician or producer working locally on a desktop machine.
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
- Gap, boundary, and crossfade tools (see Transitions And Boundaries).
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
- Preserve normal boundaries unless the user opts into gaps or crossfades.

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

Album Master supports a small set of reliable transition primitives between tracks. Generated transitions are off by default; AI-synthesized musical interludes are explicitly out of scope.

Supported primitives:

- Preserve original boundaries (default).
- Direct boundary (no processing).
- Timed gap.
- Equal-power crossfade.
- Fade in / fade out.
- Ring-out preservation.

AI-generated transitions are not a feature of YES Master. The album workflow does not try to invent new audio between tracks. If a user wants a transition that does not fit a primitive, they handle it in a DAW before importing.

Album Master still produces a continuous album WAV by default when there are multiple tracks, even when no transitions are applied. The continuous WAV is the masters concatenated with whatever boundary treatment the user chose.

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

YES Master is a local desktop application. Whatever stack it runs on must meet these platform requirements:

- Low-latency audition with same-playhead Original/Mastered switching.
- Native audio I/O (file decode, device output) that does not depend on browser audio APIs.
- Deterministic export rendering that is byte-stable for identical inputs and settings.
- Offline-first operation; no required cloud calls for core mastering, audition, or export.
- Desktop packaging that installs and launches without separate runtimes for the end user.
- Project state, autosave, and undo/redo storage that survives crashes and updates.
- Testable DSP — algorithms must be reachable from automated tests, not only the UI.
- A typed command surface so the frontend talks to product concepts, not raw CLI arrays.

The actual platform lock and the reasoning behind it live in `docs/adr/0001-tauri-rust-stack.md`. PRODUCT.md is intentionally tech-agnostic about the platform; the ADR is where platform choices get debated and recorded.

Desired typed command surface includes:

- analyze_tracks.
- render_track_master.
- render_album_master.
- prepare_waveform.
- save_project.
- autosave_session.
- save_user_preset.
- run_export_checks.
- open_output.

## DSP Correctness Commitments

These are the audio-quality dimensions the product is committed to. They are derived from the research in `docs/research/` (use those files as the technical reference when implementing). They are not features; they are the floor that any feature must clear.

1. **BS.1770-5 compliant loudness measurement.** K-weighted, 400 ms gated blocks with prescribed absolute/relative gates. Report integrated, short-term, and momentary as appropriate. Sample-rate-aware coefficients (not blind 48 kHz reuse).
2. **Oversampled true-peak measurement at ≥4×.** True-peak is what the platform sees; sample peak is not enough.
3. **Inter-sample-peak awareness.** Default delivery headroom respects platform conventions (e.g., −1 dBTP universal, −2 dBTP for loud masters destined for lossy codecs). The app should warn when a master sits closer to the ceiling than its target codec can safely reconstruct.
4. **Mastering-grade limiter.** Lookahead plus oversampled ISP detection. Quiet material is not destroyed by isolated transient spikes.
5. **TPDF dither at the final bit-depth reduction, applied exactly once.** Optional noise shaping. No double-dithering.
6. **High-quality polyphase sample-rate conversion** when SRC is needed.
7. **Canonical signal chain order** as the default: corrective EQ → compression → saturation → multiband/MS → tonal EQ → limiter → dither. Deviations are conscious choices, not accidents.
8. **Mastering-appropriate filter choices.** Minimum-phase where transient integrity matters; linear-phase only where pre-ring is acceptable.
9. **Preset calibration grounded in real corpus data**, not first-principle guesses.
10. **Reference matching as spectral subtraction toward a target curve**, not blind copying of a reference track's spectrum.

These commitments are honest about cost: they describe what correct DSP looks like, not what the app necessarily ships on day one. Features may land before they fully clear each commitment, but the gap must be tracked and closed before claiming release-candidate quality.

## Acceptance Criteria

What counts as the product working. These are behavior gates, not test commands. The actual test commands live in `CLAUDE.md`.

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
2. Read `CLAUDE.md` for repo rules and the fast/slow test-lane workflow.
3. Check `docs/HANDOFF.md` and `docs/progress.md` for current implementation status.
4. Inspect current code before assuming a decision is implemented.
5. Work in one clear workstream at a time unless the user asks for parallel work.
6. Preserve user changes and do not reset the repo.
7. Prefer product-complete vertical slices over isolated refactors.
8. Every behavioral fix ships with an automated repro test. Listening sessions are batched, not per-commit.
9. Run the appropriate test lane before claiming completion.
10. Update `docs/progress.md` after meaningful work; write a handoff doc at the end of a session.
11. Update this file only when product decisions change in a deliberate grill session, never as an incidental edit.

## Locked Decisions

These are the product decisions that have been deliberately resolved. Future agents may implement against them without asking. Revisions only happen in an explicit grill session with the user.

From the 2026-05-11 grill session:

1. The installed Tauri desktop app is the primary product surface.
2. The quality bar is release-candidate for real personal albums, not certified mastering replacement.
3. The target operator is a capable musician/producer, with non-technical accessibility.
4. The default workflow is Universal-first, not forced preset-first.
5. Preset recommendations are optional and should use humble confidence language.
6. The app supports both Track Master and Album Master.
7. Track Master and Album Master are separate intent-based modes.
8. Track Master is the first product surface to ship.
9. The Rust audio engine owns DSP, audition, and export; see `docs/adr/0001-tauri-rust-stack.md`.
10. The app is format-neutral and does not warn just because input is MP3/lossy.
11. File decisions respond mostly to measured audio issues.
12. One obvious export action exists, with smart defaults by mode.
13. Track Master uses the reference-style layout: song list, waveform, A/B, presets, intensity, EQ, export.
14. Track Master simple controls are preset tiles, intensity, and 3-band EQ.
15. Advanced controls are expandable, not front-loaded.
16. Intensity is a macro over mastering behavior, not just volume.
17. Every export gets post-render quality checks.
18. Quality checks are advisory and allow Export Anyway when possible.
19. Volume Match is optional and off by default.
20. Original/Mastered audition defaults to real output levels.
21. Add Reference is optional and non-blocking.
22. Export is allowed immediately after analysis, with preview strongly encouraged.
23. Track Master processes multiple imported songs independently by default.
24. Apply-to-all or batch consistency is optional in Track Master.
25. Album Master uses global album intent plus per-track adaptation.
26. Album Master needs granular control for wildly varied albums.
27. Album Master gets a Track Roles / Story step after analysis.
28. The story step is skippable but visibly reviewable.
29. Album Master preserves original boundaries by default.
30. Album Master exports a continuous album WAV by default for multiple tracks.
31. Reports are core confidence layers but secondary to listening.
32. The app autosaves state and supports explicit Save Project.
33. Undo/redo is required for non-destructive state, with Ctrl+Z and Ctrl+Shift+Z.
34. Exports never overwrite previous outputs by default.
35. Render history/library is later; saving settings/custom presets comes earlier.
36. Users can save custom settings chains.
37. Custom presets are shared across the app, with mode-specific fields.
38. Preset vocabulary is short, engineering-adjacent, and limited to about 6 to 8 main choices.
39. A specialty preset drawer can come later.
40. The app is for ready-made tracks; it does not become a full audio editor/DAW in the release-candidate path.
41. Waveform zoom, region selection, loop, and A/B are core mastering audition features.

Added in the 2026-05-15 grill session:

42. AI-generated musical transitions between album tracks are explicitly out of scope. Album transitions use the locked primitive set only (preserve / direct / gap / equal-power crossfade / fade / ring-out).
43. PRODUCT.md is intent, not a build status report. Status lives in `docs/HANDOFF.md` and `docs/progress.md`; architecture decisions live in `docs/adr/`. PRODUCT.md changes only through deliberate grill sessions with the user.
44. PRODUCT.md is tech-agnostic about platform requirements. The platform lock and its rationale live in `docs/adr/0001-tauri-rust-stack.md`.
45. The DSP correctness commitments in this file (BS.1770-5 LUFS, ≥4× oversampled true-peak, ISP awareness, lookahead+oversampled limiter, TPDF dither once at final reduction, polyphase SRC, canonical chain order, mastering-appropriate filter choices, corpus-grounded preset calibration, spectral-subtraction reference matching) are the audio-quality floor for release-candidate features.
46. The verification gates section is renamed Acceptance Criteria. It describes what counts as the product working; the test commands live in `CLAUDE.md`.
47. Every behavioral fix ships with an automated repro test. Listening sessions are batched, not per-commit.

## Still Open

Genuinely undecided product questions that should be resolved in future grill sessions, not assumed:

- Exact loudness and true-peak conformance defaults: which delivery profile ships as the default, and what its target LUFS / dBTP ceiling values are.
- Reference matching algorithm specifics: how to weight the reference's spectrum vs the source's existing balance, how to handle large genre mismatches, when to refuse.
- Album story / roles UX: how the inferred roles are presented, edited, and rolled into per-track adaptation.
- Codec QC scope: which codecs to preview against by default (AAC, Opus, MP3), what warnings to raise, and how the user resolves them without leaving the app.
- Installer and distribution polish: signing, auto-update, icon and brand polish, first-run experience.
- Preset numeric mappings: the actual coefficient values for each preset × intensity, calibrated against real corpus data rather than guessed.

These are planning targets under the mission above, not permission to drift from the locked decisions.

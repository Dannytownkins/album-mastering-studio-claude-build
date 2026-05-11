# Research Implementation Notes

Last updated: 2026-05-10

Two external deep-research reports were reviewed:

- `compass_artifact_wf-0dd25647-771b-4682-8d9d-4d900af5f667_text_markdown.md`
- `deep-research-report.md`

## Agreed Points Treated As Product Direction

- The app should be an album-finishing workstation, not a generic single-track auto-masterer.
- Deterministic local DSP should stay the core path. AI transition generation is optional and replaceable later.
- Album-mode loudness matters: preserve intentional track-to-track relationships instead of forcing every song to the same integrated LUFS.
- Streaming-safe defaults should include roughly `-14 LUFS` and `-1 dBTP`, with more conservative `-2 dBTP` speaker/lossy safety available.
- Final deliverables should be integer PCM WAV/FLAC style masters, not accidental 32-bit float WAVs.
- Dither belongs only at final integer-depth export, not inside the processing chain.
- Codec round-trip checks matter because lossy encoders can create clipping even when the source WAV looks safe.
- Continuous album exports should include sample-accurate cue/split data.
- Release metadata belongs in project state and render manifests, even if source files are never mutated.
- Reports should be honest about local metering proxies and should show warnings, codec risk, delivery settings, and output paths.

## Implemented From The Research

- Added standards-oriented delivery profiles in `standards.py`.
- Added GUI/CLI support for delivery profile, bit depth, and codec QC preview settings.
- Switched WAV export to deterministic dithered 24-bit or 16-bit PCM by default, with 32-bit float still available when explicitly selected.
- Added short-term loudness maximum and LRA-style loudness range proxy fields to analysis.
- Added sample-accurate `album_sequence.cue` and `album_sequence.cue.json` outputs for continuous album renders.
- Added AAC and Opus round-trip codec QC preview renders for full-album WAV renders when enabled.
- Added release metadata fields for artist, album artist, genre, year, UPC, per-track artist, and ISRC.
- Added delivery profile, normalization preview, codec QC, metadata, cue paths, and richer metering to manifest/dashboard outputs.
- Added regression coverage and smoke checks for cue sheets, codec preview records, dithered 24-bit WAV output, metadata preservation, and richer analysis fields.

## Deferred On Purpose

- Full JUCE/C++ rewrite: sensible long-term product direction, but too disruptive for the current Python personal studio.
- Plugin hosting through VST3/AU/AAX: valuable later, but a licensing/support burden and not needed for first serious personal runs.
- Demucs/stem separation: useful for future stem-assisted transitions, but should be optional and cached when added.
- Cloud or local generative music providers: keep behind an adapter later; do not make album rendering depend on provider availability, cost, or rights.
- Full key/Camelot/tempo MIR through Essentia/madmom/Rubber Band: likely worth adding, but needs optional dependency handling and listening verification.
- True metadata embedding into every audio container: project/manifest/dashboard metadata is now present; container-specific tagging can come after export behavior stabilizes.
- Full reference matching: current reference support is analysis/reporting only. Actual tonal matching needs a separate careful pass.

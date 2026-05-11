# Claude Build Instructions

This is a from-zero build repo for Album Mastering Studio. Do not import source code from the existing Codex repo unless the user explicitly asks for it.

This repo is meant to be an independent parallel build. Do not treat Codex's implementation plan as the default path, and do not look at Codex reference docs unless the user explicitly asks or a specific historical detail is required.

## Required Reading

Before planning or coding:

1. Read `docs/PRODUCT.md`.
2. Read `docs/CLAUDE_BUILD_BRIEF.md`.
3. Read `docs/CLAUDE_WORK_LOOP.md`.
4. Read `docs/PARALLEL_BUILD_NOTES.md`.
5. Read `docs/PRIVATE_AUDIO_FIXTURES.md` before using real audio.
6. Skim `docs/research/README.md`.

Do not read `docs/reference/` by default. Those files are optional Codex-path context, not startup reading.

## Product Non-Negotiables

- Private Windows desktop mastering app.
- Track Master first, Album Master near-term.
- Universal-first workflow: drop audio, analyze, safe settings, preview, export.
- Real-time or near-real-time audition is required for final Track Master quality.
- Native audio should be treated seriously; do not assume browser audio is enough.
- Original/Mastered toggle must preserve playhead.
- Volume Match is optional and off by default.
- Waveform zoom, region selection, and loop are core audition features.
- Source files are never destructively modified.
- Exports never overwrite by default.
- Generated transitions are off by default.
- Reports are confidence layers, not the main experience.
- Core processing must work local/offline by default.

## Architecture Guidance

Start from product/audio requirements, not from framework convenience.

Allowed directions include:

- Tauri plus native Rust audio/DSP.
- JUCE/native app.
- Rust-native UI/audio.
- Hybrid desktop shell plus native audio engine.
- Python or other high-level DSP only if it can meet product needs or is clearly limited to offline/export work.

Do not choose a framework without documenting why it can meet:

- Low-latency audition.
- Export parity.
- Offline rendering quality.
- Windows packaging.
- File/project safety.
- Testability.

## Working Style

- Build vertical slices, not isolated demos.
- Keep docs updated after meaningful verified work.
- Add tests or smoke checks when behavior is testable.
- Use private fixtures locally, but do not commit audio.
- Be honest about partial features. Do not call a phase complete because the UI resembles the goal.

# Album Mastering Studio Claude Build

This repo is a clean, source-free build space for Claude Code to take a from-zero shot at Album Mastering Studio.

It intentionally starts with product canon, research, and build guidance, not inherited implementation code. The goal is not to clone the existing Codex/Tauri/Python repo. The goal is to build the best private Windows desktop mastering app that satisfies the product mission.

This is a parallel independent build, not a competition brief and not a porting task. Claude should reason from the product canon, research, and its own architecture evidence.

## Start Here

1. Read `docs/PRODUCT.md`.
2. Read `CLAUDE.md`.
3. Read `docs/CLAUDE_BUILD_BRIEF.md`.
4. Read `docs/CLAUDE_WORK_LOOP.md`.
5. Read `docs/PARALLEL_BUILD_NOTES.md`.
6. Read `docs/PRIVATE_AUDIO_FIXTURES.md` before using real audio.
7. Use the research files in `docs/research/` when making mastering, DSP, metering, preset, delivery, or architecture decisions.

Do not read the Codex reference docs by default. They are available only if the user explicitly asks for Codex context or Claude needs a specific historical detail.

## Product Shape

The app has two intended modes:

- Track Master: fast mastering for one or more independent songs.
- Album Master: album-aware mastering for ordered records, with per-track adaptation and continuous album export.

Track Master should be built first, but Album Master is required near-term.

## Architecture Stance

No framework is forced at repo creation.

Claude should choose an architecture based on audio seriousness:

- Native or near-native audio audition is required for final Track Master quality.
- Real-time or near-real-time controls are required.
- Tauri, JUCE, Rust-native, C++/native, or hybrid architectures are acceptable if justified.
- Core processing must remain local/offline by default.
- Source audio must never be destructively modified.

## Private Audio Fixtures

Do not commit private audio. Use an ignored folder:

```text
private-audio-fixtures/
```

Add a local `manifest.json` there to describe fixture purpose, quick/slow test suitability, and listening notes.

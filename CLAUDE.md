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

## Test workflow — fast / slow lanes

The Rust suite is split so the daily path stays fast and the slow
real-audio fixture tests only run when explicitly opted in.

**Fast lane (default — under 30 s):**

```powershell
# From repo root or src-tauri/
cargo test --lib       # ~1 s, lib unit tests only
cargo test             # ~15-25 s, full suite with real-fixture tests skipped
```

The four real-fixture tests in `src-tauri/tests/contracts.rs`
(`analyze_tracks_runs_against_real_fixture_if_present`,
`mastering_render_processes_real_fixture_if_present`,
`decode_real_fixture_if_present`,
`phase_12_1_real_fixture_metering_snapshot`) print a skip line and
return early unless the env var below is set.

**Slow lane (migration / pre-merge gating — ~4 minutes):**

```powershell
$env:AMS_RUN_REAL_FIXTURE = "1"
cargo test
```

Or one-shot:

```powershell
$env:AMS_RUN_REAL_FIXTURE = "1"; cargo test; Remove-Item Env:\AMS_RUN_REAL_FIXTURE
```

The slow lane requires `private-audio-fixtures/<some-audio-file>` to
exist; without a fixture the tests still skip even with the env var
set. Run the slow lane before merging changes that touch the DSP
chain, the WAV writer, the LUFS landing math, or anything else where
audio-output byte-identity matters.

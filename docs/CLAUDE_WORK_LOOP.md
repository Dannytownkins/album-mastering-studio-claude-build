# Claude Work Loop

Last updated: 2026-05-11

This repo is intentionally a clean build from zero. The loop below should keep Claude thorough without forcing it to copy the Codex implementation.

## Before Any Build Work

Claude should read:

1. `docs/PRODUCT.md`
2. `CLAUDE.md`
3. `docs/CLAUDE_BUILD_BRIEF.md`
4. `docs/PARALLEL_BUILD_NOTES.md`
5. `docs/PRIVATE_AUDIO_FIXTURES.md`
6. `docs/research/README.md`
7. The relevant research files for the decision at hand

Optional context:

- `docs/reference/codex-implementation-plan.md`
- `docs/reference/codex-research-implementation-notes.md`

The Codex reference docs are not binding and should not be read by default. They exist only if the user explicitly asks for Codex context or Claude needs a narrow historical detail.

## Loop Format

Each meaningful Claude pass should follow this loop:

1. Restate the current slice in one paragraph.
2. Identify which product requirement from `docs/PRODUCT.md` it serves.
3. Inspect relevant research or architecture docs before choosing an implementation.
4. Build one vertical slice, not a disconnected demo.
5. Add or update tests/smoke checks where behavior is testable.
6. Run the relevant verification.
7. Write a concise progress note.
8. List what remains partial or unproven.

## Required Progress Note Shape

Progress notes can live in `docs/progress.md` once that file exists.

Use this shape:

```markdown
## YYYY-MM-DD - <slice name>

Goal:

What changed:

Verification:

Real-audio fixture used, if any:

What failed or remains partial:

Next recommended slice:
```

## Architecture Freedom With Evidence

Claude may choose Tauri, JUCE, Rust-native, C++/native, a hybrid shell, or another local desktop architecture.

Claude must not choose by vibes alone. It should document evidence:

- Audio latency.
- Export parity.
- Windows packaging.
- Development speed.
- Testability.
- Maintenance risk.
- Fit with Track Master and Album Master.

## Research Usage Rule

Use research files when making DSP or mastering decisions:

- `docs/research/audio-mastering-technical-research.md`
- `docs/research/deep-research-report.md`
- `docs/research/mastering-settings-reference.md`
- `docs/research/compass-artifact-e83b62aa.md`

Research should inform implementation, not become UI clutter. Prefer safe defaults, honest quality checks, and plain-language explanations.

## Private Fixture Rule

Use `private-audio-fixtures/manifest.json` when the user provides private audio.

Never commit private audio, rendered masters from private audio, or waveform snapshots derived from private audio unless the user explicitly asks.

## Completion Rule

Do not declare a slice complete unless it has:

- Product requirement coverage.
- Working behavior.
- Verification evidence.
- Known gaps listed.

If the implementation is only a demo, call it a demo.

# YES Master

YES Master is a private local desktop mastering app built with Tauri, React,
TypeScript, and a Rust audio engine. Track Master and Album Master are both
functional, exports are explicit and non-destructive, and the repo now carries
build paths for the desktop targets Dan is actively using. The product stays
local/offline by default; no private audio belongs in git.

## Current State

- Start here for any new contributor or agent session: `docs/HANDOFF.md`.
- Latest dated handoff: `docs/HANDOFF_2026-05-18_evening.md`.
- Product canon: `docs/PRODUCT.md`.
- Append-only implementation log: `docs/progress.md`.
- Architecture decision: `docs/adr/0001-tauri-rust-stack.md`.

There is no separate `CHANGELOG.md` by choice. Commit history plus
`docs/progress.md` are the change record.

## Build

Development:

```bash
git clone <repo-url>
cd album-mastering-studio-claude-build
npm install
npm run dev
```

Installer/package builds:

```bash
# macOS
npm run build:mac

# Windows
npm run build:windows
```

Use `CLAUDE.md` for the full verification recipe, including frontend tests,
Rust fast/slow lanes, and shell-specific commands.

## Product Shape

The app has two modes:

- Track Master: fast mastering for one or more independent songs.
- Album Master: album-aware mastering for ordered records, with per-track
  adaptation and continuous album export.

Track Master is the core vertical slice. Album Master builds on the same
analysis, rendering, delivery, and export foundation.

## Private Audio

Do not commit private audio, rendered masters from private audio, waveform
images derived from private audio, or fixture-specific generated artifacts.
Use the ignored folder:

```text
private-audio-fixtures/
```

Add a local `manifest.json` there to describe fixture purpose, quick/slow test
suitability, and listening notes.

## New Session Checklist

1. Read `docs/HANDOFF.md`.
2. Read `docs/HANDOFF_2026-05-18_evening.md`.
3. Read `docs/PRODUCT.md`.
4. Read `CLAUDE.md`.
5. Check the tail of `docs/progress.md`.
6. Read `docs/followups/listening-batch-2026-05-19.md` and
   `docs/followups/infrastructure-2026-05-19.md` before choosing the next
   slice.

The architecture is no longer open-ended; ADR 0001 records the Tauri + Rust
stack decision and the reasons it was chosen.

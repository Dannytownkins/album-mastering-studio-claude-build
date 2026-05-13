# Phase 11.x — notes from abandoned home `master` line

**Added:** 2026-05-12  
**Why:** This clone had diverged from `origin/master`. The branch validated at work was the **remote** history (~34 commits). These notes capture what lived only on the **local** line before it was dropped in favor of `origin/master`.

## Were you testing this at work?

**No** — if the app was pulled from GitHub `master` at work, you were exercising **`origin/master`**, not the five commits below. Those commits existed only on this machine until the histories were realigned.

## Local-only commits (not on the branch you tested)

| Commit     | Summary |
| ---------- | ------- |
| `8ff1b9f` | Phase 11.8: presets get real tonal character + tighter UI |
| `12897da` | Phase 11.8 tests + progress log |
| `408b16d` | Phase 11.9: fold StaleBar into Transport, consolidate Export row |
| `599d6a4` | Phase 11.9 follow-up: user-preset row goes inline |
| `2af55da` | Phase 11.9: log workspace consolidation in progress.md |

Recover details after a hard reset: **`git reflog`** → find the old `HEAD` (e.g. before `reset --hard origin/master`), then **`git show <hash>`** or **`git cherry-pick`** individual commits.

## Themes worth revisiting later

1. **DSP / presets (`src-tauri/src/dsp.rs`):** A `preset_character`-oriented mapping (per-preset EQ/sat/gain caps) with intensity scaling and user EQ on top; input gain driven from preset character × intensity. This conflicted with the remote approach (`preset_scale`, explicit `input_gain_db` in the chain, Phase 12.x tuning).

2. **UI (`src/App.tsx`, `src/App.css`):** Experiments to fold live/render status into **Transport** and consolidate the export/stale area; user presets in a tighter inline row. Remote kept **UndoRedoBar**, **StaleBar** (progress, live-update badge, audit WAV, clipping chip), etc.

3. **`docs/progress.md`:** Phase 11.8 / 11.9 narrative overlapped with remote’s larger 2026-05-12 progress reconciliation.

## Deliberate choice

Current **`master`** matches **`origin/master`** — the line known good from work. Nothing here implies those Phase 11.x ideas were wrong; they were simply **not merged** so this clone stays aligned with what you already validated.

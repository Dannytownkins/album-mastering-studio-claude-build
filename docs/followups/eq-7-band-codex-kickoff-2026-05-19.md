# Codex Kickoff — 7-Band EQ Expansion

Use this note to start a fresh Codex chat for the 7-band EQ expansion without
pulling in stale plans or chat-only context.

## Source Of Truth

Primary plan:

- `docs/followups/eq-7-band-plan-2026-05-19.md`

Required startup reads:

1. `CLAUDE.md`
2. `docs/HANDOFF.md`
3. `docs/HANDOFF_2026-05-18_evening.md`
4. `docs/followups/eq-7-band-plan-2026-05-19.md`
5. Tail of `docs/progress.md`

Useful reference only after reading the plan:

- `docs/EQ_ARCHITECTURE_RECON_2026-05-19.md`

Do not treat these as implementation plans:

- `docs/eq-7-band-plan-prompt-2026-05-19.md` is the prompt that led to the
  final plan. It is not the current plan.
- `docs/reference/` is older Codex-path context. Do not read it unless Dan
  explicitly asks.
- Chat screenshots or earlier v1.0/v1.1 discussion are superseded by the
  current plan file above.

Do not edit `docs/PRODUCT.md` for this slice.
Do not touch `private-audio-fixtures/`.

## Suggested Fresh-Chat Prompt

```text
Repo:
C:\Users\SM - Dan\Documents\GitHub\album-mastering-studio-claude-build

We are implementing the 7-band EQ expansion for YES Master.

Before coding:
1. Confirm repo path and current git status.
2. Read CLAUDE.md, docs/HANDOFF.md,
   docs/HANDOFF_2026-05-18_evening.md,
   docs/followups/eq-7-band-plan-2026-05-19.md,
   and the tail of docs/progress.md.
3. Treat docs/followups/eq-7-band-plan-2026-05-19.md as the source of truth.
   Do not implement from docs/eq-7-band-plan-prompt-2026-05-19.md or any older
   chat summary.
4. Do not read docs/reference/. Do not touch private-audio-fixtures/. Do not
   edit docs/PRODUCT.md.

Rollback-friendly workflow:
1. Do not start directly on master unless Dan explicitly tells you to.
2. Create a branch from current master:
   git switch master
   git pull --ff-only
   git switch -c codex/eq-7-band-expansion
3. Create a local safety tag before implementation:
   git tag pre-eq-7-band-2026-05-19
4. Keep commits small and independently revertable. If a slice goes bad,
   revert the latest commit on the branch or reset the branch to the safety
   tag. Do not force-push master.

Commit sequence from the plan:

Commit 0: Pre-flight per-preset chain-output SHA snapshots.
- Add deterministic per-preset SHA snapshots before DSP changes.
- Confirm synth_pink_stereo fixed-seed determinism.
- Run cargo test for the new SHA test.
- Run the fast Rust lane required by CLAUDE.md.
- Commit only after green verification.

Commit 1: Rust state + DSP extension.
- Add sub/high_mid/sparkle settings, calibration, coeffs, state, and chain
  placement exactly as the plan prescribes.
- Preserve the existing process_sample low_mid divergence. Do not fix it.
- Add the explicit guard test for that divergence.
- Add per-band frequency-response tests.
- Update Rust test fixtures.
- Run cargo test --lib, cargo test, and AMS_RUN_REAL_FIXTURE=1 cargo test
  from src-tauri because this touches the DSP chain.
- Commit only after green verification.

Commit 2: TypeScript state + setter + prop plumbing.
- Update bindings, defaults, setEqBand, Macros onEq type, and TS fixtures.
- Run npm test and npm run build from repo root.
- Commit only after green verification.

Commit 3: Visual EQ component.
- Expand VisualEqPanel to seven bands.
- Use sub qOctaves=1.2, high-mid qOctaves=1.0.
- Primary nodes: radius 8, label opacity 1.0, halo.
- Secondary nodes: radius 5, label opacity 0.85, same 18px hit target.
- Do not ship "TBD" colors. Pick real non-placeholder hex values in this
  commit and mention them in the progress entry.
- Run npm test and npm run build.
- Do visual smoke at 1920x1080, 1600x940, and 1366x768. Check node/label
  overlap, edge collisions, and primary/secondary hierarchy.
- Commit only after green verification.

Final verification before asking Dan to merge:
- npm test
- npm run build
- cargo test --lib from src-tauri
- cargo test from src-tauri
- AMS_RUN_REAL_FIXTURE=1 cargo test from src-tauri

Progress and review:
- Append docs/progress.md after each verified commit using the existing shape.
- Push the branch after each commit.
- Stop after each commit for Dan review unless Dan explicitly asks you to run
  the whole sequence.
- Do not merge to master until Dan approves the branch.

Important non-goals:
- Do not tune preset baselines. New preset calibration fields stay 0.0 dB.
- Do not add new knobs. Visual EQ gets the new drag-only bands.
- Do not fix process_sample's existing low_mid skip in this slice.
- Do not extend apply_album_shadow to the three new bands in this slice.
- Do not introduce portable tanh. If SHA constants differ by platform, use the
  plan's empirical-first approach and OS-gated constants only if needed.
```

## Rollback Notes

The branch and tag are intentionally conservative. The current project usually
lands verified slices on `master`, but this EQ expansion is broad enough that a
review branch is the cleaner walk-back story.

Safe rollback options:

- Before merge: delete or abandon `codex/eq-7-band-expansion`.
- After one bad commit on the branch: `git revert <commit>`.
- If the branch should return to the starting point:
  `git reset --hard pre-eq-7-band-2026-05-19` on the branch only.
- After merge to master, prefer `git revert` of the merge or individual commit.
  Do not rewrite pushed master history.

If Dan decides to keep the repo's usual master-only cadence instead, still keep
the four planned commits separate. That gives a clean revert path even without a
feature branch.

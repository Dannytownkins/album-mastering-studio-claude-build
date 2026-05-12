# `/schedule` Routine Prompt — Album Mastering Studio (Claude Build)

The exact prompt for the `/schedule` routine that drives autonomous work on this repo.

Register via `/schedule create` (only the user can run that command). Suggested cadence: every 2–4 hours during active development, or daily for slower progress.

## Routine prompt

Copy this into `/schedule create`:

```text
Work directory: C:\Users\SM - Dan\Documents\GitHub\album-mastering-studio-claude-build

1. Read docs/HANDOFF.md first. Then read CLAUDE.md, docs/PRODUCT.md, docs/IMPLEMENTATION_PLAN.md, docs/progress.md, docs/CLAUDE_WORK_LOOP.md.

2. Identify the next unfinished slice. If docs/progress.md ends with a line matching "PHASE N CONFIRMED — proceed to N+1", you may move into Phase N+1. Otherwise stay within or before the current phase.

3. Work exactly one verified slice. Follow the work loop in docs/CLAUDE_WORK_LOOP.md and docs/HANDOFF.md.

4. Run verification per docs/HANDOFF.md "Verification commands". All commands must pass.

5. If verification passes: commit with the message shape from docs/HANDOFF.md, push to origin/master, append a progress.md entry, stop.

6. If verification fails: leave the work uncommitted, append a progress.md entry describing the failure and what to try next, stop. Do not commit broken state.

7. Do not modify docs/PRODUCT.md. Do not touch private-audio-fixtures/. Do not force-push. Do not skip phases.

If a slice is too big for one session, complete one part fully, leave a clear "Next recommended slice" entry, and stop.
```

## Cadence guidance

- **Every 2h:** aggressive; expect 3–6 slices per day.
- **Every 4h:** moderate.
- **Daily:** slower but lower context drift.
- **On-demand:** trigger via `/schedule run <id>` when you want a slice worked now.

## Phase confirmation

After Dan reviews a completed phase and wants to unlock the next one, he appends a single line to `docs/progress.md`:

```
PHASE N CONFIRMED — proceed to N+1
```

The routine refuses to cross a phase boundary without this line. That's the manual gate that keeps quality control with Dan.

## Manual override

Open `claude` in the repo at any time and tell it which slice to work — it reads `HANDOFF.md` the same way the scheduled routine does. Manual sessions can also write the `PHASE N CONFIRMED` line directly when Dan signals approval.

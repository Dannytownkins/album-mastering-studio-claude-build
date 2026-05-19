# ADR 0002 — Cross-Machine Plan Handoffs

## Status

Accepted.

## Context

Dan may plan a slice in one Codex/Claude session and ship it from another
machine or context. Chat continuity is not guaranteed across those sessions,
so implementation-specific decisions can be lost unless they live in the repo.

## Decision

Any in-flight slice plan with material implementation choices should be written
to a repo note before handoff. Use `docs/followups/` for lightweight slice notes
or a numbered ADR when the decision is durable workflow/architecture guidance.

## Consequences

- The next session reads repo-local context instead of relying on chat memory.
- Plan-to-ship handoffs should include chain placement, topology, verification
  gates, and known non-goals.
- Small objective slices can still proceed directly when the repo already holds
  enough context.

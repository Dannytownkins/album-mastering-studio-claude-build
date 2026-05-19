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

## Addendum 2026-05-19 — plan-doc claims require recon-grounded verification

Observed pattern across two slices (the engine.rs split sequence and the 7-band
EQ expansion): a plan-doc's first draft claimed that an existing test gate
would catch a class of regression, when the gate actually didn't exist. In both
cases a cross-vendor review (Codex) caught the overclaim and the plan was
revised to add the missing gate before any DSP/byte-sensitive code moved.

Pattern in plain language:

> "The slow lane covers this" / "the existing tests catch this" / "this is
> already byte-identity-protected" — said about a gate that turns out not to
> exist when somebody actually greps the test files.

This is distinct from the original ADR 0002 problem (chat context lost across
sessions). It's a *plan authorship* problem: claims about repo state get
written from inferred convention rather than recon.

**Practice going forward:**

- When a plan-doc names an existing test, gate, or convention as a safety net,
  the plan author must verify by grep / file read before the claim ships.
- "I assume the slow lane catches X" is not a load-bearing claim. "I verified
  at `tests/foo.rs:NN` that the slow lane snapshots Y" is.
- When recon would have caught the overclaim, the plan-doc revision should
  name what was wrong AND add the verification gate that would have prevented
  the assumption.
- Cross-vendor review (Codex reviewing a Vera plan, or vice versa) is a
  legitimate mitigation, but the better baseline is recon-first plan
  authorship so the second-pass review catches edge cases, not load-bearing
  errors.

Two instances suffice to call this a pattern. If a third instance shows up,
revisit whether this addendum should grow into a fuller process ADR (e.g.
"0003 — Plan-doc claims require evidence").

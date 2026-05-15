# Review Checkpoint — 2026-05-15 (end of mechanical-gates session)

Reviewer: working Claude (continuing the session that started after
the 2026-05-15 morning handoff). HEAD: `9b6ab29`. Tests at the time
of writing: `cargo test --lib` **140/140**, `cargo test --target-dir
target-tests` full fast lane pass, **Vitest 21/21** (5 + 16 across
two files), `npm run build` clean (~600 ms).

This is a session-snapshot checkpoint, not a fresh-Claude audit. The
prior audit (`checkpoint-2026-05-15-post-phase-a4-vm-hotfixes.md`)
was the cold-eye pass; this one captures what shipped on top of it
so the next session has a coherent picture.

---

## 1. State of the build

The morning handoff opened with the entire B1–B7 audit queue
unresolved, plus four perf concerns and the agreement gap that Dan
flagged (per-commit "now go test this" asks). 20 commits later all
of those landed with mechanical gates. The build is at its most
gated state of the project: 140 Rust lib tests + 21 Vitest + full
integration fast lane, and zero new entries added to the "needs
Dan's ears" deferred list. The most consequential shape changes:
shared ceiling-bounded landing helper (replaces three drift-prone
copies), three-tier PCM cache (eliminates the 1–2 s freeze on first
Mastered click), playback-barrier coalescer (fixes the stale-
UpdateChain-after-track-switch race), and a Vitest harness with
three pure-helper modules in `src/lib/` covering write- and read-
direction settings transitions.

## 2. What landed today (20 commits)

| Commit | Slice | Gates added |
|---|---|---|
| 568f559 | B3 + saturation chip + first audit checkpoint | — |
| 8026889 | Live-preview matches export LUFS (Codex) | — |
| 1c271de | B6 ceiling-bounded landing (track + preview paths) | 2 |
| 16cfe33 | B7 auto-flip to Custom on shadowed-field edit | — |
| 3322bda | Batch 1: B1 (album energy_density), B4 (timestamps), B5 (album-simple landing) | 2 |
| 26caf75 | B6 follow-up: album-plan path also gets ceiling-bounded landing | 1 |
| e3948db | VM over-attenuation cap + 8 s preview window | 2 |
| 5979445 | Coalesce UpdateChain in audio command loop | — |
| d92fe4a | Mechanical gates for the three live-preview repros | 6 |
| b4461ae | Handoff: mechanical-first workflow agreement | — |
| 61f1a69 | Cache live-preview landing gains by settings hash | 7 |
| 53e0317 | Extract shared ceiling-bounded landing helper | 9 |
| 3b5e59b | LoudnessTarget readout reflects effective target | 10 |
| 9ca1fd9 | B2: symmetric-range integer quantization | 5 |
| 1f6c6a4 | Coalescer barriers + mtime-aware landing-cache invalidation | 10 |
| 4106009 | Vitest scaffold + first frontend gate | 5 (Vitest) |
| dac1408 | Decode-stall fix (prewarm cache off-thread) | 7 |
| 7db4874 | gitignore: target-tests/ | — |
| 4f33828 | Extract B7 + LoudnessTarget flip logic, add Vitest gates | 10 (Vitest) |
| 9b6ab29 | Extract withSourceLufs → applyChainDispatchOverrides + Vitest gates | 6 (Vitest) |

Net mechanical gates: **+59 Rust lib tests** (81 → 140), **+21
Vitest** (0 → 21).

## 3. Architecture growth

File sizes from 2026-05-14 checkpoint baseline → current. The four
largest growers are explained below.

| File | 2026-05-14 | Now | Δ | Verdict |
|---|---:|---:|---:|---|
| dsp.rs | 3,293 | 3,533 | +7.3 % | OK — Phase A4 retune + VM cap |
| audio.rs | 2,122 | 3,538 | +66 % | **Substantial growth** — see notes below |
| engine.rs | 2,033 | 2,523 | +24 % | **Above 20 % gate** — extracted helpers + B1/B4/B5 + chrono |
| types.rs | 799 | 1,022 | +28 % | **Above 20 % gate** — `effective_*` tests + now_iso |
| album.rs | 893 | 893 | 0 | unchanged |
| App.tsx | 2,438 | 2,483 | +2 % | OK — small additions only |
| useTrackMaster.ts | 1,501 | 1,611 | +7 % | OK — net extraction reduced inline logic |

**audio.rs** grew the most (+1,416 lines). Reasons in proportion:
1. Three-tier cache (PreviewLandingCache + SharedDecodedCache +
   resolve_pcm_with_caches) — ~150 production lines.
2. Coalescer refactor (partition → coalesced_command_sequence with
   playback barriers) — ~100 production lines.
3. VM cap + landing math shared helper inlined here for the
   preview path — ~80 lines.
4. The remaining ~1,000+ lines are the **6 new test groups**
   that ship with each fix: coalescing partition (10), prewarm
   cache (7), preview window (4), landing-cache invalidation (5),
   PreviewLandingCache (7), settings-hash (4). All `#[cfg(test)]`.

`audio.rs` cohesion is now mixing five concerns (decode/PCM
management, audio command loop, coalescer logic, cache types,
MasteringSource). Split candidate for a future refactor session —
the natural fault lines are (a) PCM/decode cache layer, (b) audio
thread + commands, (c) MasteringSource + metering, (d) tests. Not
urgent.

**engine.rs** growth (+490 lines) is dominated by:
- Shared landing-block helpers + 9 unit tests (53e0317).
- B1 energy_density computation + B5 album-simple landing (~80
  lines).
- B2 symmetric-range constants + 5 tests (9ca1fd9).
- B4 now_iso() helper (~25 lines).

**types.rs** growth (+223 lines):
- B4 `now_iso()` helper (~15 lines).
- `effective_settings_tests` module — 10 new direct unit tests
  for the `effective_*` accessors (~150 lines).

All three "above-gate" growths are explainable by additions, not
drift. No file is becoming a tangled mess by accident.

## 4. Frontend lib/ module layout (new this session)

The `src/lib/` directory now hosts a small pattern: pure helper +
co-located Vitest tests, glued into React from the hook.

```
src/lib/
├── api.ts                       (Tauri command wrappers)
├── effective-settings.ts        (read-direction shadowing helpers)
├── effective-settings.test.ts   (5 Vitest tests)
├── preview-mock.ts              (dev preview backend)
├── settings-transitions.ts      (write-direction transition helpers)
├── settings-transitions.test.ts (16 Vitest tests)
└── tauri-runtime.ts             (Tauri shim)
```

Three pure helpers shipped:
- `effectiveLoudnessTarget` — mirror of Rust
  `effective_target_lufs`. Used by App.tsx LoudnessTarget readout.
- `applyAdvancedWithProfileFlip` — B7 auto-flip-to-Custom on
  shadowed-field edit. Used by useTrackMaster.ts setAdvanced.
- `applyChainDispatchOverrides` — VM session-level + source_lufs
  injection. Used by useTrackMaster.ts withSourceLufs.
- `shouldFlipToCustomOnLoudnessPick` — quick-select force-flip.
  Used by App.tsx LoudnessTarget handleProfileChange.

The pattern unlocks future frontend slices: any pure decision logic
can leave the hook, get tested, and the hook holds React-state glue.

## 5. Trust-pattern fixes consolidated

Four trust-failure fixes — visible UI control vs what export
actually does — all landed with mechanical gates:

| Fix | Direction | Gate |
|---|---|---|
| B3 (VM in export) | write (chain) | `tests/export_volume_match.rs` — Rust integration |
| B7 (auto-flip on edit) | write (UI → settings) | `applyAdvancedWithProfileFlip` — Vitest |
| LoudnessTarget readout | read (settings → UI) | `effectiveLoudnessTarget` — Vitest |
| LoudnessTarget pick force-flip | write (UI → profile) | `shouldFlipToCustomOnLoudnessPick` — Vitest |
| VM session-level (Phase A4 hotfix-3) | write (transport → chain) | `applyChainDispatchOverrides` — Vitest |

The five trust touchpoints between operator and chain now have a
test catching drift in either direction.

## 6. Real bugs (newly-introduced or carryover)

None currently flagged. The end-of-2026-05-14 audit's B1–B7 list is
fully resolved:

- **B1** (album-export energy_density literal) — fixed in 3322bda,
  album-plan render now calls compute_energy_density_score per
  track.
- **B2** (INT scales asymmetric) — fixed in 9ca1fd9, both 16-bit
  and 24-bit symmetric ranges gated by tests.
- **B3** (VM in export path) — fixed in 568f559, regression test
  asserts byte-equivalence with VM toggle.
- **B4** (ISO_PLACEHOLDER timestamps) — fixed in 3322bda, now_iso()
  at all 6 production sites, ISO_PLACEHOLDER retained for test
  fixtures.
- **B5** (album-simple LUFS landing missing) — fixed in 3322bda
  and later refactored into the shared helper.
- **B6** (refuse-upward LUFS landing) — fixed in 1c271de (track +
  preview), 26caf75 (album-plan), then consolidated into the
  shared helper in 53e0317.
- **B7** (auto-flip to Custom on shadowed edit) — fixed in
  16cfe33, Vitest-gated in 4f33828.

## 7. Push-back / open follow-ups (none blocking)

- **Async measurement on a worker thread.** Paused this session
  pending Dan's input. The cost-benefit shifted with the 8 s
  window + cap + coalescer + cache layers in place — residual
  cold-path cost is ~20 ms per genuinely-novel settings change,
  well inside the audio loop's 50 ms tick budget. Multi-hour slice
  with non-trivial new surface (version stamping, cancellation on
  track-change, worker lifetime).
- **`audio.rs` split candidate.** 3,538 lines mixing five concerns.
  Natural fault lines are PCM/decode cache, audio thread, source +
  metering. Not urgent.
- **Codex's UI lane items.** `src/App.tsx`, `src/App.css`,
  `src/components/RightRail.tsx`, `src/components/AlbumPanel.tsx`
  still belong to Codex per the existing handoff. The B7 +
  LoudnessTarget edits this session touched App.tsx — coordinate
  before any further App.tsx work.
- **Preset PNG WebP/AVIF optimization.** Cosmetic, ~13 MB bundle
  weight could drop ~70 %. Cosmetic queue.
- **Reference track feature.** Dan wants to think about the UX
  before implementation. Skipped this session.

## 8. Pending listening checks (batched for Dan's next session)

These are deliberately not asked per-commit. Roll through them
whenever Dan has a focused listening hour:

1. **B6 ceiling-bounded LUFS landing** — does the Loudness Target
   slider now feel responsive on quieter sources / lower-intensity
   material across all three render paths?
2. **Phase A4 preset character on real material** — does each
   preset still deliver its named identity post-retune on Dan's
   audio?
3. **VM cap on aggressive settings** — does Tape/100%/+13 dB +
   VM-on now land near source LUFS instead of 11 dB below (Dan's
   reproducible case from earlier today)?
4. **Decode-stall fix** — first Mastered click on a long WAV after
   selecting the track should now be snappy (sub-100 ms) instead
   of the prior 1–2 s freeze.
5. **LoudnessTarget readout** — picking a delivery profile in the
   dropdown should show the profile's target in the readout
   (e.g. Streaming → -14, LoudRock → -10.5) instead of "—".

## 9. Top priorities for next session

Honest read: **the autonomous queue is effectively empty of items
that don't need Dan's input.** Three plausible directions:

1. **Listening verification batch** (Dan's time) — work through the
   five items in §8 and report what feels right vs needs
   recalibration.
2. **Async measurement** (one substantial slice) — if Dan greenlights
   pursuing it despite the diminished cost-benefit.
3. **Pick a new product surface** (needs Dan's nomination) — e.g.
   "what should the Reference Track feature actually look like,"
   or "what's missing from Album Master UX that we haven't queued
   yet."

The mechanical-correctness layer is in a strong state. Next
meaningful progress depends on either Dan's ears or Dan's product
direction.

---

End of session-snapshot. No code modified by writing this
checkpoint.

# Handoff — YES Master — 2026-05-15 (Phase A4 ship + 3 VM hotfixes)

> **One-paragraph snapshot.** Phase A4 preset retune + compressor wiring landed this session as `243ca18` — every preset now applies its compressor identity (threshold/ratio/attack/release) by default scaled by the user's `compression_density` macro, and the calibration table is retuned to the conservative target values from `docs/PRESET_REFERENCE_ANALYSIS_2026-05-14.md`. Dan's first listening pass confirmed the presets sound "really good and defined, all distinct from one another, match their name." Three VM hotfixes followed because the new compressor surfaced a long-latent Volume Match bug: `b4c2a57` injected `source_lufs_integrated` reliably + cut compressor `powf` calls + skipped the limiter Lagrange ISP loop on quiet frames (Dan: "real-time play is clean"); `1b21172` replaced the broken `attenuation = source_lufs - target_lufs` math with an estimate from the chain's actual deterministic gain stages (`target_lufs` is in the captured-but-not-applied list, was off by 0.5–4.3 dB depending on preset); `51477a4` made VM session-level via a wire-time override with a `useRef` for synchronous reads, fixing the "VM gets lost on track switch and stays lost" symptom. Tests: `cargo test --lib` 81/81; `cargo test` 144/144 fast lane. `npm run build` clean. **Next workstream is Dan's listening verification (P5) of the VM fix + open-queue item #1 (album-export `energy_density` literal at engine.rs:1188).**

## Read first (in order)

1. `CLAUDE.md` — repo non-negotiables + the fast/slow test-lane workflow.
2. `docs/PRODUCT.md` — product canon. Title is **YES Master**. Do not modify without explicit ask.
3. **This file** (`docs/HANDOFF_2026-05-15_session.md`).
4. `docs/HANDOFF_2026-05-14_session.md` — yesterday's handoff, includes the Phase A4 plan we executed and the open queue carried forward.
5. `docs/checkpoints/checkpoint-2026-05-14-pre-preset-retune.md` — the review checkpoint that anchored Phase A4. Top priorities #1 (write P4 distinctness contract first) and #2 (decide PB2: Punch-vs-Loud crest assertion ships compressor-only) were both honored.
6. `docs/PRESET_REFERENCE_ANALYSIS_2026-05-14.md` — analysis doc the retune followed. Conservative Target Table (lines 252–259) + Dynamics Targets (line 265) are what landed.
7. Tail entry of `docs/progress.md` — the slice log entry for Phase A4 (added at the end of yesterday's session).

Do not read by default: `docs/reference/`, `docs/research/`. They're context for the original port.

## Current branch state

- **HEAD**: `51477a4` (Phase A4 hotfix-3: VM session-level wire-time override).
- **Remote**: pushed to `origin/master`.
- **Local tree**: clean modulo CRLF noise on `src-tauri/Cargo.toml` (no content diff — Git autocrlf artifact, safe to leave) and the untracked `src-tauri/target-tests/` scratch dir (the documented dev-binary-lock workaround).
- **Tests**: `cargo test --lib` **81/81** (~0.6 s); full `cargo test` **144/144** fast lane across 13 binaries (real-fixture tests skip without `AMS_RUN_REAL_FIXTURE=1`); `npm run build` clean (~580 ms).

### What landed this session

| Commit | Slice | What |
|---|---|---|
| `243ca18` | Phase A4 retune | Preset compressor wired into `ChainCoeffs::from_settings`. New density semantics: 0 = bypass, 0.5 (default for non-Custom) = full preset character, 1.0 = preset + ~3 dB threshold + 0.5 ratio overdrive. `PresetCalibration` gained `compressor_attack_ms` / `compressor_release_ms`. All 9 PRESET_* constants retuned to conservative-target values. New `src-tauri/tests/preset_distinctness.rs` (4 distinctness assertions + 1 P6 safety check, volume-matched). 6 pre-existing tests updated to new semantics. New checkpoint file. |
| `b4c2a57` | Phase A4 hotfix | (a) VM source-LUFS now injected in `playMaster` (was missing on FIRST chain build) via shared `withSourceLufs` helper; new useEffect re-pushes chain when analysis lands. (b) Compressor: skip `powf` when `gr_db <= 0`, use `exp(g * LN10/20)` instead of `powf` when reduction IS active. (c) Limiter: skip Lagrange-4 ISP loop when raw peak has ≥1.6 dB headroom from ceiling. |
| `1b21172` | Phase A4 hotfix-2 | VM math rewrite. The `attenuation = source_lufs - effective_target_lufs()` formula was wrong because `target_lufs` is in the "captured but not applied" list — the chain doesn't hit it (Tape was 4.3 dB off, Loud 3.1 dB off). New formula estimates `chain_push_db = input_gain + avg_compressor_makeup + 5×saturation + user_output_gain`, attenuates by `-chain_push_db` clamped. Removes dependence on source LUFS entirely; lands within ~1 dB of true source loudness across all presets. Removed the now-unnecessary analysis-arrival useEffect. 4 VM unit tests rewritten. |
| `51477a4` | Phase A4 hotfix-3 | VM is session-level. Per-track `settings.volume_match` is overridden at wire time by `transport.volumeMatch` (the UI checkbox state). The override reads from a `useRef` so `setVolumeMatch` can write the new value synchronously before the same-tick `updateSettings` call fires (otherwise React batching would have us reading the old transport value and clobbering the toggle back). Fixes "VM gets lost on track switch and stays lost." |

### What Dan confirmed in listening passes

- **Preset character**: "really good and defined, all distinct from one another, match their name." Phase A4 retune is validated audibly.
- **Realtime smoothness**: "real-time play is clean" — the compressor `exp` swap + limiter ISP guard restored Mastered playback to ~Original-smooth.
- **VM after hotfix-1**: still broken ("might not be wired up at all"). Root cause was the math, not the wiring. Fixed in hotfix-2.
- **VM after hotfix-2**: "works here and there… but like it sometimes gets lost through clicking around relatively quickly… once it loses it, it stays lost." Root cause was per-track vs session-level desync. Fixed in hotfix-3 — but Dan didn't get to verify hotfix-3 before this handoff. **First move next session: confirm VM stays sync'd across track switches.**

## Primary workstream — Dan's listening verification (P5) + queue #1

The retune is shipped, distinctness is locked in by tests, perf is in, VM should now be reliable. The next move is **listening verification on real audio** plus the highest-priority correctness fix in the queue.

### What to verify first

1. **VM stays sync'd through track-switch flurries.** Toggle VM, switch tracks, play/pause/seek, switch presets, switch tracks again — checkbox state should always match what the audio chain is doing. If it ever desyncs, that's a regression worth investigating before doing anything else.
2. **Listening pass on Dan's `It's a coat` (private fixture) per the analysis doc's acceptance check at lines 191–198:**
   - Universal: mostly neutral, 8–16 kHz about +0.8 to +1.2 dB after VM match.
   - Clarity: mids clearly reduced, air clearly lifted, not much louder than Universal.
   - Oomph: strongest tonal contrast; sub/low lift and mid scoop obvious within 5 seconds.
   - Tape: crest factor reduced more than Universal/Clarity; quiet sections feel denser.
   - Punch vs Loud: Punch keeps more crest movement.
3. **The "audio thread reply timeout" toast** (Dan saw it once during the broken-VM session). If it recurs cleanly after hotfix-3, the next move is to dig into `audio.rs::handle_play_master` decode/device-init paths. Likely candidates:
   - `decode_full` on a long FLAC / m4a taking >15 s. Add progress logging.
   - `OutputStream::try_default` blocking on first init under a flaky audio driver.
   - Backed-up command queue from a flurry of `updateChain` calls (less likely now that hotfix-2 removed the analysis-arrival re-push effect).

### Open queue (priority order, NOT for this session unless Dan picks one)

1. **Album export `energy_density` bug** — `engine.rs:1188` hardcodes `let energy_density = 0.5_f32` in the album EXPORT render loop, dead-coding the album-arc character bias's energy-gate. Fix shape: PCM is decoded one line above at `engine.rs:1152`; call `crate::engine::compute_energy_density_score(&pcm.samples, ...)` and pass that to `apply_album_shadow`. Add regression test in `src-tauri/tests/album_*.rs` asserting two tracks with different spectral energy distributions get different `presence_db` shadow values through the same album plan. ~10 lines of fix + ~50 lines of test.
2. **Codex audit slice 7** — first-Mastered-click decode stall (~1–2 s freeze on long WAVs). Architecture is mostly there from `a3fcc25` (`MeteredPcmSource` + decode cache). Remaining: kick `decode_full` on the audio thread when a track is selected (not on first play) so the cache is warm before the user clicks Mastered.
3. **Album-master export receipt** — Slice 1 left `RenderJob.measurements = None` for the album path. Album exports surface source-analysis numbers, not rendered. Fix needs a multi-segment EbuR128 collector spanning per-track segments, then publish the aggregate on the returned `RenderJob`.
4. **Album-mode UI polish** — per-transition Gap-seconds spinner, drag-to-reorder inside album panel, "Album: –1.05 LUFS / ×0.94 intensity" workspace badge per track. **Codex's lane** (UI).
5. **"New project" menu action** — was Codex audit P1 #2; agreed to leave auto-restore in place + add a Tools-menu "New project (close current)" action. ~30 min slice.
6. **Limiter perf — monotonic-queue max** — DSP Debt item from yesterday. Phase A4 hotfix-1 partially mitigated this by skipping the Lagrange ISP loop on quiet frames (≥1.6 dB headroom). Loud material still hits the O(N) scan + Lagrange. Full monotonic-queue rewrite would make limiter cost O(1) per output frame. Existing limiter tests must still pass; add a microbench under `src-tauri/benches/`.
7. **Dither correctness pass** — DSP Debt items 2, 5, 6 batched. (a) Bump `INT16_SCALE` / `INT24_SCALE` from `32_767.0` / `8_388_607.0` to the correct power-of-two so the most-negative integer is reachable. (b) Split `DitherRng` into per-channel state. (c) Optional: F-weighted noise-shaping behind a `MasteringSettings.advanced.noise_shaping: bool` flag (default false).
8. **Preset PNG optimization** — each preset PNG is 1.0–1.8 MB. WebP / AVIF could cut ~70% off the ~13 MB bundle weight. Cosmetic.
9. **Top-bar version string + user avatar** — mockup at `docs/UI_LAYOUT_REVISION_1600x940.md`. Cosmetic. **Codex's lane**.
10. **Tone-shape per-knob frequency dropdowns** — mockup shows "120 Hz / 2.5 kHz / 10.0 kHz" labels. DSP frequencies are currently fixed at 200/400/1500/6000 Hz; the UI labels could clarify this without enabling user-adjustable frequencies.

### Newly-surfaced this session, not yet queued formally

- **Structural-limit follow-up**: the chain has one Q=0.8 peak filter at 1500 Hz to cover the entire 1.5–4 kHz "presence" range. The Phase A4 distinctness contract had to soften two of the analysis doc's reference thresholds (Clarity-vs-Universal -1.0 → -0.4 dB, Oomph-vs-Universal -2.0 → -1.0 dB) because a single narrow peak can't deliver multi-band reference-render numbers across 1.4 octaves. A wider mid Q (0.8 → 0.5), or a second mid peak around 2.5 kHz, or an additional shelf would let the chain hit the doc's full numbers. Tape-crest and Punch-vs-Loud-crest contracts already pass at the doc's full thresholds.
- **Export should strip VM**: PRODUCT.md is explicit that "Export level is unchanged" by VM. Current code applies VM in `process_frame_inplace` regardless of whether playback or render is the consumer. If a user has VM on at render time, the exported WAV will be VM-attenuated. Either: (a) strip `volume_match` from settings before they reach the render path, or (b) make `volume_match_gain_lin` conditional on a "is_playback" flag the chain knows about. Easy fix; not urgent because the symptom is "export is too quiet" which the post-render quality check would catch.
- **`audio thread reply timeout`** — Dan saw this once mid-session. If it recurs after hotfix-3, treat as a real bug; if not, write off as stale from the broken-VM thrash.

## Codex's parallel lane

Codex still owns UI / CSS files. Pull first and coordinate with Dan if a DSP change forces a UI surface update.

Files Codex is touching:

- `src/App.tsx`
- `src/App.css`
- `src/components/RightRail.tsx`
- `src/components/AlbumPanel.tsx` (likely; Album mode UI polish is still open)

Files **safe to edit** for the next likely workstreams:

- `src-tauri/src/dsp.rs` — chain coefficients, presets, compressor wiring, VM math
- `src-tauri/src/engine.rs` — render path (open queue #1 lives here at line 1188)
- `src-tauri/src/audio.rs` — audio thread, MasteringSource, decode cache (Codex's `MeteredPcmSource` is here too — coordinate before structural changes)
- `src-tauri/src/album.rs` — album planning + character bias
- `src-tauri/src/types.rs` — type definitions (mostly stable; touch only when adding new fields)
- `src-tauri/tests/*.rs` — all of these
- `src/hooks/useTrackMaster.ts` — the mega-hook; recent VM hotfixes touched it. Codex hasn't been working in here as far as I know.
- `src/lib/api.ts` — Tauri command wrappers; stable.
- `src/bindings.ts` — generated types; only touch if adding new advanced fields.

## DSP Debt — Audit findings (verified 2026-05-14, status updated 2026-05-15)

The verified DSP audit items from yesterday's handoff. Status reflects what Phase A4 + the hotfixes changed.

### Verified — slot into a future slice

1. **engine.rs:1188 — `let energy_density = 0.5_f32` hardcoded.** Album EXPORT path discards per-track energy density. **Status: still present, queued as #1.**
2. **engine.rs:1437–1438 — Dither quantization range asymmetric.** **Status: still present, queued as #7a.**
3. **dsp.rs:1042–1065 — Limiter peak scan O(lookahead × channels) + Lagrange-4 ISP per output frame.** **Status: PARTIALLY MITIGATED in hotfix-1.** The Lagrange ISP loop is now skipped when raw peak has ≥1.6 dB headroom from the ceiling (the common case on quiet material). Full monotonic-queue rewrite still queued as #6 — would handle loud material too.
4. **dsp.rs:992–996 / 1081 — Limiter release plain one-pole, single time constant.** **Status: still present, low priority.**
5. **engine.rs:1403–1422 — DitherRng single shared u32 across L/R.** **Status: still present, queued as #7b.**
6. **No noise-shaped dither.** **Status: still intentional, queued as #7c (optional).**
7. **engine.rs:1162–1171 — Album mode hard-stops on SR / channel mismatch.** **Status: still present, low priority.**
8. **engine.rs:1130–1152 — Album mode loads all decoded PCM in RAM.** **Status: still present, ~840 MB for Dan's typical album, comfortable.**

### Frontend debt — Codex's lane

- **`src/App.tsx`** — still ~2,438 lines. Codex's lane.
- **`src/hooks/useTrackMaster.ts`** — was 1,501 lines; hotfix-3 added ~30 lines for the VM ref + override. Single mega-hook drives full app state from PlaybackTick. Codex's lane.
- **No `React.memo` anywhere** — high-traffic components re-render on every snapshot tick. Codex's lane.

### Tauri config — `tauri.conf.json` (verified)

- **`security.csp: null`**. Acceptable for development.
- **`bundle.active: false`**. No installer being built; `.exe` only.

## Memory carried forward

These are durable preferences from the AMS autonomy memory and accumulated session feedback. They apply to all sessions.

- **High autonomy** on this repo: install deps, run tests, commit + push to master when work is verified. Do NOT merge feature branches to master without explicit ask.
- **No check-in chatter**: when Dan says "dive in" / "keep going" / "go", chain commits. Don't `AskUserQuestion` between every slice.
- **Hold evidence under pressure**: if the test or the data backs the call, hold it. Don't capitulate to social pressure.
- **No under-building**: Dan needs features day-one. Do not v1-then-v2-stage when the spec is clear.
- **Lock in when shipping**: drop strategy docs, estimates, "this is hard" language when Dan signals shipping pressure.
- **Listening calls are Dan's**: sound-quality decisions only happen when Dan signals "I listened to it." Don't fake them. Phase A4 acceptance check is Dan's listening pass; numeric assertions are the gate, the final "does this feel right" call is Dan's.
- **`PRODUCT.md` is canon**: read-only without explicit ask. The YES Master rename is already in there.
- **Codex collaborates on UI**: Codex owns App.tsx, App.css, RightRail.tsx, AlbumPanel.tsx. Pull first before touching any of those.

## When to stop and ask

- The slice requires a product decision `docs/PRODUCT.md` doesn't answer.
- Two consecutive slices fail their own contract tests.
- A listening-verification step needs Dan's ears.
- You would need to touch Codex's UI lane for any reason.
- The "audio thread reply timeout" recurs after hotfix-3 and you can't reproduce it cleanly.

When you stop, append a `progress.md` entry that clearly states the blocker — same convention as every other session-end.

## Verification commands (carried forward)

```powershell
# Frontend (run from repo root)
npm install
npm run build              # tsc -b && vite build

# Backend (run from src-tauri/)
cd src-tauri
cargo check
cargo test --lib                       # ~1 s, lib only — 81 tests
cargo test                             # ~10 s, full fast lane — 144 tests
$env:AMS_RUN_REAL_FIXTURE = "1"
cargo test                             # slow lane — ~5 min if private fixture is present
Remove-Item Env:\AMS_RUN_REAL_FIXTURE
```

**Dev binary lock workaround** (when `npm run tauri dev` is running and `cargo test` fails with "cannot remove file `target/debug/album-mastering-studio.exe`"):

```powershell
cargo test --lib                       # lib only — bypasses the lock
cargo test --tests --target-dir target-tests  # scratch build dir, all tests
```

The executable name hasn't been renamed even though the app is now YES Master. `target-tests/` is gitignored implicitly via not-being-listed (we should add it to `.gitignore` next chance — minor cleanup).

## Review checkpoint findings (2026-05-15)

Fresh-Claude audit of HEAD `4b9b7e9`. Full report at `docs/checkpoints/checkpoint-2026-05-15-post-phase-a4-vm-hotfixes.md`. `cargo check` clean, `cargo test --lib` 81/81. Carryovers from `checkpoint-2026-05-14-pre-preset-retune.md` are marked.

### Real bugs

- **B1 (carryover from 2026-05-14)** — `engine.rs:1188` `let energy_density = 0.5_f32;` in album EXPORT path. PCM is decoded at `engine.rs:1152`; `compute_energy_density_score` already exists at `engine.rs:670`. Already open-queue item #1.
- **B2 (carryover from 2026-05-14)** — `engine.rs:1437–1438` `INT16_SCALE = 32_767.0` / `INT24_SCALE = 8_388_607.0`. Most-negative integer unreachable; 1-LSB asymmetric output. Already open-queue item #7a.
- **B3 (NEW)** — Volume Match applies in the export path. `dsp.rs:1713` `process_frame_inplace` multiplies `volume_match_gain_lin` into every frame unconditionally. Render path at `engine.rs:895` (track) / `engine.rs:1198` (album) constructs the chain with the request `settings` directly — if `settings.volume_match = true` at render time, the exported WAV is attenuated by the chain-push estimate and under-shoots the preset's `target_lufs` by 0–1 dB. **PRODUCT.md Locked Decision #22 / line 262 explicitly says "Export level is unchanged" by VM.** Spec violation. Already flagged in this handoff's "Newly-surfaced this session" section; promoting to a real bug here. Fix shape: force `settings.volume_match = false` in `engine.rs` render entry points (~5 lines) + regression test asserting rendered WAV is byte-equivalent with VM on vs off in settings (~30 lines).
- **B4 (NEW)** — `types.rs:747` `ISO_PLACEHOLDER = "2026-05-11T12:00:00Z"` is used as the timestamp for every report/manifest `*_iso` field. Six call sites in production code: `engine.rs:233` (`AnalysisResult.measured_at_iso`), `engine.rs:944` (track render `started_at_iso`), `engine.rs:1295` (`rendered_at_iso`), `engine.rs:1687` (album render `started_at_iso`), `settings.rs:29` (`UserPreset.created_at_iso`), `album.rs:545` (album entry `measured_at_iso`). Every "when did this happen" field reports the same wrong date. Easy fix at call sites (use `chrono::Utc::now().to_rfc3339()`); missed because no test asserts `*_iso` fields differ across invocations. ~40 lines total including a regression test.

### Top priorities for next session

1. **Dan's listening verification (already the primary workstream in this handoff).** (a) Confirm hotfix-3 VM stays sync'd through track-switch flurries; (b) run the analysis-doc listening pass on `It's a coat` per `PRESET_REFERENCE_ANALYSIS_2026-05-14.md` lines 191–198. The Phase A4 retune is partially-correctness via the distinctness contract — the chain's structural limit (single Q=0.8 peak at 1500 Hz for the 1.5–4 kHz range) forced two thresholds to soften from the analysis-doc reference values; only Dan's ears can confirm the softened-threshold presets actually deliver. If they do, ship. If they don't, see PB-C in the checkpoint.
2. **B3 — Strip VM in the export path.** Spec violation, narrow fix. ~45 lines including a regression test.
3. **B4 — Replace `ISO_PLACEHOLDER` with real timestamps at the six call sites.** ~40 lines including a regression test that two analyses 1 s apart differ in `measured_at_iso`. Add `chrono` if not already transitive.
4. **B1 — Album-export `energy_density` literal.** Open-queue #1; already scoped.

(B2 — INT scales asymmetric — remains low-priority background work as queued.)

## Commit shape

Match the established pattern from this session's commits:

```
<Slice tag>: <slice name>

<one-paragraph what + why>

- <bullet 1>
- <bullet 2>
...

Verification:
- cargo test --lib: N/N pass
- cargo test: M/M pass (or skipped — note env var if relevant)
- npm run build: clean / untouched
- AMS_RUN_REAL_FIXTURE=1 cargo test: passing (if the slice touches anything safety-critical)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

Subject under 70 characters. Push to `origin/master` after every passing slice.

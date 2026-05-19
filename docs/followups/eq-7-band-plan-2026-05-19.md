# Plan — 7-Band EQ Expansion (4 → 7 user-facing bands)

**Date:** 2026-05-19 (revised same day after Codex review)
**Status:** Plan-doc per ADR 0002. Implementation gated on Dan's approval.
**Drives from:** `docs/eq-7-band-plan-prompt-2026-05-19.md`
**Reference:** `docs/EQ_ARCHITECTURE_RECON_2026-05-19.md`

---

## Revision note

Initial plan reviewed by Codex on 2026-05-19. Four pushbacks accepted and incorporated:

1. **Compile-order fix.** Commit 1 originally referenced `settings.eq_sub_db` etc. that weren't added until Commit 2. **Revised:** Commit 1 now bundles Rust state-model + DSP (compiles standalone); Commit 2 is TS plumbing.
2. **Byte-identity gate added explicitly.** Original plan overclaimed that the existing slow lane provided byte-identical fixture protection — it doesn't (`contracts.rs:604` logs file size, no hash; the slow lane has no SHA snapshot). **Revised:** new pre-flight Commit 0 establishes per-preset SHA snapshots before any DSP changes go in. Same shape as the wav_writer pre-flight commit.
3. **`process_sample` divergence preserved verbatim.** Codex caught a pre-existing latent bug at `dsp.rs:2059` — `process_sample` skips `state.low_mid` between `state.low` and `state.mid`, while `process_frame_inplace` has all four. This slice does NOT fix the legacy divergence; it preserves it exactly (adds sub/high_mid/sparkle but not low_mid to `process_sample`). A guard test pins the divergence so it doesn't drift silently. Fixing `process_sample` becomes its own slice afterward.
4. **TS fixture list enumerated.** Original plan named Rust test fixtures but missed TS ones. **Revised:** Commit 2 explicitly lists TS test files needing updates.

Plus two smaller revisions:

5. **Visual smoke scope corrected.** Original mentioned no specific viewport; Codex flagged 1366×768/1600×940. **Revised:** primary smoke at 1920×1080 (per `src-tauri/tauri.conf.json:17-18` native window default), plus regression check at 1366×768 (existing CSS responsive floor).
6. **Album character bias scope clarified.** `apply_album_shadow` at `album_render.rs:237` currently only biases low/low-mid/mid/high. The 3 new bands stay 4-band on the album-character side. Added to Out of Scope as a conscious deferral.

---

## Scope

Add 3 new biquad stages to the DSP chain and surface them as 3 new drag-only nodes on the Visual EQ. Total user-facing band count goes from 4 to 7.

| Band | Frequency | Filter type | Default Q | UI surface |
|---|---|---|---|---|
| Sub (new) | 80 Hz | Peaking bell | 0.8 | Visual EQ, drag-only (secondary) |
| Low | 200 Hz | Low-shelf | slope 0.7 (existing) | Knob + Visual EQ (primary) |
| Low-Mid | 400 Hz | Peaking bell | 0.9 (existing) | Visual EQ, drag-only (secondary) |
| Mid | 1500 Hz | Peaking bell | 0.8 (existing) | Knob + Visual EQ (primary) |
| High-Mid (new) | 3500 Hz | Peaking bell | 0.9 | Visual EQ, drag-only (secondary) |
| High | 6000 Hz | High-shelf | slope 0.7 (existing) | Knob + Visual EQ (primary) |
| Sparkle (new) | 12 000 Hz | High-shelf | slope 0.7 | Visual EQ, drag-only (secondary) |

**Naming:** 12 kHz band is `sparkle`, not `air` — codebase already has `PresetCalibration.air_db` (6 kHz High baseline) and `AdvancedSettings.presence_air` (10 kHz Advanced shelf); a third "air" would muddy disambiguation.

**Chain order after this slice** (`process_frame_inplace`):

```
input_gain → sub_highpass×2 → sub → low → low_mid → mid → high_mid → high → sparkle
            → warmth → presence_air → [compressor] → [transient] → [width] → [sat] → limiter → ...
```

**Chain order in `process_sample`** (preserves existing low_mid skip — see §process_sample divergence below):

```
input_gain → sub_highpass×2 → sub → low → mid → high_mid → high → sparkle
            → warmth → presence_air → ...
```

**All 8 presets default to 0.0 dB for the 3 new bands.** Listening tune-up is a separate slice owned by Dan.

---

## `process_sample` divergence (pre-existing — preserve, don't fix)

Codex's read at `dsp.rs:2057-2063` is correct: `process_sample` currently runs `sub_hp1 → sub_hp2 → low → mid → high → warmth → presence_air`, skipping `state.low_mid` between `state.low` (line 2059) and `state.mid` (line 2060). This is a pre-existing latent divergence vs. `process_frame_inplace` (`dsp.rs:1822-1827`) which has all 4 bands.

**This slice does NOT fix the divergence.** Reasons:

- Fixing `process_sample` to include low_mid changes its audio output, which would break the byte-identity gate this slice depends on.
- The pre-existing bug deserves a dedicated slice with its own byte-identity-change accepted explicitly.
- Conflating "extend chain by 3 bands" with "fix a different pre-existing bug" muddies the diff and the rollback story.

**What this slice does instead:**

- Adds `sub`, `high_mid`, `sparkle` to `process_sample` at their frequency-monotonic positions, **without** adding `low_mid`.
- Adds a guard test (Commit 1): `process_sample_intentionally_skips_low_mid_until_separate_fix_slice` — pins the divergence so a future reader doesn't accidentally fix it without thinking about the byte-identity consequences.

**Follow-up slice (out of scope for this plan):** restore `low_mid` to `process_sample`, accept the byte-identity change, document the corrected behavior. Likely needs a fresh per-preset SHA snapshot since output bytes will change for any caller of `process_sample`.

---

## DSP changes (Rust, `src-tauri/src/dsp.rs`)

### 1. `PresetCalibration` extension — `dsp.rs:280-335`

Add 3 fields. Keep the EQ cluster grouped.

```rust
pub struct PresetCalibration {
    /// 80 Hz peaking baseline gain in dB. Drag-only on Visual EQ. Adds to `eq_sub_db`.
    pub sub_db: f32,                   // NEW
    pub low_shelf_db: f32,
    pub low_mid_db: f32,
    pub presence_db: f32,
    /// 3.5 kHz peaking baseline gain in dB. Drag-only on Visual EQ. Adds to `eq_high_mid_db`.
    pub high_mid_db: f32,              // NEW
    pub air_db: f32,
    /// 12 kHz high-shelf baseline gain in dB. Distinct from `air_db` (6 kHz)
    /// and `AdvancedSettings.presence_air` (10 kHz). Adds to `eq_sparkle_db`.
    pub sparkle_db: f32,               // NEW
    pub warmth: f32,
    // ... rest unchanged
}
```

**Default values for all 9 preset constants:** `sub_db = high_mid_db = sparkle_db = 0.0` across the board (Universal, Clarity, Tape, Spatial, Oomph, Warmth, Punch, Loud, Custom). No tuning guesses.

### 2. `ChainCoeffs` extension — `dsp.rs:578-666`

```rust
pub struct ChainCoeffs {
    pub sub_highpass: BiquadCoeffs,
    pub sub: BiquadCoeffs,             // NEW
    pub low: BiquadCoeffs,
    pub low_mid: BiquadCoeffs,
    pub mid: BiquadCoeffs,
    pub high_mid: BiquadCoeffs,        // NEW
    pub high: BiquadCoeffs,
    pub sparkle: BiquadCoeffs,         // NEW
    pub warmth: BiquadCoeffs,
    pub presence_air: BiquadCoeffs,
    // ... rest unchanged
}
```

### 3. `ChannelState` extension — `dsp.rs:1114-1135`

```rust
pub struct ChannelState {
    sub_hp1: BiquadState,
    sub_hp2: BiquadState,
    sub: BiquadState,                  // NEW
    low: BiquadState,
    low_mid: BiquadState,
    mid: BiquadState,
    high_mid: BiquadState,             // NEW
    high: BiquadState,
    sparkle: BiquadState,              // NEW
    warmth: BiquadState,
    presence_air: BiquadState,
    // ... rest unchanged
}
```

### 4. `ChainCoeffs::from_settings` extension — `dsp.rs:701-720`

```rust
let effective_sub_db      = preset.sub_db      * preset_scale + settings.eq_sub_db;
let effective_low_db      = preset.low_shelf_db * preset_scale + settings.eq_low_db;
let effective_low_mid_db  = preset.low_mid_db   * preset_scale + settings.eq_low_mid_db;
let effective_mid_db      = preset.presence_db  * preset_scale + settings.eq_mid_db;
let effective_high_mid_db = preset.high_mid_db  * preset_scale + settings.eq_high_mid_db;
let effective_high_db     = preset.air_db       * preset_scale + settings.eq_high_db;
let effective_sparkle_db  = preset.sparkle_db   * preset_scale + settings.eq_sparkle_db;

// ... existing sub_highpass construction ...

let sub      = BiquadCoeffs::peaking   (sr, 80.0,    0.8, effective_sub_db);
let low      = BiquadCoeffs::low_shelf (sr, 200.0,   effective_low_db,      0.7);
let low_mid  = BiquadCoeffs::peaking   (sr, 400.0,   0.9, effective_low_mid_db);
let mid      = BiquadCoeffs::peaking   (sr, 1500.0,  0.8, effective_mid_db);
let high_mid = BiquadCoeffs::peaking   (sr, 3500.0,  0.9, effective_high_mid_db);
let high     = BiquadCoeffs::high_shelf(sr, 6000.0,  effective_high_db,     0.7);
let sparkle  = BiquadCoeffs::high_shelf(sr, 12000.0, effective_sparkle_db,  0.7);
```

The ChainCoeffs struct literal at the bottom of `from_settings` needs the 3 new fields added.

### 5. Chain order extensions

**`process_frame_inplace` at `dsp.rs:1817-1828`:**

```rust
for ch in 0..channels {
    let state = &mut self.states[ch];
    let mut y = frame[ch] * self.coeffs.input_gain_lin;
    let hp1 = state.sub_hp1.process(&self.coeffs.sub_highpass, y);
    y = state.sub_hp2.process(&self.coeffs.sub_highpass, hp1);
    y = state.sub.process(&self.coeffs.sub, y);             // NEW
    y = state.low.process(&self.coeffs.low, y);
    y = state.low_mid.process(&self.coeffs.low_mid, y);
    y = state.mid.process(&self.coeffs.mid, y);
    y = state.high_mid.process(&self.coeffs.high_mid, y);   // NEW
    y = state.high.process(&self.coeffs.high, y);
    y = state.sparkle.process(&self.coeffs.sparkle, y);     // NEW
    y = state.warmth.process(&self.coeffs.warmth, y);
    y = state.presence_air.process(&self.coeffs.presence_air, y);
    frame[ch] = y;
}
```

**`process_sample` at `dsp.rs:2057-2063` — DELIBERATELY preserves the existing `low_mid` skip:**

```rust
let hp1 = state.sub_hp1.process(&self.coeffs.sub_highpass, y);
y = state.sub_hp2.process(&self.coeffs.sub_highpass, hp1);
y = state.sub.process(&self.coeffs.sub, y);                 // NEW
y = state.low.process(&self.coeffs.low, y);
// NOTE: state.low_mid intentionally skipped here. This mirrors the
// pre-existing divergence vs process_frame_inplace at dsp.rs:1823.
// Fixing this divergence is a separate slice with its own byte-
// identity change accepted explicitly. See guard test below.
y = state.mid.process(&self.coeffs.mid, y);
y = state.high_mid.process(&self.coeffs.high_mid, y);       // NEW
y = state.high.process(&self.coeffs.high, y);
y = state.sparkle.process(&self.coeffs.sparkle, y);         // NEW
y = state.warmth.process(&self.coeffs.warmth, y);
y = state.presence_air.process(&self.coeffs.presence_air, y);
```

---

## State model changes

### 6. `MasteringSettings` (Rust, `src-tauri/src/types.rs:469-515`)

```rust
pub struct MasteringSettings {
    pub preset: Preset,
    pub intensity: f32,
    #[serde(default)]
    pub eq_sub_db: f32,              // NEW
    pub eq_low_db: f32,
    pub eq_low_mid_db: f32,
    pub eq_mid_db: f32,
    #[serde(default)]
    pub eq_high_mid_db: f32,         // NEW
    pub eq_high_db: f32,
    #[serde(default)]
    pub eq_sparkle_db: f32,          // NEW
    // ... rest unchanged
}
```

`#[serde(default)]` ensures saved projects from before the slice deserialize with 0.0 for the new fields.

### 7. `MasteringSettings` TypeScript shape (`src/bindings.ts:147-173`)

```typescript
export interface MasteringSettings {
  preset: Preset;
  intensity: number;
  eq_sub_db: number;                 // NEW
  eq_low_db: number;
  eq_low_mid_db: number;
  eq_mid_db: number;
  eq_high_mid_db: number;            // NEW
  eq_high_db: number;
  eq_sparkle_db: number;             // NEW
  // ... rest unchanged
}
```

### 8. `useTrackMaster.ts` defaults + setter (`src/hooks/useTrackMaster.ts:41-44, 856-869`)

Defaults: add `eq_sub_db: 0, eq_high_mid_db: 0, eq_sparkle_db: 0`.

Setter widening:

```typescript
const setEqBand = useCallback(
  (band: "sub" | "low" | "low-mid" | "mid" | "high-mid" | "high" | "sparkle",
   db: number) => {
    if (!selectedTrackId) return;
    updateSettings(selectedTrackId, (prev) => {
      const next = { ...prev };
      if      (band === "sub")      next.eq_sub_db      = db;
      else if (band === "low")      next.eq_low_db      = db;
      else if (band === "low-mid")  next.eq_low_mid_db  = db;
      else if (band === "mid")      next.eq_mid_db      = db;
      else if (band === "high-mid") next.eq_high_mid_db = db;
      else if (band === "high")     next.eq_high_db     = db;
      else                          next.eq_sparkle_db  = db; // "sparkle"
      return next;
    });
  },
  [selectedTrackId, updateSettings],
);
```

### 9. `App.tsx` `Macros` `onEq` prop signature (`App.tsx:1388`)

Widen the band union to the 7-band tuple. The 3 existing knob `onChange` calls at `App.tsx:1432, 1444, 1456` keep passing `"low"`, `"mid"`, `"high"`. New bands enter only through `VisualEqPanel` drag callbacks.

---

## Visual EQ component changes (`src/components/VisualEqPanel.tsx`)

### 10. `BANDS` constant extension (lines 46-51)

```typescript
type BandId = "sub" | "low" | "low-mid" | "mid" | "high-mid" | "high" | "sparkle";
type BandTier = "primary" | "secondary";

interface Band {
  id: BandId;
  label: string;
  hz: number;
  color: string;
  kind: BandKind;
  tier: BandTier;          // NEW
  qOctaves: number;
}

const BANDS: readonly Band[] = [
  { id: "sub",      label: "SUB",      hz: 80,    color: "TBD",     kind: "peak",       tier: "secondary", qOctaves: 1.2 },
  { id: "low",      label: "LOW",      hz: 200,   color: "#22d3ee", kind: "shelf-low",  tier: "primary",   qOctaves: 0   },
  { id: "low-mid",  label: "LOW-MID",  hz: 400,   color: "#4ade80", kind: "peak",       tier: "secondary", qOctaves: 1.0 },
  { id: "mid",      label: "MID",      hz: 1500,  color: "#a78bfa", kind: "peak",       tier: "primary",   qOctaves: 1.2 },
  { id: "high-mid", label: "HIGH-MID", hz: 3500,  color: "TBD",     kind: "peak",       tier: "secondary", qOctaves: 1.0 },
  { id: "high",     label: "HIGH",     hz: 6000,  color: "#60a5fa", kind: "shelf-high", tier: "primary",   qOctaves: 0   },
  { id: "sparkle",  label: "SPARKLE",  hz: 12000, color: "TBD",     kind: "shelf-high", tier: "secondary", qOctaves: 0   },
];
```

### 11. Visual hierarchy

| Property | Primary (knob-bound) | Secondary (drag-only) |
|---|---|---|
| Node radius | 8 | 5 |
| Hit-target radius | 18 | 18 |
| Fill opacity | 1.0 | 0.85 |
| Outline halo | 1.5px ring | none |
| Label opacity | 1.0 | 0.85 |
| Drag interaction | Vertical | Vertical |
| Double-click reset | Yes | Yes |

Functional differentiation: primary bands have two control surfaces (knob + Visual EQ); secondary bands have one (Visual EQ only). Both kinds are legitimate user-shaping territory — visual hierarchy reflects surface, not opinion about use.

### 12. Visual smoke check (REQUIRED before Commit 3 ships)

- **Primary viewport: 1920×1080** (current Tauri native window default per `src-tauri/tauri.conf.json:17-18`).
- **Floor regression: 1366×768** (existing CSS responsive floor per `App.css:1618`).
- **Mid check: 1600×940** (existing layout-revision reference per `docs/UI_LAYOUT_REVISION_1600x940.md`).

Check items at each viewport:

- All 7 nodes render without label overlap.
- 80 Hz and 12 kHz nodes don't collide with the panel edges.
- Primary vs secondary visual differentiation reads at a glance.
- Compact embedded mode (the variant used inside the Macros row) is the most space-constrained — verify there especially.

### 13. Response curve renderer extension

`totalResponseDb` iterates over `BANDS` and sums each band's contribution. Extending `BANDS` to 7 entries means the renderer extends automatically — `bandResponseDb` already handles `peak`/`shelf-low`/`shelf-high` kinds. `N_SAMPLES = 180` is still smooth at 7 bands.

---

## Test surface

### 14. New per-band frequency-response tests (`dsp.rs` `#[cfg(test)] mod tests`)

Mirror `low_mid_band_centred_at_400hz_with_q_point_9` at `dsp.rs:3394`:

- `sub_band_centred_at_80hz_with_q_point_8`
- `high_mid_band_centred_at_3500hz_with_q_point_9`
- `sparkle_band_centred_at_12khz_high_shelf_slope_point_7`

### 15. Guard test for `process_sample` divergence

```rust
#[test]
fn process_sample_intentionally_skips_low_mid_until_separate_fix_slice() {
    // This test pins the pre-existing divergence between process_sample
    // and process_frame_inplace. process_sample currently runs the chain
    // WITHOUT state.low_mid (dsp.rs:2057-2063), while process_frame_inplace
    // includes it (dsp.rs:1822-1827).
    //
    // The 7-band EQ expansion deliberately preserved this divergence
    // because fixing it would change byte output for any caller of
    // process_sample and break the byte-identity gate.
    //
    // A future slice will fix this divergence as a dedicated change with
    // its own byte-identity-change accepted explicitly. Until then, this
    // test exists to prevent silent drift.
    //
    // To remove this test: do so as part of the slice that adds low_mid
    // back into process_sample. Update per-preset SHA snapshots in
    // wav_writer-style at the same time.

    // Test body: feed a synthetic impulse through both paths with a non-
    // identity low_mid coefficient (e.g. +6 dB at 400 Hz) and assert the
    // outputs DIFFER at the expected magnitude. If they ever match
    // unexpectedly, the divergence was fixed (intentionally or not) and
    // this test should be removed alongside the SHA update.
}
```

### 16. Slow-lane gate downgraded to per-preset chain coefficient SHAs

The original plan claimed "slow-lane byte-identical fixture output" — that overstated what the slow lane provides. The slow lane runs the real fixture through `mastering_render_with_progress` and measures LUFS / metering, but doesn't snapshot bytes (`tests/contracts.rs:604` logs file size, not hash).

**Revised gate** (established in Commit 0): per-preset SHA snapshots of `process_frame_inplace` output on a fixed synthetic input (deterministic pink noise via `synth_pink_stereo` — the existing pattern at `tests/preset_distinctness.rs:68`). After the 7-band extension, all SHAs must match — proves the chain's audible behavior is unchanged for every preset.

**Determinism confirmed.** `synth_pink_stereo` at `tests/preset_distinctness.rs:68-106` uses a fixed-seed LCG (`let mut state: u32 = 0xCAFE_BABE` with constants `1_103_515_245` / `12345`). Same input args produce the same output bytes across runs. The Commit 0 SHA strategy is safe to lean on.

**Why not real-fixture SHAs:**

- Real fixtures live in private-audio-fixtures/ (not committed); SHAs would only verify on machines with the fixture.
- Synthetic input is portable and runs in fast lane.
- The proof "new biquads at 0 dB are identity" is mathematical — synthetic input is sufficient to verify; real input doesn't strengthen the proof.

**Cross-platform note:** the chain contains `tanh` in the saturation stage (`dsp.rs:1867+`), which CAN be platform-dependent depending on the libm implementation. **Verify empirically first** — Rust's `f32::tanh` may already be cross-platform deterministic on standard targets, in which case the concern is moot. Commit 0 acceptance includes running on both Mac and Windows; if SHAs match, no gating needed; if they diverge, OS-gated `#[cfg(target_os = "...")]` constants are acceptable in-slice (same pattern as the existing `#[cfg(target_os = "windows")]` test at `engine.rs`). **A portable tanh implementation (polynomial approximation, `libm::tanhf`, etc.) is explicitly out of scope for this slice** — it would change current audio output and is its own DSP-output-changing slice to handle deliberately.

### 17. Existing Rust test files needing fixture updates

Every file that constructs `MasteringSettings` with explicit `eq_*_db` fields needs the 3 new fields added (all at 0.0):

- `tests/album_arc_trace.rs:31-34`
- `tests/album_character_bias.rs:34-37`
- `tests/album_plan_landing.rs:52-55`
- `tests/album_render.rs:30-33`
- `tests/album_simple_landing.rs:55-58`
- `tests/contracts.rs:1776-1779` (and any other construction sites)
- `tests/delivery_profile_render.rs:51-54`
- `tests/dither_absence_of_harmonics.rs:37-40`
- `tests/export_volume_match.rs:40-43`
- `tests/preset_distinctness.rs:191-194`
- `tests/preset_loudness_balance.rs:92-95`
- `tests/preset_signature.rs:114-117`
- In-file `types.rs:835+` test fixtures.

If a `Default` impl exists on `MasteringSettings`, the struct-update-syntax (`..Default::default()`) callers get the new fields free — only explicit-construction sites need updating.

### 18. Existing TypeScript test files needing fixture updates

`grep "MasteringSettings" src/` to enumerate exhaustively. Known sites from Codex's review:

- `src/lib/compressor-auto.test.ts`
- `src/lib/effective-settings.test.ts`
- `src/lib/settings-transitions.test.ts`
- `src/components/SignalChain.test.tsx`
- `src/hooks/useTrackMaster.integration.test.tsx`
- `src/App.loudness-target.test.tsx`
- `src/App.preset-save.test.tsx`

Plus any helpers like `src/lib/preview-mock.ts` that construct `MasteringSettings` literals. `npm test` will catch missed sites via TypeScript build errors, but the plan lists them so the diff scope is honest.

---

## Commit shape (REVISED — 4 commits)

### Commit 0 — Pre-flight: per-preset chain-output SHA snapshots

**Purpose:** establish the byte-identity gate before any DSP changes.

- New test module `src-tauri/src/dsp.rs` `#[cfg(test)] mod preset_byte_identity` (or similar location).
- For each of 9 presets (Universal, Clarity, Tape, Spatial, Oomph, Warmth, Punch, Loud, Custom): render a fixed deterministic synthetic input (e.g. `synth_pink_stereo(48_000, 0.3)` for 1s of pink noise at -10 dBFS peak) through `MasteringChain::process_frame_inplace`, capture rendered output, compute SHA-256 over the f32 bytes.
- Pin SHA as a string constant per preset.
- Test: each preset's rendered output hashes to the pinned constant.
- Verification on macOS first; flag any Mac/Windows divergence empirically and gate with `#[cfg(target_os = "...")]` if needed.

**Verification:** `cargo test preset_byte_identity` — 9/9 pass. Fast lane.

**Commit message:** `Phase B.0: per-preset chain-output SHA snapshots` with verification block.

### Commit 1 — Rust state + DSP extension

- `MasteringSettings` Rust extension (3 new fields, `#[serde(default)]`).
- `PresetCalibration` extension (3 new fields, all 9 preset defaults 0.0).
- `ChainCoeffs` + `ChannelState` extensions.
- `from_settings` extension (effective_*_db + new biquad construction).
- Chain order extension in `process_frame_inplace`.
- Chain order extension in `process_sample` (PRESERVES low_mid skip).
- New per-band frequency-response tests (sub/high_mid/sparkle).
- Guard test for `process_sample` divergence (§15).
- Rust test fixture updates (§17).

**Verification:**

- `cargo test` (fast lane): all pass, including new per-band tests, guard test, and the Commit 0 SHA snapshots **unchanged**.
- `AMS_RUN_REAL_FIXTURE=1 cargo test` (slow lane): all pass (metering snapshots unchanged within tolerance).

If any Commit 0 SHA differs, the implementation is wrong — stop and debug.

### Commit 2 — TS state + setter + Macros prop + TS fixture churn

- `bindings.ts` MasteringSettings extension.
- `useTrackMaster.ts` defaults + `setEqBand` widening.
- `App.tsx` Macros `onEq` prop widening.
- TS test fixture updates (§18, plus any others surfaced by `grep`).

**Verification:** `npm test` + `npm run build` (TypeScript build catches any missed fixture). Fast lane only — no audio path changes.

### Commit 3 — Visual EQ component

- `VisualEqPanel.tsx` `BANDS` extension with `tier` field.
- Visual hierarchy implementation (primary vs secondary node sizes, opacities, halo).
- Color choices for the 3 new bands.
- Visual smoke at 1920×1080 (primary), 1600×940 (mid), 1366×768 (floor) — see §12.

**Verification:** `npm test` + `npm run build` + manual visual smoke at all three viewports.

---

## Flagged uncertainties

### 1. Color choices for the 3 new bands — Commit 3 implementer picks (not "TBD" at ship)

Existing 4 colors are hardcoded hex values, not drawn from `KnobTone`. The "TBD" placeholders in the §10 BANDS example are illustrative only — **Commit 3 must ship with real hex values, not "TBD" strings.** The Commit 3 implementer picks final colors at implementation time from the existing palette family (the inline hex values in `BANDS` and `TONE_COLOR` at `Knob.tsx:30+`) and ships them. Dan can adjust in a tiny follow-up slice if any choice doesn't sit right after seeing it rendered.

Starting suggestions for the implementer to riff from (NOT prescriptive):

- `sub` (80 Hz): muted blue/slate — deep/foundation
- `high-mid` (3500 Hz): muted amber/gold — warm-mid between purple and blue
- `sparkle` (12 kHz): pale gold or pale pink — top-end shimmer

**Hard rule: Commit 3 ships with non-placeholder hex values.** If the implementer can't make a confident choice, surface that as a chat checkpoint before pushing Commit 3 — don't ship "TBD".

### 2. Sparkle slope at 12 kHz (0.7 vs 0.5)

Plan defaults to slope 0.7 to match existing 6 kHz / 10 kHz convention. Gentler slope (0.5) might shape better at 12 kHz but it's speculation. Conservative default holds; reconsider in future tuning pass.

### 3. Sparkle naming

`sparkle` resolves the three-way "air" collision. Alternatives: `top`, `shimmer`, `extreme-high`. If Dan prefers another name, the rename is mechanical.

### 4. Cross-platform SHA portability

`tanh` in saturation MAY produce different bits on Mac vs Windows. **Verify empirically first** (Commit 0 runs on both platforms). If SHAs match cross-platform, no gating needed. If they diverge, OS-gated `#[cfg(target_os = "...")]` constants are acceptable in-slice. **Portable tanh (polynomial approximation, `libm::tanhf`) is explicitly out of scope** — that's a DSP-output-changing slice to land deliberately later. See §16 for full sequencing.

### 5. `PresetCalibration` field ordering

The struct has `#[derive(Debug, Clone, Copy)]` only at `dsp.rs:279`, no `Deserialize` — field-order is cosmetic, not serde-load-bearing. Worth a quick grep to confirm `PresetCalibration` isn't serialized anywhere; if it is, field-order matters.

### 6. `qOctaves` values for sub and high-mid in `VisualEqPanel`

Plan uses `sub` qOctaves=**1.2** and `high-mid` qOctaves=1.0, derived from the existing mapping precedent: low_mid (DSP Q=0.9) → qOctaves=1.0; mid (DSP Q=0.8) → qOctaves=1.2. Since sub shares DSP Q=0.8 with mid, it inherits mid's qOctaves=1.2 (originally drafted as 1.1; corrected after Codex caught the math). high_mid shares DSP Q=0.9 with low_mid, inheriting qOctaves=1.0. Both values affect the visual response-curve approximation only, not audio. The formula at `VisualEqPanel.tsx:97` (`sigma = qOctaves * 0.5 / 2.355`) confirms qOctaves directly represents the Gaussian's FWHM in octaves.

---

## Out of scope (sanity check list)

NOT part of this slice:

- **Freq/Q sweep (horizontal drag on Visual EQ).** Stays disabled per `VisualEqPanel.tsx:14-18`.
- **Surfacing preset baselines on the Visual EQ.** Nodes still show user offsets only.
- **New knobs in the Tone Shape row.** Stays at exactly 3 (Low / Mid / High).
- **Promoting `sub_highpass` to user-facing surface.** Stays preset-locked.
- **Touching `warmth` (300 Hz Advanced shelf) or `presence_air` (10 kHz Advanced shelf).** Out of scope.
- **Per-preset tuning of the new bands.** All defaults 0.0 dB. Tuning is a separate listening-batch slice.
- **`science_note` tooltip on preset orbs.** Separate future slice.
- **Product positioning copy in README/onboarding.** Separate future slice.
- **Fixing `process_sample`'s pre-existing `low_mid` skip.** Separate slice afterward (see §process_sample divergence).
- **Extending `apply_album_shadow` to the 3 new bands.** `album_render.rs:237` currently biases low/low-mid/mid/high only; the 3 new bands stay 4-band on the album-character side. **In this slice the new bands are user offsets only** — preset baselines default to 0.0 dB AND there's no album-character bias, so the new bands contribute only what the user drags in. The deferral is audibly inert at slice-land (0.0 + 0.0 = 0.0). It only becomes audibly visible after a separate listening slice tunes per-preset Sub/High-Mid/Sparkle baselines; that slice should reconsider extending `apply_album_shadow` to the new bands in the same pass.

---

## Verification path before merge

1. **Commit 0:** `cargo test preset_byte_identity` — 9/9 pass on macOS. If Windows is available, run there too and gate any divergent SHAs with `cfg(target_os)`.
2. **Commit 1 fast lane:** `cargo test --lib` from `src-tauri/`. New per-band tests + guard test + all existing tests pass. Commit 0 SHAs unchanged.
3. **Commit 1 slow lane:** `AMS_RUN_REAL_FIXTURE=1 cargo test`. All pass. Metering snapshots within existing tolerance.
4. **Commit 2 fast lane:** `npm test` + `npm run build` + `cargo test`. State plumbing only; no audio path. Existing tests pass unchanged.
5. **Commit 3 fast lane:** `npm test` + `npm run build` + visual smoke at 1920×1080, 1600×940, 1366×768.
6. **Final slow lane after commit 3:** `AMS_RUN_REAL_FIXTURE=1 cargo test` to confirm end-to-end. Commit 0 SHAs still unchanged.

Per CLAUDE.md commit-shape convention, each commit includes a `Verification:` block in its message with the specific command outputs.

---

*Drafted by Vera; revised to incorporate Codex review pushbacks (compile order, byte-identity gate, `process_sample` divergence, TS fixture list, sub qOctaves math, secondary label opacity, color-decision sequencing, tanh portability scoping). Awaiting Dan's approval before implementation begins.*

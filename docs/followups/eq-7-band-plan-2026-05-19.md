# Plan — 7-Band EQ Expansion (4 → 7 user-facing bands)

**Date:** 2026-05-19
**Status:** Plan-doc per ADR 0002. Implementation gated on Dan's approval.
**Drives from:** `docs/eq-7-band-plan-prompt-2026-05-19.md`
**Reference:** `docs/EQ_ARCHITECTURE_RECON_2026-05-19.md`

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

**Naming:** the 12 kHz band is `sparkle`, not `air` — the codebase already has `PresetCalibration.air_db` (6 kHz High baseline) and `AdvancedSettings.presence_air` (10 kHz Advanced shelf); a third "air" reference would muddy disambiguation.

**Chain order after this slice** (`process_frame_inplace` / `process_sample`):

```
input_gain → sub_highpass×2 → sub → low → low_mid → mid → high_mid → high → sparkle
            → warmth → presence_air → [compressor] → [transient] → [width] → [sat] → limiter → ...
```

Sub before Low. High-Mid between Mid and High. Sparkle after High. `warmth` (300 Hz) and `presence_air` (10 kHz) stay in their current post-EQ positions per the existing chain comment at `dsp.rs:1815-1816` — they're a "post-tonal-shape voicing pass," not part of the main monotonic EQ.

**All 8 presets default to 0.0 dB for the 3 new bands.** Listening tune-up is a separate slice owned by Dan. Slow-lane fixture output must be **byte-identical** to pre-change for every preset (new biquads at 0 dB are identity; any drift means the implementation is wrong).

---

## DSP changes (Rust, `src-tauri/src/dsp.rs`)

### 1. `PresetCalibration` extension — `dsp.rs:280-335`

Add 3 fields. Insertion order: keep the existing EQ block grouped together. Suggested placement: `sub_db` before `low_shelf_db`; `high_mid_db` between `presence_db` and `air_db`; `sparkle_db` after `air_db`.

```rust
pub struct PresetCalibration {
    /// 80 Hz peaking baseline gain in dB. Drag-only band on the Visual EQ
    /// (no Tone Shape knob). Adds to `eq_sub_db`.
    pub sub_db: f32,                   // NEW
    pub low_shelf_db: f32,
    pub low_mid_db: f32,
    pub presence_db: f32,
    /// 3.5 kHz peaking baseline gain in dB. Drag-only band on the Visual EQ
    /// (no Tone Shape knob). Adds to `eq_high_mid_db`.
    pub high_mid_db: f32,              // NEW
    pub air_db: f32,
    /// 12 kHz high-shelf baseline gain in dB. Distinct from
    /// `AdvancedSettings.presence_air` (10 kHz) and `air_db` (6 kHz High).
    /// Drag-only band on the Visual EQ (no Tone Shape knob). Adds to
    /// `eq_sparkle_db`.
    pub sparkle_db: f32,               // NEW
    pub warmth: f32,
    // ... rest unchanged
}
```

**Default values for all 9 preset constants** (`dsp.rs:340-561`, including Custom):

| Preset | `sub_db` | `high_mid_db` | `sparkle_db` |
|---|---|---|---|
| Universal | 0.0 | 0.0 | 0.0 |
| Clarity | 0.0 | 0.0 | 0.0 |
| Tape | 0.0 | 0.0 | 0.0 |
| Spatial | 0.0 | 0.0 | 0.0 |
| Oomph | 0.0 | 0.0 | 0.0 |
| Warmth | 0.0 | 0.0 | 0.0 |
| Punch | 0.0 | 0.0 | 0.0 |
| Loud | 0.0 | 0.0 | 0.0 |
| Custom (Neutral) | 0.0 | 0.0 | 0.0 |

No tuning guesses. Listening calibration is a follow-up slice.

### 2. `ChainCoeffs` extension — `dsp.rs:578-666`

Add 3 fields. Keep the EQ-cluster grouping intact.

```rust
pub struct ChainCoeffs {
    pub sub_highpass: BiquadCoeffs,
    /// Phase B: 80 Hz peaking. Drag-only on Visual EQ.
    pub sub: BiquadCoeffs,             // NEW
    pub low: BiquadCoeffs,
    pub low_mid: BiquadCoeffs,
    pub mid: BiquadCoeffs,
    /// Phase B: 3.5 kHz peaking. Drag-only on Visual EQ.
    pub high_mid: BiquadCoeffs,        // NEW
    pub high: BiquadCoeffs,
    /// Phase B: 12 kHz high-shelf. Drag-only on Visual EQ. Distinct from
    /// `presence_air` (10 kHz, Advanced panel).
    pub sparkle: BiquadCoeffs,         // NEW
    pub warmth: BiquadCoeffs,
    pub presence_air: BiquadCoeffs,
    // ... rest unchanged
}
```

### 3. `ChannelState` extension — `dsp.rs:1114-1135`

Add 3 `BiquadState` fields matching the new coefficients.

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

`BiquadState::default()` initializes to silence (z₁ = z₂ = 0), so the new states are byte-identical to "the band wasn't there" until the coefficients become non-identity.

### 4. `ChainCoeffs::from_settings` extension — `dsp.rs:701-720`

Mirror the existing 4-band pattern. Three new `effective_*_db` calculations and three new `BiquadCoeffs::*` constructions.

```rust
// Phase B: 3 new bands. Pattern matches existing 4-band runtime mapping.
//   preset.sub_db        → 80 Hz peaking  (drag-only)
//   preset.high_mid_db   → 3.5 kHz peaking (drag-only)
//   preset.sparkle_db    → 12 kHz high-shelf (drag-only)
let effective_sub_db      = preset.sub_db      * preset_scale + settings.eq_sub_db;
let effective_low_db      = preset.low_shelf_db * preset_scale + settings.eq_low_db;
let effective_low_mid_db  = preset.low_mid_db   * preset_scale + settings.eq_low_mid_db;
let effective_mid_db      = preset.presence_db  * preset_scale + settings.eq_mid_db;
let effective_high_mid_db = preset.high_mid_db  * preset_scale + settings.eq_high_mid_db;
let effective_high_db     = preset.air_db       * preset_scale + settings.eq_high_db;
let effective_sparkle_db  = preset.sparkle_db   * preset_scale + settings.eq_sparkle_db;

// ... existing sub_highpass construction ...

let sub      = BiquadCoeffs::peaking   (sr, 80.0,   0.8, effective_sub_db);
let low      = BiquadCoeffs::low_shelf (sr, 200.0,  effective_low_db,      0.7);
let low_mid  = BiquadCoeffs::peaking   (sr, 400.0,  0.9, effective_low_mid_db);
let mid      = BiquadCoeffs::peaking   (sr, 1500.0, 0.8, effective_mid_db);
let high_mid = BiquadCoeffs::peaking   (sr, 3500.0, 0.9, effective_high_mid_db);
let high     = BiquadCoeffs::high_shelf(sr, 6000.0, effective_high_db,     0.7);
let sparkle  = BiquadCoeffs::high_shelf(sr, 12000.0, effective_sparkle_db, 0.7);
```

The ChainCoeffs constructor at the bottom of `from_settings` (around `dsp.rs:1043+`) needs the 3 new fields added to its struct literal. Mirror the existing field order.

### 5. Chain order extension — `dsp.rs:1817-1828` (`process_frame_inplace`) and `dsp.rs:2049-2065` (`process_sample`)

Insert the 3 new biquad processes in frequency-monotonic order.

```rust
// process_frame_inplace:
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

Mirror the same insertions in `process_sample` at `dsp.rs:2057-2063`.

---

## State model changes

### 6. `MasteringSettings` (Rust, `src-tauri/src/types.rs:469-515`)

Add 3 new fields with `#[serde(default)]` so old saved presets/projects deserialize cleanly (defaulting to 0.0).

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

The `Default` impl for `MasteringSettings` (if one exists; otherwise wherever the test fixtures construct defaults around `types.rs:835`+) also needs the 3 new fields set to 0.0.

### 7. `MasteringSettings` TypeScript shape (`src/bindings.ts:147-173`)

```typescript
export interface MasteringSettings {
  preset: Preset;
  intensity: number;
  /// Phase B — user offset on top of the preset's sub baseline
  /// (80 Hz peaking @ Q=0.8). 0 = use preset value as-is. Drag-only.
  eq_sub_db: number;                 // NEW
  eq_low_db: number;
  eq_low_mid_db: number;
  eq_mid_db: number;
  /// Phase B — user offset on top of the preset's high-mid baseline
  /// (3.5 kHz peaking @ Q=0.9). 0 = use preset value as-is. Drag-only.
  eq_high_mid_db: number;            // NEW
  eq_high_db: number;
  /// Phase B — user offset on top of the preset's sparkle baseline
  /// (12 kHz high-shelf). 0 = use preset value as-is. Drag-only.
  eq_sparkle_db: number;             // NEW
  // ... rest unchanged
}
```

### 8. `useTrackMaster.ts` defaults + setter

**Defaults** (`useTrackMaster.ts:41-44`):

```typescript
eq_sub_db: 0,                        // NEW
eq_low_db: 0,
eq_low_mid_db: 0,
eq_mid_db: 0,
eq_high_mid_db: 0,                   // NEW
eq_high_db: 0,
eq_sparkle_db: 0,                    // NEW
```

**Setter** (`useTrackMaster.ts:856-869`):

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

Widen the band union:

```typescript
onEq: (
  band: "sub" | "low" | "low-mid" | "mid" | "high-mid" | "high" | "sparkle",
  db: number,
) => void;
```

The 3 existing knob `onChange` calls at `App.tsx:1432, 1444, 1456` continue to pass `"low"`, `"mid"`, `"high"` — unchanged. The new bands enter only through `VisualEqPanel`'s drag callbacks.

---

## Visual EQ component changes (`src/components/VisualEqPanel.tsx`)

### 10. `BANDS` constant extension (lines 46-51)

```typescript
type BandId =
  | "sub" | "low" | "low-mid" | "mid" | "high-mid" | "high" | "sparkle";
type BandTier = "primary" | "secondary";
type BandKind = "shelf-low" | "peak" | "shelf-high";

interface Band {
  id: BandId;
  label: string;
  hz: number;
  color: string;
  kind: BandKind;
  tier: BandTier;          // NEW — drives visual hierarchy
  qOctaves: number;
}

const BANDS: readonly Band[] = [
  { id: "sub",      label: "SUB",     hz: 80,    color: "TBD", kind: "peak",       tier: "secondary", qOctaves: 1.0 },
  { id: "low",      label: "LOW",     hz: 200,   color: "#22d3ee", kind: "shelf-low",  tier: "primary",   qOctaves: 0   },
  { id: "low-mid",  label: "LOW-MID", hz: 400,   color: "#4ade80", kind: "peak",       tier: "secondary", qOctaves: 1.0 },
  { id: "mid",      label: "MID",     hz: 1500,  color: "#a78bfa", kind: "peak",       tier: "primary",   qOctaves: 1.2 },
  { id: "high-mid", label: "HIGH-MID", hz: 3500, color: "TBD", kind: "peak",       tier: "secondary", qOctaves: 1.0 },
  { id: "high",     label: "HIGH",    hz: 6000,  color: "#60a5fa", kind: "shelf-high", tier: "primary",   qOctaves: 0   },
  { id: "sparkle",  label: "SPARKLE", hz: 12000, color: "TBD", kind: "shelf-high", tier: "secondary", qOctaves: 0   },
];
```

The 4 existing band entries keep their colors. The 3 new ones (`sub`, `high-mid`, `sparkle`) need color choices — see Uncertainties §1.

The 3 existing secondary band (`low-mid`) keeps its current color (#4ade80, green-ish), but its **tier classification** now matches the new secondary bands. Visual hierarchy below treats `low-mid` as secondary.

### 11. Visual hierarchy implementation

Per the prompt-doc: primary vs secondary surface is the functional differentiation. Both kinds are legitimate shaping bands; the visual difference reflects "primary control surface (also has a knob)" vs "secondary control surface (Visual EQ only)."

**Concrete proposal:**

| Property | Primary band node | Secondary band node |
|---|---|---|
| Node radius | 8 (slightly up from existing 7) | 5 |
| Hit-target radius | 18 (unchanged) | 18 |
| Fill opacity | 1.0 | 0.85 |
| Outline ring | 1.5px ring at full opacity (anchor halo) | none |
| Label text size | unchanged | unchanged |
| Label opacity | unchanged | 0.7 (slightly subdued) |
| Drag interaction | Vertical (existing) | Vertical (existing) |
| Double-click reset | Yes | Yes |

The differentiation should read at a glance — primary bands look like anchored "headline" nodes; secondary bands look like additional points. Color is the second axis (primary bands get the brighter / cyan/green/purple/blue existing palette; secondary bands get a quieter palette).

**Color palette extension:** the 3 new bands need 3 distinct colors. Suggested starting point, leaning on `KnobTone` values already in the design system (`Knob.tsx:20-28`):

- `sub` (80 Hz): muted blue or slate — sub region reads as "deep / foundation"
- `high-mid` (3500 Hz): muted amber / gold — between mid (purple) and high (blue), warm-mid character
- `sparkle` (12 kHz): pale gold or pale pink — top-end "shimmer" character

These are starting points only. Final colors should be validated against the existing CSS tokens and the design lead's call. Flagged uncertain — see §1.

### 12. Response curve renderer extension (`VisualEqPanel.tsx:91-116, 173-187`)

The `totalResponseDb` function iterates over `BANDS` and sums each band's contribution. Extending `BANDS` to 7 entries means the curve renderer extends automatically — `bandResponseDb` already handles `peak` / `shelf-low` / `shelf-high` kinds. No structural change needed.

`N_SAMPLES = 180` (line 176) stays — 180 points across the log-frequency range is still smooth at 7 bands.

The Gaussian-for-peaks approximation uses `qOctaves`; for `sub` and `high-mid` (Q=0.8 and 0.9 in DSP), the recon doc's existing `low-mid` mapping (DSP Q=0.9 → UI qOctaves=1.0) suggests `sub` at qOctaves≈1.1 and `high-mid` at qOctaves≈1.0. These are approximation tunings, not audible — see §2.

---

## Test surface

### 13. New per-band frequency-response tests (`dsp.rs` `#[cfg(test)] mod tests`)

Mirror the existing `low_mid_band_centred_at_400hz_with_q_point_9` test pattern at `dsp.rs:3394`:

```rust
#[test]
fn sub_band_centred_at_80hz_with_q_point_8() {
    let sr = 48_000.0_f32;
    let coeffs = BiquadCoeffs::peaking(sr, 80.0, 0.8, 6.0);
    let at_80  = biquad_magnitude_db_at(&coeffs, 80.0, sr);
    let at_30  = biquad_magnitude_db_at(&coeffs, 30.0, sr);
    let at_200 = biquad_magnitude_db_at(&coeffs, 200.0, sr);
    assert!((at_80 - 6.0).abs() < 0.3, ...);
    assert!(at_30.abs()  < 1.5, ...);
    assert!(at_200.abs() < 2.5, ...);  // 80 Hz Q=0.8 has wider skirts
}

#[test]
fn high_mid_band_centred_at_3500hz_with_q_point_9() {
    // Same pattern. Test points: 3500 (peak), 1500 (low neighbor), 6000 (high neighbor).
}

#[test]
fn sparkle_band_centred_at_12khz_high_shelf_slope_point_7() {
    // High-shelf pattern. At +6 dB gain: ~+6 dB at 18 kHz, ~+3 dB at 12 kHz,
    // ~0 dB at 6 kHz (well below). Mirror existing high_shelf test conventions.
}
```

Plus three "preset baseline + user offset combine cleanly" tests if those patterns exist for the current bands (grep `effective_low_db` / `effective_mid_db` in the existing test module for the right shape).

### 14. Slow-lane byte-identical fixture gate

The slow lane (`AMS_RUN_REAL_FIXTURE=1 cargo test`) must produce **byte-identical WAV output** to pre-change for every preset, since:

- All 8 presets default `sub_db = high_mid_db = sparkle_db = 0.0`.
- `MasteringSettings` defaults all three `eq_*_db` user offsets to 0.0.
- `BiquadCoeffs::peaking(sr, freq, q, 0.0)` returns identity (verified by the existing `BiquadCoeffs` early-return at `dsp.rs:194-220` for `gain_db.abs() < 1e-4`).
- `BiquadCoeffs::high_shelf(sr, freq, 0.0, slope)` returns identity (same pattern at `dsp.rs:56-83`).

Same gate the wav_writer lift used in commits 1-2 of the engine.rs split sequence. If a byte differs, the implementation is wrong — likely an `f32` arithmetic path that doesn't actually short-circuit at gain=0.

**Verification command** (per CLAUDE.md):
```powershell
$env:AMS_RUN_REAL_FIXTURE = "1"
cargo test
Remove-Item Env:\AMS_RUN_REAL_FIXTURE
```

### 15. Existing tests to extend

Every test file that constructs `MasteringSettings` with the four `eq_*_db` fields needs the three new fields added (all set to 0.0). From the recon:

- `tests/album_arc_trace.rs:31-34`
- `tests/album_character_bias.rs:34-37`
- `tests/album_plan_landing.rs:52-55`
- `tests/album_render.rs:30-33`
- `tests/album_simple_landing.rs:55-58`
- `tests/contracts.rs:1776-1779` (and any other construction sites in that file)
- `tests/delivery_profile_render.rs:51-54`
- `tests/dither_absence_of_harmonics.rs:37-40`
- `tests/export_volume_match.rs:40-43`
- `tests/preset_distinctness.rs:191-194`
- `tests/preset_loudness_balance.rs:92-95`
- `tests/preset_signature.rs:114-117`

Plus the in-file `types.rs:835`+ test fixtures.

If `MasteringSettings` derives `Default` and the new fields use `#[serde(default)]`, only the explicit-construction sites need updating; struct-update-syntax callers (`MasteringSettings { eq_low_db: 0.0, ..Default::default() }`) get the new fields for free.

`tests/preset_distinctness.rs` and `tests/preset_signature.rs` exercise the per-preset character envelope. They use Goertzel filters at specific frequencies (`tests/preset_distinctness.rs:125, 142`, `tests/preset_signature.rs:89`). These tests should **still pass unchanged** since the new bands at 0.0 dB are identity — verify this explicitly.

---

## Commit shape

Three commits proposed, audio.rs-split discipline. Slow lane runs on every commit that touches DSP coefficients or the chain.

### Commit 1 — DSP extension + preset calibration zeros + runtime mapping

- `PresetCalibration` extension (3 new fields).
- All 9 preset constants extended with 0.0 defaults for the new fields.
- `ChainCoeffs` and `ChannelState` extensions (3 new BiquadCoeffs / BiquadState fields each).
- `from_settings` extension (3 new effective_*_db calculations + biquad construction + struct literal update).
- Chain order extension in `process_frame_inplace` and `process_sample`.
- New `dsp.rs` frequency-response tests for sub/high-mid/sparkle.
- Update in-file test fixtures in `dsp.rs` `mod tests`.

**Slow-lane gate:** `AMS_RUN_REAL_FIXTURE=1 cargo test` must produce byte-identical WAV output for every preset.

### Commit 2 — State model extension (Rust + TS + setter + tests)

- `MasteringSettings` extension in `types.rs` (3 new fields, `#[serde(default)]`).
- `MasteringSettings` extension in `bindings.ts`.
- `useTrackMaster.ts` defaults + `setEqBand` band-union widening.
- `Macros` `onEq` prop signature widening in `App.tsx`.
- All external test fixtures in `tests/*.rs` extended with the 3 new fields at 0.0.

**Fast lane only.** State plumbing doesn't change audio output; the byte-identity gate from commit 1 still holds.

### Commit 3 — Visual EQ component (BANDS + visual hierarchy + colors)

- `VisualEqPanel.tsx` BANDS constant extension to 7 bands with `tier` field.
- Visual hierarchy implementation (primary vs secondary node sizes, opacities, halo).
- Color choices for the 3 new bands.
- Response curve renderer continues to work unchanged (just iterates BANDS).

**Fast lane only.** No audio changes.

Optional **commit 4** (if test-extension churn is large): consolidate the test-fixture extensions into a focused commit so the state-model changes (commit 2) stay tight.

---

## Flagged uncertainties

### 1. Color choices for the 3 new bands

The existing 4 band colors are hardcoded hex values, not drawn from `KnobTone`. I'm proposing colors based on "muted to differentiate from primary, harmonious with existing palette":

- `sub`: muted blue/slate (deep / foundation character)
- `high-mid`: muted amber/gold (warm-mid between purple and blue)
- `sparkle`: pale gold / pale pink (top-end shimmer)

But I don't know the design system's tokens beyond the inline values in `BANDS` and the `TONE_COLOR` map in `Knob.tsx:30+`. Final choices should be validated against existing CSS tokens. If Dan or a design pass produces specific hex values, lock them in commit 3.

### 2. Sparkle slope at 12 kHz (0.7 vs 0.5)

Plan defaults to slope 0.7 to match the existing High shelf at 6 kHz and `presence_air` at 10 kHz. A gentler slope (0.5) might serve shaping use better at 12 kHz — wider transition band, less "abrupt edge" feel — but it's speculation without a listening pass. Conservative default holds for this slice; reconsider in a future tuning pass.

### 3. Sparkle naming

Recommending `sparkle` to resolve the three-way "air" collision (`PresetCalibration.air_db` at 6 kHz, `AdvancedSettings.presence_air` at 10 kHz, proposed new band at 12 kHz). Alternatives considered: `top`, `shimmer`, `extreme-high`, `ultra`. None feel as strong. If Dan prefers another name, change is mechanical — rename `sparkle_db` / `eq_sparkle_db` / band `id: "sparkle"` / label `"SPARKLE"`.

### 4. Saved-preset deserialization (serde defaults)

`#[serde(default)]` on the 3 new `MasteringSettings` fields should let old saved projects deserialize cleanly with the new fields defaulting to 0.0. Verification before merge: deserialize an old `.ams` project (or whatever the persistence format is — TBD) and confirm:

- The 3 new fields are populated with 0.0.
- No deserialization error.
- The mastered output is byte-identical to what it was before the upgrade (because all the new EQ user offsets default to 0.0 and the preset's new EQ baselines also default to 0.0).

If projects are persisted as fully-serialized `MasteringSettings` (not a delta), this is mechanical. If there's a more nuanced persistence layer, audit before merging commit 2.

### 5. `PresetCalibration` field ordering

The struct currently has EQ fields clustered together (`low_shelf_db`, `low_mid_db`, `presence_db`, `air_db`). Inserting `sub_db` before `low_shelf_db` and `high_mid_db` between `presence_db` and `air_db` and `sparkle_db` after `air_db` keeps the EQ cluster grouped. But `PresetCalibration` doesn't derive `Deserialize` (it's a `pub struct` with `Debug, Clone, Copy` only at `dsp.rs:279`), so field-order doesn't affect serde — it's just readability. Worth a quick check whether `PresetCalibration` is ever serialized anywhere; if not, the field ordering proposal is purely cosmetic.

### 6. `qOctaves` values for sub and high-mid in `VisualEqPanel`

The existing mapping in `BANDS` doesn't perfectly mirror DSP Q values (e.g., `low-mid` is DSP Q=0.9 → UI qOctaves=1.0). Suggesting `sub` qOctaves≈1.1 and `high-mid` qOctaves≈1.0 by analogy. This affects the visual response-curve approximation only, not actual audio. Confirm or adjust during commit 3.

---

## Out of scope (sanity check list)

These are NOT part of this slice:

- Freq/Q sweep (horizontal drag on Visual EQ). Stays disabled. `VisualEqPanel.tsx:14-18` constraint stays true.
- Surfacing preset baselines on the Visual EQ. Nodes still show user offsets only.
- New knobs in the Tone Shape row. Stays at exactly 3 (Low / Mid / High).
- Promoting `sub_highpass` to the user-facing surface. Stays preset-locked.
- Touching `warmth` (300 Hz Advanced shelf) or `presence_air` (10 kHz Advanced shelf). Out of scope.
- Per-preset tuning of the new bands. All defaults 0.0 dB. Tuning is a separate listening-batch slice.
- `science_note` tooltip on preset orbs. Separate future slice.
- Product positioning copy in README/onboarding. Separate future slice.

---

## Verification path before merge

1. **Commit 1 fast lane:** `cargo test --lib` from `src-tauri/`. New per-band tests + all existing tests pass.
2. **Commit 1 slow lane:** `AMS_RUN_REAL_FIXTURE=1 cargo test`. Byte-identical WAV output for every preset. If any byte differs, stop and debug.
3. **Commit 2 fast lane:** `npm test` + `npm run build` + `cargo test`. State plumbing changes; no audio path. Existing tests should pass unchanged.
4. **Commit 3 fast lane:** `npm test` + `npm run build`. Visual EQ extension; no audio path.
5. **Final slow lane:** `AMS_RUN_REAL_FIXTURE=1 cargo test` after commit 3 to confirm byte identity end-to-end across the three commits.

Per CLAUDE.md commit-shape convention, each commit includes a `Verification:` block in its message with the specific command outputs.

---

*Plan drafted by Vera, 2026-05-19. Awaiting Dan's review before implementation begins.*

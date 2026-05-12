# Warmth + Presence/Air Advanced Controls Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the existing `warmth` and `presence_air` Advanced controls so they drive real DSP — two added biquad shelves on top of the existing 3-band EQ — and drop their "(coming soon)" labels.

**Architecture:** Pure-EQ shelves added to `ChainCoeffs`/`ChannelState`/`process_frame_inplace`/`process_sample`. Slider `0..1` maps to `0..+4 dB` gain on a low-shelf @ 300 Hz (warmth) and a high-shelf @ 10 kHz (presence/air). Both shelves sit alongside the existing low/mid/high biquads in Pass 1 of the chain. Identity-default via `BiquadCoeffs::low_shelf`/`high_shelf`'s built-in early-return for `gain_db ≈ 0`. No saturation interaction. Backed by `docs/superpowers/specs/2026-05-12-warmth-presence-air-design.md` and the research extract in `docs/research/most-recent-mastering-app-research.md`.

**Tech Stack:** Rust (Tauri backend, biquad math in `src-tauri/src/dsp.rs`), TypeScript/React (`src/App.tsx`'s `AdvancedPanel`).

---

## File Structure

- **Modify** `src-tauri/src/dsp.rs`:
  - `ChainCoeffs` struct → 2 new fields (`warmth`, `presence_air`, both `BiquadCoeffs`)
  - `ChainCoeffs::from_settings` → compute both biquads
  - `ChannelState` struct → 2 new fields (`warmth`, `presence_air`, both `BiquadState`)
  - `MasteringChain::process_frame_inplace` → apply both biquads inside Pass 1, after the existing high-shelf, before the width transform
  - `MasteringChain::process_sample` (legacy path) → apply both biquads after the existing high-shelf
  - `mod tests` → 5 new tests (3 warmth + 2 presence_air)
- **Modify** `src/App.tsx`:
  - `AdvancedPanel` → drop `"(coming soon)"` suffix on Warmth and Presence/Air labels (lines ~1384 and ~1393)
- **Modify** `docs/progress.md`: append a progress entry under the loop convention
- **Create**: none (no new files)

Each task is self-contained; the slice commits as a single push at the end.

---

## Task 1: Wire warmth biquad (low-shelf @ 300 Hz, 0..1 → 0..+4 dB)

**Files:**
- Modify: `src-tauri/src/dsp.rs` (`ChainCoeffs`, `ChannelState`, `ChainCoeffs::from_settings`, `MasteringChain::process_frame_inplace`, `MasteringChain::process_sample`, `mod tests`)

---

- [ ] **Step 1.1: Add a tiny biquad-response helper near the top of `mod tests` (above the existing width tests)**

This helper lets the tests assert frequency-response behavior without driving samples through the chain. Pin behavior via the closed-form response of a biquad evaluated on the unit circle.

```rust
/// Magnitude (in dB) of a biquad's frequency response at a given Hz value.
/// Evaluates the transfer function `H(z) = (b0 + b1*z^-1 + b2*z^-2) /
/// (1 + a1*z^-1 + a2*z^-2)` at `z = e^(j*omega)` where `omega = 2*pi*f/sr`.
/// Used to verify shelf gain at well-below-corner and well-above-corner
/// frequencies without running audio through the chain.
fn biquad_magnitude_db_at(c: &BiquadCoeffs, freq_hz: f32, sample_rate: f32) -> f32 {
    let omega = 2.0 * std::f32::consts::PI * freq_hz / sample_rate;
    let cos_o = omega.cos();
    let sin_o = omega.sin();
    // z^-1 = cos(-w) + j*sin(-w) = cos(w) - j*sin(w)
    let z1_re = cos_o;
    let z1_im = -sin_o;
    // z^-2 = (z^-1)^2; expand: (a + jb)^2 = (a^2 - b^2) + j(2ab)
    let z2_re = z1_re * z1_re - z1_im * z1_im;
    let z2_im = 2.0 * z1_re * z1_im;
    let num_re = c.b0 + c.b1 * z1_re + c.b2 * z2_re;
    let num_im = c.b1 * z1_im + c.b2 * z2_im;
    let den_re = 1.0 + c.a1 * z1_re + c.a2 * z2_re;
    let den_im = c.a1 * z1_im + c.a2 * z2_im;
    let num_mag = (num_re * num_re + num_im * num_im).sqrt();
    let den_mag = (den_re * den_re + den_im * den_im).sqrt();
    20.0 * (num_mag / den_mag).log10()
}
```

Place it inside `#[cfg(test)] mod tests { use super::*; ... }`, right after the existing `fn approx_eq` helper.

---

- [ ] **Step 1.2: Write the three failing warmth tests at the bottom of `mod tests`**

```rust
/// Phase 12.2 — warmth control. When `Advanced.warmth = None`, the chain's
/// warmth biquad must be identity (b0 = 1.0, all other coeffs ~0) so the
/// untouched-slider path is byte-equivalent to the pre-slice chain output.
#[test]
fn warmth_default_is_identity() {
    let settings = MasteringSettings {
        preset: Preset::Custom { id: "t".to_string() },
        intensity: 0.0,
        eq_low_db: 0.0,
        eq_mid_db: 0.0,
        eq_high_db: 0.0,
        volume_match: false,
        input_gain_db: 0.0,
        output_gain_db: 0.0,
        advanced: AdvancedSettings::default(),
    };
    let c = ChainCoeffs::from_settings(44_100, &settings);
    assert!(approx_eq(c.warmth.b0, 1.0, 1e-6), "warmth.b0 should be 1.0, got {}", c.warmth.b0);
    assert!(approx_eq(c.warmth.b1, 0.0, 1e-6), "warmth.b1 should be 0.0, got {}", c.warmth.b1);
    assert!(approx_eq(c.warmth.b2, 0.0, 1e-6), "warmth.b2 should be 0.0, got {}", c.warmth.b2);
    assert!(approx_eq(c.warmth.a1, 0.0, 1e-6), "warmth.a1 should be 0.0, got {}", c.warmth.a1);
    assert!(approx_eq(c.warmth.a2, 0.0, 1e-6), "warmth.a2 should be 0.0, got {}", c.warmth.a2);
}

/// Phase 12.2 — warmth control. Slider at 1.0 must lift the 300 Hz low
/// frequencies by close to the design's max of +4 dB and leave the high
/// frequencies near 0 dB. Pins both the magnitude AND the shelf shape.
#[test]
fn warmth_at_one_lifts_300hz_band() {
    let settings = MasteringSettings {
        preset: Preset::Custom { id: "t".to_string() },
        intensity: 0.0,
        eq_low_db: 0.0,
        eq_mid_db: 0.0,
        eq_high_db: 0.0,
        volume_match: false,
        input_gain_db: 0.0,
        output_gain_db: 0.0,
        advanced: AdvancedSettings {
            warmth: Some(1.0),
            ..AdvancedSettings::default()
        },
    };
    let c = ChainCoeffs::from_settings(44_100, &settings);

    // 100 Hz is well below the shelf corner @ 300 Hz — the boost should be
    // near the full +4 dB (allow some tolerance for shelf slope).
    let gain_low = biquad_magnitude_db_at(&c.warmth, 100.0, 44_100.0);
    assert!(
        gain_low > 3.0,
        "warmth=1.0 should give >+3 dB at 100 Hz (below shelf corner), got {} dB",
        gain_low
    );

    // 5 kHz is well above the shelf corner — gain should be near 0 dB.
    let gain_high = biquad_magnitude_db_at(&c.warmth, 5_000.0, 44_100.0);
    assert!(
        gain_high.abs() < 0.5,
        "warmth=1.0 should leave 5 kHz near 0 dB, got {} dB",
        gain_high
    );
}

/// Phase 12.2 — warmth control clamping. Out-of-range slider values (5.0,
/// -1.0) must clamp into [0, 1] before mapping to dB, so a runaway value
/// can't push the shelf past +4 dB or invert gain.
#[test]
fn chain_coeffs_clamps_warmth_into_range() {
    let make = |w: f32| MasteringSettings {
        preset: Preset::Custom { id: "t".to_string() },
        intensity: 0.0,
        eq_low_db: 0.0,
        eq_mid_db: 0.0,
        eq_high_db: 0.0,
        volume_match: false,
        input_gain_db: 0.0,
        output_gain_db: 0.0,
        advanced: AdvancedSettings {
            warmth: Some(w),
            ..AdvancedSettings::default()
        },
    };
    let c_high = ChainCoeffs::from_settings(44_100, &make(5.0));
    let c_max = ChainCoeffs::from_settings(44_100, &make(1.0));
    // Clamped-high and at-max should produce the SAME biquad coefficients.
    assert!(approx_eq(c_high.warmth.b0, c_max.warmth.b0, 1e-6),
        "warmth=5.0 should clamp to 1.0 (b0 mismatch: {} vs {})",
        c_high.warmth.b0, c_max.warmth.b0);

    let c_neg = ChainCoeffs::from_settings(44_100, &make(-1.0));
    let c_zero = ChainCoeffs::from_settings(44_100, &make(0.0));
    assert!(approx_eq(c_neg.warmth.b0, c_zero.warmth.b0, 1e-6),
        "warmth=-1.0 should clamp to 0.0 (b0 mismatch: {} vs {})",
        c_neg.warmth.b0, c_zero.warmth.b0);
}
```

---

- [ ] **Step 1.3: Run `cargo check --tests` to verify the tests don't compile (no `warmth` field yet)**

```bash
cd "C:\Users\SM - Dan\Documents\GitHub\album-mastering-studio-claude-build\src-tauri"
cargo check --tests
```

Expected: compile error of the form `no field 'warmth' on type ChainCoeffs` (and possibly the same on `ChannelState` once we add the apply step). This proves the test is exercising real new behavior.

---

- [ ] **Step 1.4: Add `warmth: BiquadCoeffs` to `ChainCoeffs`**

Locate the `ChainCoeffs` struct definition (around line 130 of `dsp.rs`):

```rust
#[derive(Debug, Clone, Copy)]
pub struct ChainCoeffs {
    pub low: BiquadCoeffs,
    pub mid: BiquadCoeffs,
    pub high: BiquadCoeffs,
    pub input_gain_lin: f32,
    pub saturation_amount: f32,
    pub ceiling_lin: f32,
    pub user_output_gain_lin: f32,
    pub volume_match_gain_lin: f32,
    pub width_side_scale: f32,
}
```

Replace with:

```rust
#[derive(Debug, Clone, Copy)]
pub struct ChainCoeffs {
    pub low: BiquadCoeffs,
    pub mid: BiquadCoeffs,
    pub high: BiquadCoeffs,
    /// Phase 12.2 — surgical low-mid warmth shelf, additive on top of the
    /// preset and the main Low band. Low-shelf @ 300 Hz, slope 0.7. Slider
    /// 0..1 in `AdvancedSettings::warmth` maps to 0..+4 dB; clamped on read.
    pub warmth: BiquadCoeffs,
    /// Phase 12.2 — surgical air shelf, additive on top of the preset and
    /// the main High band. High-shelf @ 10 kHz, slope 0.7. Slider 0..1 in
    /// `AdvancedSettings::presence_air` maps to 0..+4 dB; clamped on read.
    pub presence_air: BiquadCoeffs,
    pub input_gain_lin: f32,
    pub saturation_amount: f32,
    pub ceiling_lin: f32,
    pub user_output_gain_lin: f32,
    pub volume_match_gain_lin: f32,
    pub width_side_scale: f32,
}
```

---

- [ ] **Step 1.5: Add `warmth: BiquadState` to `ChannelState`**

Locate `ChannelState` (around line 258 of `dsp.rs`):

```rust
#[derive(Debug, Clone, Default)]
pub struct ChannelState {
    low: BiquadState,
    mid: BiquadState,
    high: BiquadState,
}
```

Replace with:

```rust
#[derive(Debug, Clone, Default)]
pub struct ChannelState {
    low: BiquadState,
    mid: BiquadState,
    high: BiquadState,
    warmth: BiquadState,
    presence_air: BiquadState,
}
```

---

- [ ] **Step 1.6: Wire `warmth` (and a `presence_air` stub) into `ChainCoeffs::from_settings`**

Locate the section of `from_settings` that builds the three existing biquads (around lines 205-208):

```rust
let low = BiquadCoeffs::low_shelf(sr, 200.0, effective_low_db, 0.7);
let mid = BiquadCoeffs::peaking(sr, 1500.0, 0.8, effective_mid_db);
let high = BiquadCoeffs::high_shelf(sr, 6000.0, effective_high_db, 0.7);
```

Add the warmth and presence_air biquads immediately after:

```rust
let low = BiquadCoeffs::low_shelf(sr, 200.0, effective_low_db, 0.7);
let mid = BiquadCoeffs::peaking(sr, 1500.0, 0.8, effective_mid_db);
let high = BiquadCoeffs::high_shelf(sr, 6000.0, effective_high_db, 0.7);

// Phase 12.2 — Advanced warmth (low-shelf @ 300 Hz). Slider value clamped
// into [0, 1] then scaled to a 0..+4 dB lift. When the slider is None or
// zero, `BiquadCoeffs::low_shelf` returns identity via its built-in
// early-return at `gain_db < 1e-4`.
let warmth_db = settings
    .advanced
    .warmth
    .unwrap_or(0.0)
    .clamp(0.0, 1.0)
    * 4.0;
let warmth = BiquadCoeffs::low_shelf(sr, 300.0, warmth_db, 0.7);

// Phase 12.2 — Advanced presence/air (high-shelf @ 10 kHz). Same clamp +
// scale pattern as warmth. Sits above the main High band (6 kHz) so the
// two controls shape distinct perceptual regions.
let presence_air_db = settings
    .advanced
    .presence_air
    .unwrap_or(0.0)
    .clamp(0.0, 1.0)
    * 4.0;
let presence_air = BiquadCoeffs::high_shelf(sr, 10_000.0, presence_air_db, 0.7);
```

Then locate the `Self { ... }` literal at the bottom of `from_settings` (around line 245-256):

```rust
Self {
    low,
    mid,
    high,
    input_gain_lin,
    saturation_amount,
    ceiling_lin,
    user_output_gain_lin,
    volume_match_gain_lin,
    width_side_scale,
}
```

Add `warmth` and `presence_air` to the literal:

```rust
Self {
    low,
    mid,
    high,
    warmth,
    presence_air,
    input_gain_lin,
    saturation_amount,
    ceiling_lin,
    user_output_gain_lin,
    volume_match_gain_lin,
    width_side_scale,
}
```

---

- [ ] **Step 1.7: Apply warmth + presence_air in `MasteringChain::process_frame_inplace` (Pass 1)**

Locate the Pass 1 loop inside `process_frame_inplace` (around lines 487-498):

```rust
for ch in 0..channels {
    let state = &mut self.states[ch];
    let mut y = frame[ch] * self.coeffs.input_gain_lin;
    y = state.low.process(&self.coeffs.low, y);
    y = state.mid.process(&self.coeffs.mid, y);
    y = state.high.process(&self.coeffs.high, y);
    frame[ch] = y;
}
```

Replace with:

```rust
for ch in 0..channels {
    let state = &mut self.states[ch];
    let mut y = frame[ch] * self.coeffs.input_gain_lin;
    y = state.low.process(&self.coeffs.low, y);
    y = state.mid.process(&self.coeffs.mid, y);
    y = state.high.process(&self.coeffs.high, y);
    y = state.warmth.process(&self.coeffs.warmth, y);
    y = state.presence_air.process(&self.coeffs.presence_air, y);
    frame[ch] = y;
}
```

The two new biquads run on the same `y` after the existing three, still per-channel, still inside Pass 1 (before width transform).

---

- [ ] **Step 1.8: Apply warmth + presence_air in `MasteringChain::process_sample` (legacy path)**

Locate `process_sample` (around lines 588-610):

```rust
let state = &mut self.states[idx];
let mut y = sample * self.coeffs.input_gain_lin;
y = state.low.process(&self.coeffs.low, y);
y = state.mid.process(&self.coeffs.mid, y);
y = state.high.process(&self.coeffs.high, y);
if self.coeffs.saturation_amount > 0.0 {
```

Replace the EQ section with:

```rust
let state = &mut self.states[idx];
let mut y = sample * self.coeffs.input_gain_lin;
y = state.low.process(&self.coeffs.low, y);
y = state.mid.process(&self.coeffs.mid, y);
y = state.high.process(&self.coeffs.high, y);
y = state.warmth.process(&self.coeffs.warmth, y);
y = state.presence_air.process(&self.coeffs.presence_air, y);
if self.coeffs.saturation_amount > 0.0 {
```

The legacy path now mirrors Pass 1 in `process_frame_inplace` for the EQ stage.

---

- [ ] **Step 1.9: Run `cargo test --lib` and verify all warmth tests pass**

```bash
cd "C:\Users\SM - Dan\Documents\GitHub\album-mastering-studio-claude-build\src-tauri"
cargo test --lib
```

Expected: 22 tests passing (was 19; +3 warmth tests). Look for these names in the green list:
- `warmth_default_is_identity`
- `warmth_at_one_lifts_300hz_band`
- `chain_coeffs_clamps_warmth_into_range`

If any fail, fix and re-run. If `warmth_default_is_identity` fails, the biquad isn't identity at gain_db=0 — check the `from_settings` warmth construction and the `BiquadCoeffs::low_shelf` early-return threshold.

---

## Task 2: Add presence_air tests

**Files:**
- Modify: `src-tauri/src/dsp.rs` (`mod tests`)

The implementation for `presence_air` already shipped in Task 1 (since both fields had to land together for the struct to compile). This task just adds the two presence_air-specific tests.

---

- [ ] **Step 2.1: Write the two presence_air tests at the bottom of `mod tests`**

```rust
/// Phase 12.2 — presence_air control. Default `None` must produce an
/// identity biquad, matching the warmth default contract.
#[test]
fn presence_air_default_is_identity() {
    let settings = MasteringSettings {
        preset: Preset::Custom { id: "t".to_string() },
        intensity: 0.0,
        eq_low_db: 0.0,
        eq_mid_db: 0.0,
        eq_high_db: 0.0,
        volume_match: false,
        input_gain_db: 0.0,
        output_gain_db: 0.0,
        advanced: AdvancedSettings::default(),
    };
    let c = ChainCoeffs::from_settings(44_100, &settings);
    assert!(approx_eq(c.presence_air.b0, 1.0, 1e-6),
        "presence_air.b0 should be 1.0, got {}", c.presence_air.b0);
    assert!(approx_eq(c.presence_air.b1, 0.0, 1e-6));
    assert!(approx_eq(c.presence_air.b2, 0.0, 1e-6));
    assert!(approx_eq(c.presence_air.a1, 0.0, 1e-6));
    assert!(approx_eq(c.presence_air.a2, 0.0, 1e-6));
}

/// Phase 12.2 — presence_air control. Slider at 1.0 must lift the 10 kHz
/// high frequencies by close to +4 dB and leave the low frequencies near
/// 0 dB. Mirror-image of the warmth test.
#[test]
fn presence_air_at_one_lifts_10khz_band() {
    let settings = MasteringSettings {
        preset: Preset::Custom { id: "t".to_string() },
        intensity: 0.0,
        eq_low_db: 0.0,
        eq_mid_db: 0.0,
        eq_high_db: 0.0,
        volume_match: false,
        input_gain_db: 0.0,
        output_gain_db: 0.0,
        advanced: AdvancedSettings {
            presence_air: Some(1.0),
            ..AdvancedSettings::default()
        },
    };
    let c = ChainCoeffs::from_settings(44_100, &settings);

    // 18 kHz is well above the shelf corner @ 10 kHz — should be near full
    // +4 dB lift. Cap the test frequency at 18 kHz so we stay clear of
    // Nyquist (22.05 kHz) where biquad magnitude estimates get noisy.
    let gain_high = biquad_magnitude_db_at(&c.presence_air, 18_000.0, 44_100.0);
    assert!(
        gain_high > 3.0,
        "presence_air=1.0 should give >+3 dB at 18 kHz (above shelf corner), got {} dB",
        gain_high
    );

    // 1 kHz is well below the shelf corner — gain should be near 0 dB.
    let gain_low = biquad_magnitude_db_at(&c.presence_air, 1_000.0, 44_100.0);
    assert!(
        gain_low.abs() < 0.5,
        "presence_air=1.0 should leave 1 kHz near 0 dB, got {} dB",
        gain_low
    );
}
```

---

- [ ] **Step 2.2: Run `cargo test --lib` and verify all 5 new tests pass**

```bash
cd "C:\Users\SM - Dan\Documents\GitHub\album-mastering-studio-claude-build\src-tauri"
cargo test --lib
```

Expected: 24 tests passing (19 baseline + 3 warmth + 2 presence_air). All 5 new tests by name:
- `warmth_default_is_identity` ✓
- `warmth_at_one_lifts_300hz_band` ✓
- `chain_coeffs_clamps_warmth_into_range` ✓
- `presence_air_default_is_identity` ✓
- `presence_air_at_one_lifts_10khz_band` ✓

---

## Task 3: Drop "(coming soon)" labels in AdvancedPanel

**Files:**
- Modify: `src/App.tsx` (around lines 1384, 1393)

---

- [ ] **Step 3.1: Rename Warmth label**

Locate the `NumberField` for Warmth in `AdvancedPanel` (around line 1384):

```tsx
        <NumberField
          label="Warmth (coming soon)"
          value={a.warmth}
          step={0.05}
          min={0}
          max={1}
          format={(v) => v.toFixed(2)}
          onChange={(v) => update("warmth", v)}
        />
```

Replace with:

```tsx
        <NumberField
          label="Warmth"
          value={a.warmth}
          step={0.05}
          min={0}
          max={1}
          format={(v) => v.toFixed(2)}
          onChange={(v) => update("warmth", v)}
        />
```

---

- [ ] **Step 3.2: Rename Presence/Air label**

Locate the `NumberField` for Presence/Air (around line 1393):

```tsx
        <NumberField
          label="Presence/Air (coming soon)"
          value={a.presence_air}
          step={0.05}
          min={0}
          max={1}
          format={(v) => v.toFixed(2)}
          onChange={(v) => update("presence_air", v)}
        />
```

Replace with:

```tsx
        <NumberField
          label="Presence/Air"
          value={a.presence_air}
          step={0.05}
          min={0}
          max={1}
          format={(v) => v.toFixed(2)}
          onChange={(v) => update("presence_air", v)}
        />
```

---

## Task 4: Final verification + progress.md + commit + push

**Files:**
- Modify: `docs/progress.md` (append a new entry)

---

- [ ] **Step 4.1: Run the full test suite**

```bash
cd "C:\Users\SM - Dan\Documents\GitHub\album-mastering-studio-claude-build\src-tauri"
cargo test
```

Expected: 61 tests pass (was 56; +5 new). Specifically:
- `running 24 tests` in the `lib.rs` section (was 19)
- `running 35 tests` in `contracts.rs` (unchanged — no contract tests added or removed)
- Doc-tests: 0 (unchanged)

The 34 contract tests that use `default_settings()` (which has `advanced: AdvancedSettings::default()` → all None) should be byte-equivalent because the warmth/presence_air biquads default to identity. Watch in particular for:
- `presets_produce_distinct_chain_coefficients` ✓
- `mastering_render_writes_processed_wav` ✓
- `mastering_render_processes_real_fixture_if_present` ✓ (real-fixture; runs ~120s)
- `phase_12_1_real_fixture_metering_snapshot` ✓ (real-fixture; runs ~120s)

If any pre-existing test fails, the chain-order or biquad ordering is wrong — revisit Step 1.7 and 1.8.

---

- [ ] **Step 4.2: Run the frontend build**

```bash
cd "C:\Users\SM - Dan\Documents\GitHub\album-mastering-studio-claude-build"
npm run build
```

Expected: clean build, dist/ written, bundle size approximately the same as before (`253.65 KB / 77.57 KB gzipped` ± a few bytes). The two label changes save 26 characters of bundle text.

---

- [ ] **Step 4.3: Append the progress entry**

Open `docs/progress.md` and append at the end:

```markdown

## 2026-05-12 — Phase 12.2 (cont): wire warmth and presence_air (Advanced)

Goal:

Close the fifth P0 slice of the session. Both `warmth` and `presence_air` Advanced controls existed in the UI and the type schema but did nothing. Design grounded in `docs/research/most-recent-mastering-app-research.md` (Sonible smart:limit, LANDR, BandLab, Ozone — pure-EQ shelves, one-sided, additive on top of the 3-band EQ). Spec at `docs/superpowers/specs/2026-05-12-warmth-presence-air-design.md`, plan at `docs/superpowers/plans/2026-05-12-warmth-presence-air.md`.

What changed:

Backend (Rust, `src-tauri/src/dsp.rs`):

- **`ChainCoeffs`**: new `warmth: BiquadCoeffs` and `presence_air: BiquadCoeffs` fields.
- **`ChannelState`**: parallel `warmth: BiquadState` and `presence_air: BiquadState` fields for filter memory.
- **`ChainCoeffs::from_settings`**: maps `Advanced.warmth` slider value `[0..1]` → low-shelf @ 300 Hz, slope 0.7, `[0..+4 dB]`. Same shape for `Advanced.presence_air` → high-shelf @ 10 kHz. Clamped on read; defaults to None → 0 dB → identity biquad (via `BiquadCoeffs::low_shelf`/`high_shelf`'s built-in early-return).
- **`MasteringChain::process_frame_inplace`**: warmth + presence_air biquads applied per-channel inside Pass 1, after the existing low/mid/high biquads, before the width transform.
- **`MasteringChain::process_sample`** (legacy path): same two biquads applied in the same order.
- **Tests**: 5 new in `mod tests` (helper `biquad_magnitude_db_at` for closed-form response checks):
  - `warmth_default_is_identity` — `Advanced.warmth = None` produces identity biquad.
  - `warmth_at_one_lifts_300hz_band` — slider 1.0 gives >+3 dB at 100 Hz and ~0 dB at 5 kHz (pins both magnitude and shelf shape).
  - `chain_coeffs_clamps_warmth_into_range` — values 5.0 and -1.0 clamp to 1.0 and 0.0 respectively.
  - `presence_air_default_is_identity` — mirror-image of warmth default.
  - `presence_air_at_one_lifts_10khz_band` — slider 1.0 gives >+3 dB at 18 kHz and ~0 dB at 1 kHz.

Frontend (`src/App.tsx`):

- `AdvancedPanel`: "(coming soon)" dropped from Warmth and Presence/Air labels. Slider config unchanged.

Verification:

- `cargo test --lib`: 24/24 pass (was 19).
- `cargo test` (full): **61/61 pass** (was 56). Real-fixture tests unchanged — both new biquads default to identity on `default_settings()` and on every existing preset.
- `npm run build`: clean (~253.6 KB / ~77.6 KB gzipped — flat).

Real-audio fixture used: none. Tests use closed-form biquad-response math + the existing real-fixture render tests as a backward-compatibility guarantee.

What failed or remains partial:

- **No frontend test** for the slider's new behavior (vitest infra still deferred).
- **Warmth/Presence_air interaction with the Warmth preset**: stacks additively (no special handling). If users find the Warmth preset + Warmth slider feels redundant or harsh, future polish could either rename the preset or add a per-preset baseline.
- **Adaptive air (Ozone Clarity-style STFT-domain shaping)**: out of scope per spec; static shelf shipped here.

Next recommended slice:

The HANDOFF P0 wired-controls list is now down to one: `compression_density` (real envelope-following compressor before the limiter, ~300-500 lines per HANDOFF). Worth a brainstorm/plan before coding. If listening notes from Dan come in first, those override the queue.
```

---

- [ ] **Step 4.4: Commit and push**

```bash
cd "C:\Users\SM - Dan\Documents\GitHub\album-mastering-studio-claude-build"
git status --short
```

Expected: `M docs/progress.md`, `M src-tauri/src/dsp.rs`, `M src/App.tsx`.

If those are the only modifications, commit:

```bash
git add docs/progress.md src-tauri/src/dsp.rs src/App.tsx
git commit -m "$(cat <<'EOF'
Phase 12.2: wire Warmth and Presence/Air (Advanced)

Two more "(coming soon)" labels become real controls.

Backend (dsp.rs):
- ChainCoeffs: new warmth + presence_air BiquadCoeffs fields.
- ChannelState: parallel BiquadState fields for filter memory.
- from_settings: slider value 0..1 -> 0..+4 dB on a low-shelf @ 300 Hz
  (warmth) and high-shelf @ 10 kHz (presence_air), both slope 0.7.
  Clamped on read; None -> 0 dB -> identity biquad via BiquadCoeffs's
  built-in early return.
- process_frame_inplace + process_sample: both new biquads applied
  per-channel after the existing low/mid/high biquads, before width.
- 5 new tests (closed-form biquad-response checks via a new
  biquad_magnitude_db_at helper): default identity, at-one shelf
  lift + corner-distance shape, and warmth-clamp behavior for each.

Frontend (App.tsx):
- AdvancedPanel: drop "(coming soon)" from Warmth and Presence/Air.

Design backed by docs/research/most-recent-mastering-app-research.md
(Sonible smart:limit, LANDR, BandLab, Ozone consensus = pure-EQ
tilt-style controls, one-sided, additive on top of the 3-band EQ —
no saturation interaction).

Verification:
- cargo test: 61/61 pass (was 56). Real-fixture tests unchanged.
- npm run build: clean, ~253.6 KB / ~77.6 KB gzipped (flat).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
git push origin master
```

Expected push output: a single new commit pushed to `master`. Confirm the commit SHA is shown.

---

## Self-Review Checklist (for the plan author)

After writing, the plan author checks:

1. **Spec coverage** — every section of `2026-05-12-warmth-presence-air-design.md` mapped to a task?
   - Control shape (one-sided, 0..1 → 0..+4 dB): Task 1 (Step 1.6) and Task 2.
   - Numeric values (300 Hz, 10 kHz, slope 0.7): Task 1 Step 1.6.
   - Chain placement (Pass 1, after existing EQ, before width): Task 1 Steps 1.7 / 1.8.
   - Skip-guard via built-in identity: implicit in Step 1.6's straight `unwrap_or(0.0).clamp(0,1)*4.0` mapping (no explicit Option-match), and explicitly tested in `warmth_default_is_identity` / `presence_air_default_is_identity`.
   - ChainCoeffs / ChannelState data model: Steps 1.4 / 1.5.
   - Frontend labels: Task 3.
   - 5 unit tests: Tasks 1 + 2 add exactly those 5.
2. **No placeholders** — search the plan for "TBD", "TODO", "implement later", "Add appropriate error handling", "fill in details". None present.
3. **Type consistency** — `warmth` and `presence_air` are the field names everywhere: `ChainCoeffs`, `ChannelState`, `AdvancedSettings`, tests, biquad construction calls. No drift.

---

*Plan ready for execution.*

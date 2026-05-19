# YES Master — Dynamic EQ Architecture Recon

**Date:** 2026-05-19
**Mode:** Recon only — no design proposals, no changes
**Scope:** Map the current dynamic EQ architecture end-to-end so an extension decision can be made on real ground truth.

---

## 1. DSP layer — EQ stage in the audio processing chain

**Chain order** (`src-tauri/src/dsp.rs:1809-1828`, `process_frame_inplace`):

```
input_gain → sub_highpass (×2 stages) → low → low_mid → mid → high
            → warmth → presence_air → [multiband compressor]
            → [transient shaper] → [width M/S] → [saturation tanh]
            → limiter → volume_match → export_landing_gain
```

The same order is mirrored in `MasteringChain::process_sample` (`dsp.rs:2049-2065`) for the offline / non-interleaved path.

**Band count:**

- 1 subsonic high-pass (`sub_highpass`) — preset-gated, not exposed in EQ UI
- 4 user-shapable tone bands (`low`, `low_mid`, `mid`, `high`)
- 2 Advanced-only shelves (`warmth`, `presence_air`)

Total: **7 filter stages**, of which **4 are user-EQ-shapable** and **the Visual EQ renders 4 nodes**.

**Per-band defaults** (built in `ChainCoeffs::from_settings`, `dsp.rs:712-750`):

| # | Field | Filter type | Frequency | Q / slope | Source of gain |
|---|---|---|---|---|---|
| 0 | `sub_highpass` | Butterworth HPF, cascaded 2-biquad (24 dB/oct) | `preset.highpass_hz` clamped to [20, 40] Hz | `BUTTERWORTH_Q` | Preset-driven; identity if `highpass_hz == 0.0` |
| 1 | `low` | Low-shelf (RBJ) | 200 Hz | slope 0.7 | `preset.low_shelf_db × preset_scale + settings.eq_low_db` |
| 2 | `low_mid` | Peaking (RBJ) | 400 Hz | Q 0.9 | `preset.low_mid_db × preset_scale + settings.eq_low_mid_db` |
| 3 | `mid` | Peaking (RBJ) | 1500 Hz | Q 0.8 | `preset.presence_db × preset_scale + settings.eq_mid_db` |
| 4 | `high` | High-shelf (RBJ) | 6000 Hz | slope 0.7 | `preset.air_db × preset_scale + settings.eq_high_db` |
| 5 | `warmth` | Low-shelf | 300 Hz | slope 0.7 | `settings.advanced.warmth ∈ [0..1] × 4 dB` |
| 6 | `presence_air` | High-shelf | 10 000 Hz | slope 0.7 | `settings.advanced.presence_air ∈ [0..1] × 4 dB` |

`preset_scale = 0.4 + 1.2 × intensity` (`dsp.rs:689`), so at Intensity 50% the preset is at "full character" (`preset_scale = 1.0`); at 0% it softens to 40%; at 100% it overdrives to 160%.

**`PresetCalibration` struct** (`dsp.rs:280-335`) — 17 fields total, EQ-relevant fields bolded:

```rust
pub struct PresetCalibration {
    pub low_shelf_db: f32,       // EQ: 200 Hz low-shelf baseline
    pub low_mid_db: f32,         // EQ: 400 Hz peaking baseline (Phase A2)
    pub presence_db: f32,        // EQ: 1.5 kHz peaking baseline (Codex `presence_db`)
    pub air_db: f32,             // EQ: 6 kHz high-shelf baseline (Codex `air_db`)
    pub warmth: f32,             // Saturation drive 0..1 (post-EQ tanh)
    pub stereo_width: f32,       // M/S widener default; 1.0 = neutral
    pub transient_punch: f32,    // Transient shaper intent
    pub highpass_hz: f32,        // Subsonic HPF cutoff, 20-40 Hz (or 0 = identity)
    pub target_lufs: f32,        // Preset intent only; not applied
    pub ceiling_dbfs: f32,       // Captured; not applied directly
    pub compressor_threshold_dbfs: f32,
    pub compressor_ratio: f32,
    pub compressor_attack_ms: f32,
    pub compressor_release_ms: f32,
    pub science_note: &'static str,
    pub baseline_gain_push_db: f32,
}
```

**Runtime mapping** (`dsp.rs:701-720`):

```rust
let effective_low_db     = preset.low_shelf_db * preset_scale + settings.eq_low_db;
let effective_low_mid_db = preset.low_mid_db   * preset_scale + settings.eq_low_mid_db;
let effective_mid_db     = preset.presence_db  * preset_scale + settings.eq_mid_db;
let effective_high_db    = preset.air_db       * preset_scale + settings.eq_high_db;
// ...
let low      = BiquadCoeffs::low_shelf (sr, 200.0,  effective_low_db,     0.7);
let low_mid  = BiquadCoeffs::peaking   (sr, 400.0,  0.9, effective_low_mid_db);
let mid      = BiquadCoeffs::peaking   (sr, 1500.0, 0.8, effective_mid_db);
let high     = BiquadCoeffs::high_shelf(sr, 6000.0, effective_high_db,    0.7);
```

Preset baseline (scaled by intensity) and user offset are **summed before biquad coefficients are computed**. There is no separate per-band gain stage; the user offset and preset baseline collapse into a single dB number per band.

---

## 2. Preset calibration tables — EQ values per preset

`dsp.rs:340-561`. Values are pre-scale; multiply by `preset_scale` for runtime application.

| Preset | `low_shelf_db` (200 Hz) | `low_mid_db` (400 Hz) | `presence_db` (1.5 kHz) | `air_db` (6 kHz) | `highpass_hz` |
|---|---|---|---|---|---|
| Universal | +0.2 | −0.1 | 0.0 | +1.1 | 24 |
| Clarity | +0.2 | −1.0 | −0.8 | +1.7 | 28 |
| Tape | −0.2 | +0.3 | −1.4 | +2.0 | 24 |
| Spatial | +0.1 | −0.8 | −0.3 | +1.3 | 24 |
| Oomph | +2.4 | −3.0 | −2.6 | −0.8 | 22 |
| Warmth | +0.8 | +0.7 | −1.8 | −0.8 | 24 |
| Punch | +0.8 | −1.8 | +1.6 | +0.8 | 28 |
| Loud | +0.4 | −1.6 | +1.8 | +1.2 | 30 |
| Custom (Neutral) | 0.0 | 0.0 | 0.0 | 0.0 | 0 (identity) |

---

## 3. TypeScript state model

**Band state shape** — fields live on `MasteringSettings`, not a separate band array (`src/bindings.ts:147-173`):

```typescript
export interface MasteringSettings {
  preset: Preset;
  intensity: number;
  eq_low_db: number;
  /// Phase A2 — user offset on top of the preset's low-mid baseline
  /// (400 Hz peaking @ Q=0.9). 0 = use preset value as-is.
  eq_low_mid_db: number;
  eq_mid_db: number;
  eq_high_db: number;
  // ...
  advanced: AdvancedSettings;
}
```

`AdvancedSettings` (also in `bindings.ts`, definition mirrored in `types.rs:551`) carries the `warmth` and `presence_air` fields as `Option<number>` — those are independent of the four EQ bands.

**Initialization** (`src/hooks/useTrackMaster.ts:41-44`, default settings constructor):

```typescript
eq_low_db: 0,
eq_low_mid_db: 0,
eq_mid_db: 0,
eq_high_db: 0,
```

Defaults are always 0 dB user offset. The audible EQ shape on a freshly-loaded preset comes entirely from `PresetCalibration × preset_scale`.

**Preset → band-value feed:** there is none on the frontend. The preset is sent to Rust as the `preset` field on `MasteringSettings`; the Rust chain reads `preset_calibration(&settings.preset)` and applies the baseline server-side. The frontend never holds the per-preset baseline numbers. The `VisualEqPanel` and Tone Shape knobs only show / set the *user offset* (`eq_*_db`), not the effective curve. The displayed nodes always sit at user-offset gain, with no preset baseline overlaid.

---

## 4. Tone Shape knob bindings

**Knob renders** (`src/App.tsx:1419-1458`):

| Knob label | Tone | Range | `value=` | `onChange=` | Band ID passed to setter |
|---|---|---|---|---|---|
| "Low" | cyan | ±12 dB step 0.1 | `settings.eq_low_db` | `(v) => onEq("low", v)` | `"low"` |
| "Mid" | green | ±12 dB step 0.1 | `settings.eq_mid_db` | `(v) => onEq("mid", v)` | `"mid"` |
| "High" | purple | ±12 dB step 0.1 | `settings.eq_high_db` | `(v) => onEq("high", v)` | `"high"` |

No `"low-mid"` knob exists in the Tone Shape cluster. The Tone Shape UI has only three knobs.

**Frequencies confirmed against code:**

- Low knob → `eq_low_db` → `BiquadCoeffs::low_shelf(sr, 200.0, ...)` at `dsp.rs:717` → **200 Hz ✓**
- Mid knob → `eq_mid_db` → `BiquadCoeffs::peaking(sr, 1500.0, 0.8, ...)` at `dsp.rs:719` → **1.5 kHz ✓**
- High knob → `eq_high_db` → `BiquadCoeffs::high_shelf(sr, 6000.0, ...)` at `dsp.rs:720` → **6 kHz ✓**

The UI does not display these frequencies next to the knobs; the labels are just "Low" / "Mid" / "High."

**Binding path:**

```
Knob onChange(v)
  → onEq("low" | "mid" | "high", v)               [App.tsx:1432, 1444, 1456]
  → setEqBand(band, db)                            [useTrackMaster.ts:856-869]
  → updateSettings(trackId, prev =>                [in-crate state setter]
       { ...prev, eq_low_db|eq_mid_db|eq_high_db: db })
  → Rust chain re-runs ChainCoeffs::from_settings on the next update
  → effective_low_db / effective_mid_db / effective_high_db recomputed
  → BiquadCoeffs regenerated and active in process_frame_inplace
```

---

## 5. Visual EQ component

**File:** `src/components/VisualEqPanel.tsx` (403 lines).

**Points rendered:** 4 — declared as a constant array `BANDS` (`VisualEqPanel.tsx:46-51`):

```typescript
const BANDS: readonly Band[] = [
  { id: "low",     label: "LOW",     hz: 200,  color: "#22d3ee", kind: "shelf-low",  qOctaves: 0   },
  { id: "low-mid", label: "LOW-MID", hz: 400,  color: "#4ade80", kind: "peak",       qOctaves: 1.0 },
  { id: "mid",     label: "MID",     hz: 1500, color: "#a78bfa", kind: "peak",       qOctaves: 1.2 },
  { id: "high",    label: "HIGH",    hz: 6000, color: "#60a5fa", kind: "shelf-high", qOctaves: 0   },
];
```

The frequencies, kinds, and Q-octaves are **hardcoded constants** that mirror the Rust chain; a comment at `VisualEqPanel.tsx:41-45` references the matching `dsp.rs` lines.

**Point positioning** (`VisualEqPanel.tsx:144-149, 350-353`):

```typescript
const gains: Record<BandId, number> = {
  "low":     settings.eq_low_db,
  "low-mid": settings.eq_low_mid_db,
  "mid":     settings.eq_mid_db,
  "high":    settings.eq_high_db,
};
// ...
const x = localFreqToX(band.hz);      // hardcoded frequency → log-scale X
const y = localDbToY(gains[band.id]); // band state → linear-dB Y
```

X is always fixed (frequencies are hardcoded). Y is driven by the user-offset state (no preset baseline overlay).

**Drag behavior** (`VisualEqPanel.tsx:198-243`):

- Pointer-down captures pointer on the band's hit-target.
- Pointer-move maps client Y → SVG-local Y → dB via `yToDbInPlot()`, then calls `onEq(band, db)` with the dB rounded to 0.1 (`Math.round(newDb * 10) / 10`).
- Pointer-up releases capture.
- Double-click on any node calls `onEq(band, 0)` — resets that band's user offset to 0 dB.
- **Vertical drag only.** Horizontal drag is intentionally not implemented; the file comment at lines 14-18 says: "V1 intentionally OMITS Horizontal drag (the DSP doesn't yet support variable band frequency or Q — the UI must not promise what the engine can't honor)."

**Color coding logic per point:** colors are hardcoded in the `BANDS` constant (above). They're used in three places: the node `fill`, the per-band label text fill, and the per-band value-readout text fill (`VisualEqPanel.tsx:361, 381, 392`). No conditional coloring (e.g., red for cuts vs green for boosts).

**Response-curve drawing** (`VisualEqPanel.tsx:91-116, 173-187`): the curve is an **approximation**, not the actual filter response. Peaks use a Gaussian shaped by `qOctaves`; shelves use a logistic sigmoid. The file comment at lines 7-12 calls this out: "an APPROXIMATION of the chain's filter cascade — Gaussian peaks + sigmoid shelves in log-frequency space — chosen to give the user a fast visual feedback loop, not numerically-exact dB-vs-frequency response."

**Live FFT spectrum overlay:** the component accepts an optional `spectrumDb: number[]` prop and renders a stepped fill behind the response curve (`VisualEqPanel.tsx:308-343`), driven by the Rust audio thread's `SPECTRUM_N_BINS = 32`.

---

## 6. The "extra" band

The 4th node is **LOW-MID**.

- **Frequency:** 400 Hz, peaking biquad, Q = 0.9 (`dsp.rs:718`, `VisualEqPanel.tsx:48`).
- **Role:** "mud-zone" scoop / lift between Low and Mid. The struct doc at `dsp.rs:283-286` says: "NEW band in Phase A2. Heavy presets carry significant CUTS here (the '250-800 Hz mud zone that muddies dense arrangements')." Phase A2 added it specifically because Codex's preset calibration required this control point.
- **Where it's controlled:**
  1. **Preset-driven baseline:** `PresetCalibration.low_mid_db` — values per preset listed in §2 (Oomph at −3.0 dB is the strongest cut, Warmth at +0.7 dB is the only meaningful lift).
  2. **User offset:** `MasteringSettings.eq_low_mid_db` — set only through dragging the LOW-MID node in `VisualEqPanel`. No knob, no slider, no Advanced-panel control.
- **Preset-driven?** Yes — preset baseline added at runtime via `effective_low_mid_db = preset.low_mid_db × preset_scale + settings.eq_low_mid_db`.
- **User-manipulable?** Yes, but **only via the Visual EQ drag** — there's no other UI surface for it. The Tone Shape knob row skips it deliberately.

The setter (`useTrackMaster.ts:861-864`) and the `onEq` prop signature in `Macros` (`App.tsx:1388`) both accept `"low-mid"` as a band ID; the Visual EQ is the only call site that uses it.

---

## 7. Tests covering the EQ

**`src-tauri/src/dsp.rs`** (in-file `#[cfg(test)] mod tests`):

- `chain_coeffs_default_width_is_neutral` (line 2445)
- `chain_coeffs_clamps_width_into_safe_range` (line 2473)
- `chain_coeffs_clamps_warmth_into_range` (line 2673)
- `presence_air_default_is_identity` (line 2708)
- `presence_air_at_one_lifts_10khz_band` (line 2737)
- `low_mid_band_centred_at_400hz_with_q_point_9` (line 3394)
- `heavy_presets_cut_low_mid_band` (line 3812)
- Also: shelf / peaking / butter_hp unit tests around `dsp.rs:1351-1374` (the `BiquadCoeffs` constructors)

**`src-tauri/tests/contracts.rs`:**

- `dsp_low_shelf_boost_raises_low_frequency_energy` (line 1332)
- Lines 1030-1031 use `magnitude_db_at(&cu.low_mid, ...)` and `&cp.low_mid` inside a larger test (function name not isolated in this pass).

**`src-tauri/tests/preset_signature.rs`:**

- `dump_observed_tilts` (line 156)
- `preset_signatures_match_calibration_tuples` (line 191)
- `preset_tape_introduces_third_harmonic_saturation` (line 307)
- `preset_warmth_introduces_third_harmonic_saturation` (line 327)
- `preset_spatial_widener_increases_side_signal_rms` (line 347)

**`src-tauri/tests/preset_distinctness.rs`:**

- `dump_observed_distinctness_metrics` (line 230)
- `clarity_drops_presence_and_lifts_air_relative_to_universal` (line 269)
- `oomph_lifts_sub_and_scoops_low_mid_relative_to_universal` (line 305)
- `tape_compresses_crest_relative_to_universal` (line 340)
- `punch_preserves_more_crest_than_loud` (line 357)
- One more test at line 388 (function name not captured in this pass)

**Other test files that construct `MasteringSettings` with the four `eq_*_db` fields** (default 0.0, so they're not exercising EQ directly but they pin the field shape):

- `tests/album_arc_trace.rs`
- `tests/album_character_bias.rs`
- `tests/album_plan_landing.rs`
- `tests/album_render.rs`
- `tests/album_simple_landing.rs`
- `tests/delivery_profile_render.rs`
- `tests/dither_absence_of_harmonics.rs`
- `tests/export_volume_match.rs`
- `tests/preset_loudness_balance.rs`

---

## 8. Surprising / non-obvious things

1. **"Dynamic" in the UI label is decorative.** The section header in `App.tsx:1468` reads `"EQUALIZER (Dynamic)"`, but the EQ stage itself is **purely static biquads** — no signal-dependent gain, no envelope-tracked Q, no sidechain. The only dynamic stage with frequency awareness is the **3-band multiband compressor** further down the chain (LR4 crossover split, peak-detector envelopes, soft 6 dB knee). The compressor is fully gain-controlled, not EQ-controlled. If the design intent ever was "dynamic EQ," the implementation today is "static EQ + multiband comp" in series.

2. **The 4th node has no knob — only drag.** Asymmetric UI surface: Low / Mid / High are reachable from both knobs *and* the Visual EQ; Low-Mid is reachable only by dragging the SVG node. The setter `setEqBand` accepts `"low-mid"` and the prop signature on `Macros` (`App.tsx:1388`) was widened in slice 4b to include `"low-mid"` for this exact reason.

3. **`warmth` and `presence_air` are EQ stages with non-EQ semantics.** They render in the audio chain as biquad shelves (`dsp.rs:739, 750`), but they're not surfaced as EQ in the UI:
   - **They have separate units** — both are `0..1` slider values mapping to `0..+4 dB` shelves, not bipolar dB knobs.
   - **They sit on `AdvancedSettings`**, not `MasteringSettings` top-level — so they live behind the Advanced panel, alongside compressor overrides and width.
   - **They're additive over the main EQ bands** — `warmth` adds another low-shelf at 300 Hz on top of the `low` band's 200 Hz; `presence_air` adds another high-shelf at 10 kHz on top of `high` at 6 kHz.
   - `VisualEqPanel.tsx:19-21` explicitly calls out this exclusion: "Warmth + Presence/Air nodes (different units — 0..1 saturation drive vs dB EQ — would need separate scaling and don't fit the same plot cleanly)."

4. **The "warmth" name is overloaded.** `PresetCalibration.warmth: f32` (`dsp.rs:295`) is the **saturation drive amount**, applied at the tanh stage post-EQ — NOT the EQ shelf. `AdvancedSettings.warmth` is the **300 Hz low-shelf**. Two separate concepts share the field name.

5. **Preset baseline isn't visible on the curve.** The Visual EQ plots `settings.eq_*_db` only, not `(preset.*_db × preset_scale) + settings.eq_*_db`. So switching presets doesn't move the nodes — they stay at user-offset (often 0 dB / center) even when the actual audible chain has shifted significantly. A user staring at a flat-looking Visual EQ on the Oomph preset is still getting +2.4 / −3.0 / −2.6 / −0.8 dB of preset shaping invisibly.

6. **Sub-highpass is preset-locked.** `highpass_hz` is on `PresetCalibration` (`dsp.rs:303-305`), clamped to [20, 40] Hz at runtime, and **not exposed in the UI** as either a knob, slider, or node. The user can't change the subsonic cutoff without changing the preset. Clarity gets 28 Hz, Oomph gets 22 Hz, Loud gets 30 Hz, etc.

7. **Custom preset is identity, not Universal.** `Custom` has all four EQ bands at 0.0, `highpass_hz: 0.0` (no HPF), `warmth: 0.0` (no saturation), `stereo_width: 1.0` (neutral), `transient_punch: 0.0`, and a special-case `default_density_for_preset = 0.0` for the compressor (`dsp.rs:843-847`) so a fresh Custom session is byte-identical to bypass. Switching from any other preset to Custom is therefore a **disabling action**, not a "use Universal's identity" action.

8. **The `science_note` field is preset metadata that lives in Rust but is never read on the frontend.** It's a `&'static str` rationale string for each preset (e.g., Loud: "Strongest density and limiting; assertive but not smashed — enough movement remains to read as a master, not a preview."). No `bindings.ts` exposure, no UI surface. It's there as in-source documentation.

9. **Preset baseline lives in Rust only.** The frontend has no way to display "what the preset is doing" — it can only display "what the user is adding." Any future "show effective curve" feature would require either (a) duplicating the calibration table in TS, (b) adding a Tauri command that returns the computed `effective_*_db` numbers, or (c) plumbing the chain coefficients out to the frontend.

10. **EQ ordering matters and is intentional.** The comment at `dsp.rs:1815-1816` documents that low-mid was inserted between low and mid "so the mud-zone cleanup (250–800 Hz) sits in the natural frequency order." The chain is frequency-monotonic: 200 → 400 → 1500 → 6000 → 300 (warmth) → 10000 (presence_air). The warmth stage comes after `high`, not after `low`, even though acoustically it overlaps `low`'s territory — placement choice favors keeping the Advanced shelves as a "post-tonal-shape" voicing pass rather than mixing them into the main EQ.

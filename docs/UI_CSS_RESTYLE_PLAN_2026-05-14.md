# UI CSS Restyle Plan - 2026-05-14

## Goal

Move the current Track Master UI toward the premium "studio console" direction in the reference screenshots without rebuilding the app logic or replacing functional controls with static images.

The current app already has the important product pieces:

- realtime Rust playback
- waveform and mini waveform
- Original / Mastered toggle
- preset image tiles
- CSS/SVG knobs
- right-rail meters
- export path

The work is mostly hierarchy, density, and polish. The interface should feel less like stacked debug panels and more like one mastering deck with a clear listening workflow.

## Asset Decision

Keep using the existing preset assets in:

`src/assets/presets/`

Current files:

- `universal.png`
- `clarity.png`
- `tape.png`
- `spatial.png`
- `oomph.png`
- `warmth.png`
- `punch.png`
- `loud.png`

Do not generate PNGs for:

- knobs
- meters
- waveform
- transport controls
- sliders
- buttons

Those should stay CSS/SVG/React because they need to resize, animate, respond to state, and remain accessible.

Optional later asset pass:

- Convert preset PNGs to WebP or AVIF.
- Target roughly 250-500 KB per preset tile if visual quality holds.
- Preserve transparent/dark-background renders and consistent lighting.

## Design Direction

Tone: dark professional mastering room, not sci-fi debug console.

Use the reference screenshots for structure:

- left rail: album/track list and import
- center: waveform deck, transport, presets, main controls
- right rail: master output, live levels, export
- bottom: minimal status

Avoid turning the app into a glossy mockup. The controls should still feel like usable software, not a fake hardware skin.

## Current Issues To Address

### 1. Main Surface Still Feels Like Panels Stacked Vertically

Current sections:

- waveform card
- transport card
- preset card
- signal chain card
- controls card
- stale/debug bar

These are individually decent, but together they create a banded layout. The reference UI groups the waveform and transport into one "deck", then groups presets and controls into one "console".

CSS direction:

- Merge visual treatment of `.waveform-card` and `.transport`.
- Reduce hard borders between waveform and transport.
- Use a single deck shadow/outline around both.
- Move transport visually closer to waveform.

Target structure:

```txt
.track-deck
  .waveform-card
  .transport

.mastering-console
  .presets
  .macros
```

Implementation can start in CSS without JSX changes by making neighboring sections share backgrounds and border radii. A later JSX pass can wrap them.

### 2. Debug / Migration Language Is Still Visible

Current user-facing examples:

- `Mastered playback is live -- drag controls and hear the change immediately.`
- `live: 0/0`
- `Re-render audit WAV`

These are technically useful, but they make the UI feel like a migration test harness.

CSS/product direction:

- Hide `.live-update-badge` outside dev builds.
- Move render-audit action out of the main footer row.
- Replace stale bar with compact session status:

```txt
Ready
Realtime
Peak -3.3 dBFS
GR L/M/H
```

If render-audit remains, place it in a secondary menu or export tools area.

### 3. Preset Tiles Are Good, But Need More "Selected Card" Drama

The existing assets are worth keeping. The selected tile should feel like a chosen mastering direction, not just a bordered card.

CSS direction:

- Make preset images slightly larger.
- Let selected tile use its `--tile-accent` more strongly.
- Add a low, colored floor glow behind the asset.
- Reduce text clutter inside each tile.
- Keep all eight tiles in one row at wide desktop.

Suggested CSS shape:

```css
.preset-tile {
  position: relative;
  min-height: 136px;
  background:
    radial-gradient(circle at 50% 28%, color-mix(in srgb, var(--tile-accent) 26%, transparent), transparent 46%),
    linear-gradient(180deg, rgba(31,37,51,.84), rgba(13,16,24,.96));
  border: 1px solid color-mix(in srgb, var(--tile-accent) 18%, var(--border));
}

.preset-tile.is-active {
  border-color: color-mix(in srgb, var(--tile-accent) 72%, white 8%);
  box-shadow:
    0 0 0 1px color-mix(in srgb, var(--tile-accent) 28%, transparent),
    0 18px 38px rgba(0, 0, 0, .34),
    0 0 32px color-mix(in srgb, var(--tile-accent) 24%, transparent);
}

.preset-icon-img {
  width: clamp(64px, 5.8vw, 92px);
  height: clamp(64px, 5.8vw, 92px);
  object-fit: contain;
  filter: saturate(1.08) contrast(1.04);
}
```

Class names may differ; align with the live `App.css` selectors during implementation.

### 4. Knobs Should Stay Code-Generated

`src/components/Knob.tsx` is already the right direction: SVG ticks, metallic cap, accent arc, pointer drag, wheel support, double-click reset.

Do not replace this with generated knob PNGs.

CSS direction:

- Give the large intensity knob a stronger cockpit role.
- Make small EQ knobs sit in one dark recessed bay.
- Use per-band color:
  - Low: cyan
  - Mid: green
  - High: purple
  - Presence/Air: pink
  - Width: gold
  - Compression: blue/cyan

Suggested treatment:

```css
.macros {
  background:
    linear-gradient(180deg, rgba(24, 29, 41, .94), rgba(12, 15, 23, .98));
  border: 1px solid rgba(111, 163, 255, .14);
  box-shadow:
    inset 0 1px 0 rgba(255,255,255,.04),
    0 20px 42px rgba(0,0,0,.28);
}

.knob-lg .knob-vis {
  filter: drop-shadow(0 0 22px rgba(77,139,255,.22));
}
```

### 5. Add A Visual EQ Panel, But Make It Code-Rendered

The reference-style EQ/spectrum panel is a strong fit for this product. It gives the user a direct mental model for tone shaping instead of making them infer everything from knobs.

Do not use a static image for this. Build it as a real component:

```txt
src/components/VisualEqPanel.tsx
  canvas.eq-spectrum     optional analyzer / spectrum fill
  svg.eq-overlay         grid, response curve, band nodes, labels
```

Recommended v1:

- SVG frequency grid from 20 Hz to 20 kHz on a logarithmic x-axis.
- Gain grid from -12 dB to +12 dB on the y-axis.
- Fixed-frequency nodes for the DSP bands the app already has.
- Vertical drag changes gain.
- Double-click resets a band to 0 dB.
- Curve updates immediately from current settings.
- Mastered playback hears changes via the existing realtime `update_chain` path.
- Spectrum fill can be omitted in v1 if live FFT data is not ready.

Important constraint:

If Rust DSP frequencies are fixed, v1 nodes should drag only up/down. Do not let users drag nodes left/right until the DSP actually supports adjustable frequency and Q. The UI should not promise a parameter the audio engine cannot honor.

Suggested band mapping:

| Node | Setting | Display Frequency | Color |
|---|---|---:|---|
| Low | `eq_low_db` | 120 Hz or 200 Hz | cyan |
| Low-Mid | `eq_low_mid_db` | 400 Hz | green |
| Mid | `eq_mid_db` | 1.5 kHz or 2.5 kHz | purple |
| High | `eq_high_db` | 6 kHz or 10 kHz | blue |
| Air | `advanced.presence_air` | 10 kHz+ | pink |
| Warmth | `advanced.warmth` | 300 Hz shelf | gold |

The exact displayed frequencies should match `ChainCoeffs::from_settings` in `src-tauri/src/dsp.rs`, not arbitrary UI taste.

Layout recommendation:

```txt
.mastering-console
  .visual-eq-panel
  .macro-controls
```

The visual EQ should sit where the current lower tone-shape area lives, or directly above the knobs. The knobs remain as precision controls; the visual EQ becomes the primary "shape the tone" surface.

Suggested CSS shape:

```css
.visual-eq-panel {
  position: relative;
  min-height: clamp(220px, 24vh, 320px);
  border: 1px solid var(--line-soft);
  border-radius: var(--radius);
  overflow: hidden;
  background:
    linear-gradient(180deg, rgba(8, 12, 20, .98), rgba(5, 8, 14, .98));
  box-shadow:
    inset 0 1px 0 rgba(255,255,255,.04),
    0 18px 42px rgba(0,0,0,.28);
}

.eq-spectrum {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  opacity: .54;
}

.eq-overlay {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}

.eq-grid-major {
  stroke: rgba(132, 154, 205, .18);
  stroke-width: 1;
}

.eq-grid-minor {
  stroke: rgba(132, 154, 205, .07);
  stroke-width: 1;
}

.eq-zero-line {
  stroke: rgba(236, 240, 250, .28);
  stroke-width: 1.2;
}

.eq-response-fill {
  fill: rgba(77, 139, 255, .12);
}

.eq-response-line {
  fill: none;
  stroke: var(--accent-bright);
  stroke-width: 3;
  filter: drop-shadow(0 0 12px rgba(77,139,255,.45));
}

.eq-node {
  cursor: ns-resize;
  filter: drop-shadow(0 0 12px color-mix(in srgb, var(--node-color) 55%, transparent));
}

.eq-node-hit {
  fill: transparent;
  pointer-events: all;
}

.eq-label {
  fill: var(--text-2);
  font-size: .72rem;
  letter-spacing: .04em;
  text-transform: uppercase;
}
```

Suggested v2:

- Add a live FFT analyzer from the native playback path.
- Let users enable/disable individual bands.
- Add frequency drag only after Rust supports variable band frequency.
- Add Q/width handles only after Rust supports variable Q.
- Add pre/post spectrum toggle if useful.

Why do this now:

The visual EQ affects layout. If the restyle first locks in the lower console as knobs-only, the EQ will feel bolted on later. Reserve the panel space now, even if v1 starts as static response curve plus draggable gain nodes.

### 6. Waveform Deck Should Be The Hero

The waveform is working. The issue is visual priority.

CSS direction:

- Taller waveform at desktop.
- Darker surrounding deck, less flat interior.
- Stronger active playhead.
- More contrast between main waveform and overview.
- Integrate time scale and dB scale into the deck instead of looking like separate labels.

Suggested deck treatment:

```css
.waveform-card {
  min-height: 310px;
  background:
    linear-gradient(180deg, rgba(8, 13, 23, .98), rgba(5, 8, 14, .98));
  border-color: rgba(111, 163, 255, .16);
  box-shadow:
    inset 0 1px 0 rgba(255,255,255,.035),
    0 24px 60px rgba(0,0,0,.32);
}

.waveform-main {
  filter: drop-shadow(0 0 14px rgba(77,139,255,.18));
}

.wf-playhead {
  stroke: rgba(235, 241, 255, .9);
  stroke-width: 1.5;
}
```

### 7. Transport Should Feel Attached To Playback, Not A Separate Card

Current transport is readable but large and detached.

CSS direction:

- Reduce top/bottom empty space.
- Group controls around one primary play button.
- Put Original/Mastered and Volume Match in the same horizontal comparison cluster.
- Keep one clear Play/Pause button.

Preferred order:

```txt
[Play] [time] [loop]                         [Original | Mastered] [Volume Match]
```

Avoid adding extra playback modes.

### 8. Right Rail Should Become Premium Meter / Export Rail

The right rail is already close. It needs cleaner order and less advanced-control dominance.

Preferred order:

1. MASTER OUT meter
2. live levels / GR
3. Export Master button
4. Quality checks
5. Advanced controls collapsed by default

Reason:

Advanced controls are useful, but in the reference UI the right rail sells confidence: meters, quality, export. The current rail lets technical controls visually dominate the meters.

CSS direction:

- Keep meter tower tall.
- Increase export CTA prominence.
- Collapse or visually quiet advanced controls.
- Reduce explanatory copy in the delivery profile panel.

### 9. Bottom Status Should Be Quiet

The bottom bar should not compete with the app.

Preferred content:

```txt
Analyzed  |  Quality not run  |  Peak -3.3 dBTP  |  Loudness -14.6 LUFS  |  Processing Ready
```

Avoid long sentences in the bottom status.

## CSS Token Pass

Current tokens are workable:

```css
--bg-0
--bg-1
--bg-2
--bg-3
--border
--text-0
--text-1
--accent
--accent-bright
--accent-deep
--accent-warm
```

Add semantic tokens before restyling:

```css
:root {
  --panel: rgba(17, 21, 31, .92);
  --panel-raised: rgba(24, 30, 43, .94);
  --panel-deep: rgba(7, 10, 17, .98);
  --line-soft: rgba(132, 154, 205, .12);
  --line-bright: rgba(132, 174, 255, .24);
  --glow-blue: 0 0 28px rgba(77, 139, 255, .28);
  --glow-cyan: 0 0 28px rgba(34, 211, 238, .22);
  --meter-green: #87d37c;
  --meter-yellow: #f5c84c;
  --meter-red: #ff6b6b;
}
```

Then migrate component CSS to semantic tokens rather than hard-coding more one-off blues.

## Suggested Implementation Slices

### Slice 1 - Hide Debug Surface

Files:

- `src/App.tsx`
- `src/App.css`

Work:

- Remove or hide `live: applied/attempts`.
- Rename or move `Re-render audit WAV`.
- Replace long live-playback sentence with compact status.

Acceptance:

- User can understand the main workflow without reading any technical caveats.

### Slice 2 - Deck Polish

Files:

- `src/App.css`
- optionally `src/App.tsx` if adding a `.track-deck` wrapper

Work:

- Visually join waveform + transport.
- Make waveform deck stronger.
- Tighten transport spacing.

Acceptance:

- The waveform/transport reads as the primary instrument.

### Slice 3 - Preset Tiles

Files:

- `src/App.css`
- `src/components/PresetIcon.tsx` only if adding loading hints or WebP imports

Work:

- Larger preset images.
- Better selected state.
- Per-preset glow.
- Optional WebP conversion later.

Acceptance:

- Preset row feels premium and intentional, not a generic card strip.

### Slice 4 - Console Controls

Files:

- `src/App.css`
- `src/components/Knob.tsx` only if needed

Work:

- Rebalance intensity and tone knobs.
- Make controls feel like one console panel.
- Use distinct knob tones consistently.
- Reserve visual space for `VisualEqPanel` even if the first implementation only renders the EQ response without live FFT.

Acceptance:

- Main controls are easier to scan than the advanced rail.

### Slice 4b - Visual EQ V1

Files:

- `src/components/VisualEqPanel.tsx`
- `src/App.tsx`
- `src/App.css`

Work:

- Add log-frequency grid.
- Add fixed-frequency EQ nodes.
- Add response curve derived from current settings.
- Add vertical drag for gain-only updates.
- Wire node updates to the same settings paths as the existing knobs.

Acceptance:

- Dragging a visual EQ node while Mastered playback is active changes audio in realtime.
- Nodes do not move horizontally until Rust supports variable frequency.
- No static PNG controls are introduced.

### Slice 5 - Right Rail Reorder

Files:

- `src/components/RightRail.tsx`
- `src/App.css`

Work:

- Export CTA moves above advanced controls.
- Advanced controls collapsed by default or visually reduced.
- Meter panel keeps the most attention.

Acceptance:

- Right rail says "meter, quality, export" before it says "technical settings".

### Slice 6 - Responsive Check

Files:

- `src/App.css`

Work:

- Verify 1920x1080, 1600x900, 1366x768.
- Ensure preset row does not overflow.
- Ensure right rail does not bury Export.
- Ensure text does not overlap in buttons, tiles, or meter cards.

Acceptance:

- No horizontal scroll at common desktop sizes.
- Export and playback remain visible.

## What Not To Do

- Do not add another preview mode.
- Do not add more user-facing caveats to explain migration state.
- Do not bake UI controls into images.
- Do not make the right rail a long always-open settings form.
- Do not let preset artwork inflate the app bundle unnecessarily.
- Do not flatten everything into the same card style.

## Best Next Step

Start with Slice 1 and Slice 2. That will change the feel immediately without risking DSP, export, or state behavior.

The target first impression should be:

> Load track. See waveform. Press Play. Toggle Mastered. Move a knob. Hear it.

Everything else should support that sequence.

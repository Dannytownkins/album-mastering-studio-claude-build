# Preset Tile Art Prompts

Eight image prompts for the eight Track Master presets. Each is written so
the generator treats it as a **UI tile asset** rather than a fine-art piece —
that nudges the model toward a clean, centered subject, consistent rendering
style, and the negative space we need around the edges so the image sits
properly inside a software tile in the app.

## How to use

1. Pick a generator: **Midjourney v6.1+**, **DALL-E 3**, **Imagen 3**, or
   **Stable Diffusion XL / SDXL** — any model that can produce 1024×1024
   PNG output works.
2. Generate each prompt once, regenerate as needed until you're happy.
3. Save each output as a **transparent PNG** (file format `.png` with an
   alpha channel) using the exact filename shown next to each prompt.
4. Drop the eight PNG files into `src/assets/presets/` (Claude will create
   that folder when it wires them up).
5. If your generator can't produce a true alpha channel (most can't natively
   today — DALL-E and Imagen output a flat black or white background), the
   prompts already ask for a pure black background. Run the files through
   <https://www.remove.bg/> or Photopea's magic-wand-on-black workflow to
   strip the black to transparency. Midjourney v6.1's `--transparent`
   parameter gives you alpha directly.
6. Each prompt explicitly names "1:1 square aspect ratio" and "1024×1024".
   For Midjourney, also append `--ar 1:1 --style raw` at the end of each
   prompt. For DALL-E and Imagen, just paste the prompt as-is — those
   models read the resolution and aspect ratio from the natural-language
   description.

## Style anchors (already baked into every prompt below)

- Photorealistic 3D render — Octane / Redshift product-shot style.
- Centered subject floating in transparent space with generous padding on
  all four sides.
- Strong rim/back lighting and a soft volumetric glow matching the subject's
  character color.
- Reference aesthetic: premium audio software plugin tiles
  (iZotope Ozone, FabFilter, UAD, Sonible, Soundtheory).
- No text, no watermark, no logos, no surface or floor — the subject must
  not be sitting on anything.

## The eight prompts

### 1 · Universal → save as `universal.png`

> A premium UI tile asset for a desktop audio mastering application,
> 1024×1024 pixels, 1:1 square aspect ratio, output as a transparent PNG
> with a clean alpha channel (use a pure black background that can be
> removed if the generator cannot produce true transparency).  The subject
> is a translucent electric-blue sphere floating in the exact center of
> the frame, suggesting a balanced, universal default preset. The sphere
> has delicate luminous cyan latitude and longitude gridlines wrapping
> around its surface, and a soft volumetric blue glow radiating outward
> from inside.  The material is glassy and refractive, with crisp specular
> highlights from a single key light above and a rim light from behind.
> Photorealistic 3D render in the style of premium audio plugin product
> art (think iZotope or FabFilter), Octane/Redshift quality, ultra-sharp
> focus, professional studio lighting, no text, no logos, no surface
> beneath the sphere — it floats in space with generous empty padding on
> all four sides of the frame.

### 2 · Clarity → save as `clarity.png`

> A premium UI tile asset for a desktop audio mastering application,
> 1024×1024 pixels, 1:1 square aspect ratio, output as a transparent PNG
> with a clean alpha channel (use a pure black background that can be
> removed if the generator cannot produce true transparency).  The
> subject is a small cluster of pristine crystalline quartz shards
> fanning upward from a central base in the exact center of the frame,
> suggesting precision, focus, and high-frequency sparkle. The crystals
> are ice-blue and white, with sharp angular facets that catch and
> refract light, throwing tiny prism rainbows. A soft cyan-white inner
> glow emanates from within the cluster, and a rim light from behind
> defines the edges of each shard.  Photorealistic 3D render in the
> style of premium audio plugin product art, Octane/Redshift quality,
> ultra-sharp focus, glassy refractive materials, no text, no logos, no
> surface beneath the cluster — it floats in space with generous empty
> padding on all four sides of the frame.

### 3 · Tape → save as `tape.png`

> A premium UI tile asset for a desktop audio mastering application,
> 1024×1024 pixels, 1:1 square aspect ratio, output as a transparent PNG
> with a clean alpha channel (use a pure black background that can be
> removed if the generator cannot produce true transparency).  The
> subject is a vintage analog magnetic tape reel seen at a slight
> three-quarter angle in the exact center of the frame, suggesting
> analog warmth, saturation, and retro hi-fi character. The reel has
> matte-black plastic flanges with three triangular cut-outs, a polished
> brass center hub, and warm amber-translucent recording tape spooled
> around it. A subtle motion blur on the spinning reel suggests it is
> playing back. A warm amber-orange glow surrounds the reel, with a
> backlight rim defining its silhouette.  Photorealistic 3D render in
> the style of premium audio plugin product art, Octane/Redshift
> quality, ultra-sharp focus, tactile materials, no text, no logos, no
> surface beneath the reel — it floats in space with generous empty
> padding on all four sides of the frame.

### 4 · Spatial → save as `spatial.png`

> A premium UI tile asset for a desktop audio mastering application,
> 1024×1024 pixels, 1:1 square aspect ratio, output as a transparent PNG
> with a clean alpha channel (use a pure black background that can be
> removed if the generator cannot produce true transparency).  The
> subject is a swirling cosmic nebula contained inside a glassy
> spherical orb in the exact center of the frame, suggesting space,
> width, and stereo dimension. The nebula inside the orb has deep
> purple, magenta, and violet clouds with tiny pinpoint stars and a
> sense of depth — you can see through the front of the sphere into the
> swirling material behind. A soft pink-purple volumetric glow surrounds
> the orb, with a back rim light defining its outer edge.
> Photorealistic 3D render in the style of premium audio plugin product
> art, Octane/Redshift quality, ultra-sharp focus, glassy refractive
> materials with volumetric internal effects, no text, no logos, no
> surface beneath the orb — it floats in space with generous empty
> padding on all four sides of the frame.

### 5 · Oomph → save as `oomph.png`

> A premium UI tile asset for a desktop audio mastering application,
> 1024×1024 pixels, 1:1 square aspect ratio, output as a transparent PNG
> with a clean alpha channel (use a pure black background that can be
> removed if the generator cannot produce true transparency).  The
> subject is a black subwoofer speaker driver seen from the front,
> centered in the frame, with a concentric low-frequency pressure wave
> visibly rippling outward through the air around it, suggesting low-end
> weight and bass impact. The cone is matte black with a chrome dust
> cap and a heavy black rubber surround, and a deep crimson-red glow
> radiates from the center of the cone outward into the surrounding
> pressure wave, which is rendered as semi-transparent red-orange
> ripples.  Photorealistic 3D render in the style of premium audio
> plugin product art, Octane/Redshift quality, ultra-sharp focus, no
> text, no logos, no surface beneath the speaker — it floats in space
> with generous empty padding on all four sides of the frame.

### 6 · Warmth → save as `warmth.png`

> A premium UI tile asset for a desktop audio mastering application,
> 1024×1024 pixels, 1:1 square aspect ratio, output as a transparent PNG
> with a clean alpha channel (use a pure black background that can be
> removed if the generator cannot produce true transparency).  The
> subject is a single vintage 12AX7-style vacuum tube standing upright
> in the exact center of the frame, suggesting analog warmth and
> tube-style harmonic saturation. The tube has clear glass with internal
> metal plates and a vivid amber-orange filament glow visible inside.
> The base is a brass-colored cap with eight gold pins. A warm
> amber halo radiates outward from the tube, with a soft back rim light
> highlighting the glass curvature.  Photorealistic 3D render in the
> style of premium audio plugin product art, Octane/Redshift quality,
> ultra-sharp focus, glassy translucent materials, no text, no logos, no
> surface beneath the tube — it floats in space with generous empty
> padding on all four sides of the frame.

### 7 · Punch → save as `punch.png`

> A premium UI tile asset for a desktop audio mastering application,
> 1024×1024 pixels, 1:1 square aspect ratio, output as a transparent PNG
> with a clean alpha channel (use a pure black background that can be
> removed if the generator cannot produce true transparency).  The
> subject is a red leather boxing glove frozen mid-strike in the exact
> center of the frame, suggesting transient impact and punch. The glove
> is crimson and scarlet with visible leather grain and stitching,
> angled toward the viewer as if just landing a hit. A dramatic rim
> light from behind defines its silhouette, and a soft red-orange glow
> radiates outward as if the strike has compressed the air. Light
> motion-blur trails suggest the moment of impact.  Photorealistic 3D
> render in the style of premium audio plugin product art,
> Octane/Redshift quality, ultra-sharp focus, no text, no logos, no
> surface beneath the glove — it floats in space with generous empty
> padding on all four sides of the frame.

### 8 · Loud → save as `loud.png`

> A premium UI tile asset for a desktop audio mastering application,
> 1024×1024 pixels, 1:1 square aspect ratio, output as a transparent PNG
> with a clean alpha channel (use a pure black background that can be
> removed if the generator cannot produce true transparency).  The
> subject is a brilliant electric-blue lightning bolt frozen in the
> exact center of the frame, suggesting density, energy, and loudness.
> The bolt is jagged and angular with a bright white-hot core and
> cyan-blue electric arcs branching outward in fractal patterns. A
> strong volumetric blue glow surrounds the bolt, and a faint
> atmospheric haze suggests the air around it has been ionized by the
> discharge.  Photorealistic 3D render in the style of premium audio
> plugin product art, Octane/Redshift quality, ultra-sharp focus, no
> text, no logos, no surface beneath the bolt — it floats in space with
> generous empty padding on all four sides of the frame.

## Tips for getting consistent output

- **Generate one prompt first** (Universal is a good baseline) and check
  the lighting / framing / padding. If you like it, copy the same model
  / sampler / seed settings across all eight so they feel like a set.
- **Keep the background pure black** if your generator can't do true
  alpha. The remove-bg step is reliable on flat-black backgrounds and
  much messier on mid-tone gradients.
- **Reject any output with text, watermarks, surfaces under the
  subject, or asymmetric off-center framing.** Easier to regenerate
  than to fix in post.
- **Don't overthink color matching.** The tile background in the app
  will be a dark gradient (`#1f2533`-ish), so the assets just need to
  be vivid against transparent — Claude will handle the tile chrome
  around them.

## When done

Drop the eight PNGs into `src/assets/presets/`, and tell me. I'll wire
them into `PresetIcon.tsx` (or its replacement) and they'll appear in
the preset row immediately.

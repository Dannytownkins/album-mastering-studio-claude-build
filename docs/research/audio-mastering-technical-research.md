# Deep Research: Audio Mastering — Technical Standards, Workflows & Implementation Reference

> Research compiled 2026-05-11 for the **album-mastering-studio** project.
> Sources: ITU-R, EBU, Apple, Spotify, iZotope, FabFilter, Sound on Sound, Bob Katz, AES community references. Tiered Gold / Silver / Bronze in the Sources section.

---

## Executive Summary

Audio mastering in 2026 is bounded by two convergent technical regimes: the **loudness-normalization era** (ITU-R BS.1770 and platform-specific LUFS targets, replacing the peak-normalization "loudness war") and **lossy-codec-aware delivery** (true-peak limiting with headroom for AAC/Opus reconstruction). The canonical signal chain — corrective EQ → compression → saturation → tonal EQ → stereo processing → limiter → dither — has remained stable across two decades, but the **measurement, target levels, and limiter behavior** are now standardized in ways that make algorithmic mastering tractable. For software you build, the load-bearing technical primitives are: a BS.1770-5–compliant LUFS meter (K-weighted, gated), an oversampled true-peak detector (≥4× oversampling), a lookahead brickwall limiter, TPDF dither with optional noise shaping, and high-quality polyphase SRC. Everything else (EQ, compression, stereo, saturation) is craft layered on top.

## Key Findings

- **Loudness measurement is fully specified.** ITU-R BS.1770-5 (Nov 2023) defines the K-weighting filter, channel weights, gating, and true-peak measurement to the bit-exact level. EBU R128 wraps this for delivery practice (−23 LUFS, −1 dBTP, ±0.5 LU). All major streaming platforms have converged on −14 to −16 LUFS integrated [1][2][3].
- **The "−14 LUFS / −1 dBTP" master is now universal.** A single master at ≈−14 LUFS integrated with −1 dBTP true-peak ceiling falls within preferred range for Spotify, YouTube, Tidal, Amazon, and Deezer; only Apple (−16 LUFS) and Qobuz (−18 LUFS) diverge meaningfully, and only Apple/Spotify *boost* quieter masters [3].
- **Inter-sample peaks are the silent killer.** Lossy codecs (AAC, Opus, MP3) reconstruct waveforms that can exceed the sample-domain peak by **+0.5 to +3 dB**; a 0 dBFS sample-peak master routinely measures +1 to +2 dBTP after AAC. Spotify explicitly recommends −2 dBTP on louder masters; Apple Digital Masters mandates −1 dBTP, verified with their `afclip` tool [4][5][6].
- **The signal chain is canonical, not arbitrary.** Industry consensus order: corrective EQ → compression (1.5–2:1, light) → saturation → multiband (surgical) → mid-side processing → tonal/additive EQ → limiter → dither. Variations exist (some engineers compress before EQ), but the limiter and dither are always last [7].
- **Limiter design = lookahead + oversampled ISP detection.** Modern brickwall limiters use 1–5 ms lookahead and ≥4× internal oversampling to predict and clip inter-sample peaks before the host's DAC stage [8][9].
- **Dithering rules: TPDF at 2 LSB peak-to-peak, applied once, at the final bit-depth reduction.** Noise shaping (Lipshitz-Vanderkooy filters, POW-r, Sony SBM) pushes quantization noise into the 15–20 kHz region where hearing sensitivity collapses [10][11].
- **The K-weighting filter is two cascaded biquads.** Sample-rate-dependent coefficients are published; libebur128 uses canonical 48 kHz values that implementers re-derive for other rates [12].
- **Genre-based tonal-balance targets are derived from corpus analysis** (thousands of commercial masters), not engineering first principles — important if you're building reference-matching [13].
- **AI mastering is hybrid DSP+ML.** LANDR, eMastered, CloudBounce do not "neural-network the audio"; they analyze the source (spectrum, dynamics, ITD/IID, transient density), select and parameterize a conventional DSP chain via learned mappings, and optionally do reference-match by spectral subtraction toward a target curve [14].

---

## Detailed Analysis

### 1. Loudness Measurement — ITU-R BS.1770-5 in Depth

The ITU-R BS.1770 family (BS.1770-1 through -5, current as of Nov 2023) is the **measurement substrate** that every other loudness specification — EBU R128, ATSC A/85, AES TD1004, all streaming platform specs — references rather than redefines. Implementing this correctly is the single most important DSP requirement in a mastering program.

**Algorithm overview (4 stages):** [1][2]

1. **K-weighting filter** — a two-stage biquad cascade per channel
2. **Mean-square calculation** per channel over a sliding window
3. **Channel-weighted sum** with prescribed weights (LFE excluded)
4. **Gating** with absolute and relative thresholds, 400 ms blocks, 75% overlap

**K-weighting filter specification.** Two biquads in series: a high-shelf "pre-filter" that simulates the head's acoustical effects (roughly +4 dB shelf at ~1681 Hz), followed by the "RLB" high-pass that approximates the equal-loudness contour's low-frequency rolloff (~38 Hz corner). Canonical published 48 kHz coefficients (used in libebur128 and pyloudnorm) [12]:

```
Stage 1 — Pre-filter (high-shelf):
  b0 =  1.53512485958697
  b1 = -2.69169618940638
  b2 =  1.19839281085285
  a1 = -1.69065929318241
  a2 =  0.73248077421585

Stage 2 — RLB (high-pass):
  b0 =  1.0
  b1 = -2.0
  b2 =  1.0
  a1 = -1.99004745483398
  a2 =  0.99007225036621
```

**Critical implementation note:** these coefficients are *only correct at 48 kHz*. BS.1770-5 specifies the analog prototype (poles and zeros in s-domain); implementers must use the bilinear transform with frequency pre-warping to derive coefficients for other rates (44.1, 88.2, 96, 192 kHz). Naive coefficient reuse causes audible measurement error [12].

**Channel weights (BS.1770-5 §3.1):**
- L, R: 1.0
- C: 1.0
- Ls, Rs (surround): 1.41
- LFE: 0.0 (excluded; this is **mandatory**, not optional)

**Gating algorithm:**
- Block size: **Tg = 400 ms**, overlap **75%** (hop = 100 ms)
- **Absolute gate Γa = −70 LUFS** — discard any block below this
- **Relative gate Γr = (ungated mean) − 10 LU** — compute mean of remaining blocks, drop blocks below this, recompute
- **Integrated loudness** = mean of surviving blocks (no fixed window — measured over entire program) [1]

**Three measurement timeframes:**

| Measure | Window | Use |
|---|---|---|
| **Momentary (M)** | 400 ms sliding | Peaks of perceived loudness |
| **Short-term (S)** | 3 sec sliding | Programme-level monitoring |
| **Integrated (I)** | Full programme, gated | Compliance / delivery target |

**Loudness Range (LRA):** Statistical descriptor of dynamic range over time. Computed as the difference between the 95th and 10th percentile of short-term loudness values (gated). Defined in EBU Tech 3342. Typical mastered values [16]:
- EDM: ~3–6 LU
- Pop: ~3.7–12 LU (wide variance)
- Rock: ~6–10 LU
- Jazz: ~13–23 LU
- Classical/Orchestral: ~20–32 LU (often unmastered for loudness)

**True-peak (dBTP) measurement.** Defined in BS.1770 Annex 2. The audio is **oversampled ≥4×** (minimum), then the absolute-value peak of the oversampled signal is taken. Oversampling is implemented as polyphase upsampling with anti-imaging filter. 4× is the spec minimum; high-end limiters (FabFilter Pro-L 2, Weiss MM-1) oversample 8× or 16× for sub-0.1 dB ISP accuracy [4][5][8].

### 2. Platform Delivery Targets

Single most actionable table for software defaults [3][15][17]:

| Platform | Integrated LUFS | True-peak ceiling | Behavior on louder masters | Behavior on quieter masters |
|---|---|---|---|---|
| **Spotify** (Normal) | −14 LUFS | −1 dBTP (−2 recommended on loud) | Attenuates to −14 | Boosts to −14 (with limiter) |
| **Spotify** (Loud) | −11 LUFS | −1 dBTP | Attenuates | Boosts (caps at −11) |
| **Spotify** (Quiet) | −19 LUFS | −1 dBTP | Attenuates | Boosts |
| **Apple Music** | −16 LUFS | −1 dBTP | Attenuates | Boosts (no limiter; preserves dynamics) |
| **YouTube** | −14 LUFS | −1 dBTP | Attenuates | **Does not boost** |
| **Tidal** | −14 LUFS | −1 dBTP | Attenuates | Does not boost |
| **Amazon Music** | −14 LUFS | −2 dBTP | Attenuates | Does not boost |
| **Deezer** | −15 LUFS | −1 dBTP | Attenuates | Does not boost |
| **SoundCloud** | −14 LUFS (approx) | −1 dBTP | Attenuates | Does not boost |
| **EBU R128 (broadcast)** | −23 LUFS ±0.5 | −1 dBTP | Strict compliance | Strict compliance |
| **ATSC A/85 (US TV)** | −24 LKFS ±2 | −2 dBTP | Strict compliance | Strict compliance |
| **Netflix originals** | −27 LKFS ±2 | −2 dBTP | Strict compliance | Strict compliance |

**The universal master heuristic** (well-supported across sources): aim **−14 LUFS integrated, −1 dBTP**, **LRA ≥ 5 LU** for pop/rock. This satisfies all major streaming platforms without separate delivery masters. Genre-appropriate dynamic range matters more than the LUFS number itself; an over-limited −9 LUFS master will be attenuated by Spotify *and* sound flat at playback level [3][17].

### 3. Canonical Mastering Signal Chain

Consensus order across iZotope, Sage Audio, Sound on Sound, Mastering The Mix, and practitioner sources [7]:

```
WAV input  →  [optional 32-bit float upsampling]
            →  Corrective EQ (subtractive, surgical)
            →  Compression (broadband, 1.2–2:1, light)
            →  Saturation / harmonic enhancement (subtle)
            →  Multiband compression (only if needed)
            →  Mid-Side processing (EQ and/or compression)
            →  Tonal / additive EQ (broad, musical)
            →  Stereo imager (width control)
            →  Brickwall limiter (true-peak, ceiling −1 dBTP)
            →  Dither (TPDF + noise shaping) [only when reducing bit depth]
            →  Output: target sample rate / bit depth
```

**Order is convention, not law.** Common variations:
- Compression before corrective EQ (when dynamics are masking tonal problems)
- Saturation before compression (analog tape emulation use case)
- Multiband replaces broadband for problem-frequency-only material
- Stereo imager moved earlier when L-R imbalance is severe

**What does NOT belong in a mastering chain:** reverb, delay, gates, expanders (except occasional downward expansion for noise), heavy de-essing (that's a mix problem), creative pitch shifting. Mastering processes the **whole programme**; mix processes individual elements.

### 4. EQ — Technical Parameters

**Two roles, two EQs.** [7][18]

**Corrective EQ (first in chain):**
- Surgical cuts, narrow Q (4–10), small attenuations (−1 to −3 dB)
- Typical targets: low-mid mud (200–400 Hz), boxiness (300–500 Hz), harshness (2.5–4 kHz), sibilance buildup (5–8 kHz)
- High-pass filter: 20–30 Hz (12–24 dB/oct slope) to remove sub-rumble
- Phase mode: **minimum phase preferred** for transients; preserves "feel"

**Tonal/additive EQ (later in chain):**
- Broad boosts, wide Q (0.5–1.5), small lifts (+0.5 to +2 dB)
- Typical targets: weight (60–120 Hz shelf), presence (3–6 kHz bell), air (10–16 kHz shelf)
- Phase mode: **linear phase preferred** for symmetric impulse response across the spectrum

**Linear vs minimum phase tradeoff:** [19]

| Property | Minimum Phase | Linear Phase |
|---|---|---|
| Phase response | Frequency-dependent shift | Identical delay all frequencies |
| Latency | Near-zero | 3,000–66,000 samples (typical 10–30k) |
| Pre-ringing | None | Present (audible "smear" before transients) |
| CPU | Low | High (long FIR) |
| Best for | Transient-rich material, low end | Mastering broad tonal shaping |
| Boost vs cut | Equal | **Cut produces less pre-ring than boost** |

**Mid/Side EQ specifics:**
- Mid: clean up center buildup (vocals, kick, snare, bass)
- Side: high-shelf boost (+1 to +2 dB above 6 kHz) for "air" width
- **Never widen below 120–200 Hz** — bass should be mono (vinyl compatibility, mono playback summing) [20]
- Recommended: linear-phase M/S EQ to preserve phase relationship on L-R reconstruction

### 5. Compression — Technical Parameters

Mastering compression is **glue, not control.** [7][21]

**Broadband (single-band) mastering compressor settings:**
- Ratio: **1.2:1 to 2:1** (rarely above 2.5:1; >2:1 = audible pumping)
- Threshold: set for **1–3 dB gain reduction at peaks**
- Attack: **10–30 ms** (preserves transients)
- Release: **100–250 ms** or auto/program-dependent
- Knee: soft (3–6 dB) for transparency
- Makeup gain: bypass-matched (A/B at equal loudness)

**Compressor topology choices:**
- **VCA** (e.g., SSL bus emulation): fast, neutral, "glue"
- **Opto** (LA-2A emulation): program-dependent, smooth, slow
- **FET** (1176 emulation): aggressive, fast, colored — rare in mastering
- **Vari-Mu** (Manley emulation): tube, slow, dense — favored for "warmth"
- **Feedback vs feed-forward detection:** feedback (slower, more musical) standard for mastering

**Multiband compression:** [22][23]

| Bands | Common Crossovers | Use Case |
|---|---|---|
| 2 | 120–200 Hz | Tame kick/bass vs everything else |
| 3 | 150 Hz, 5 kHz | Lows / mids / highs independent |
| 4 | 80, 250, 5 kHz | Surgical only |
| 5+ | varies | Almost never in mastering |

- Crossover slope: **18 dB/oct minimum** (24 dB/oct preferred for isolation)
- Per-band ratio: **1.25:1 to 1.5:1** (lighter than broadband because cumulative)
- Per-band attack: **5–10 ms minimum** to preserve transients
- Per-band release: **50–150 ms**
- Avoid splitting fundamental frequencies of lead vocal/main hook into different bands

### 6. Limiter — Algorithm Internals

**Brickwall limiter = compressor with ∞:1 ratio + ultra-fast attack + lookahead.** [8][9]

**Algorithm structure:**
1. **Lookahead buffer:** delay input by L samples (typical L = 44–220 samples at 44.1 kHz = 1–5 ms)
2. **Oversampling:** upsample 4–16× via polyphase FIR (anti-imaging filter)
3. **Peak detection** on oversampled signal: find max(|x|) in upcoming window
4. **Gain envelope computation:** if peak > ceiling, compute required attenuation; smooth via attack (instantaneous) / release (10–1000 ms, program-dependent)
5. **Apply** envelope to delayed (un-oversampled) signal
6. **Downsample** if oversampling was internal-only

**Key parameters:**
- **Ceiling:** −1.0 dBTP (universal default); −1.5 to −2 dBTP for material destined for AAC/Opus
- **Threshold:** sets effective gain reduction; "loudness" parameter on most limiters
- **Release:** 10–50 ms = aggressive/pumping; 200–1000 ms = transparent; "auto" = program-dependent
- **Lookahead:** 1–5 ms; longer = more transparent but more latency
- **True-peak mode (ISP):** enable always for final master
- **Style/character:** "transparent" (linear), "aggressive" (saturation-coupled), "clipper" (no release, hard clip at ceiling) — many modern limiters offer a "soft clipper" stage *before* the limiter for additional 1–3 dB loudness without further reduction

**Reference implementations to study:** FabFilter Pro-L 2 (4-style algorithm + 32x ISP), Pro-Q 4, Weiss MM-1, iZotope Maximizer (IRC modes), Sonnox Oxford Limiter v3.

### 7. Dithering & Noise Shaping

**The rule:** Dither **once, at the final bit-depth reduction**, regardless of intermediate float precision. [10][11]

**Dither types:**

| Type | PDF | Amplitude | Spectral character | Notes |
|---|---|---|---|---|
| **RPDF** | Uniform | 1 LSB peak-to-peak | White | Modulates noise floor (audible) — avoid for final |
| **TPDF** | Triangular | 2 LSB peak-to-peak | White, 4.77 dB louder than RPDF | **Industry default** — eliminates noise modulation |
| **Gaussian** | Normal | σ = 1 LSB | White | Analog-like; slightly louder noise floor |
| **High-pass TPDF** | Triangular, shaped | varies | Reduced LF noise | For 24→16 with shaping |

**Noise shaping** = feedback filter that pushes quantization-error spectrum out of the 1–5 kHz peak-sensitivity region into 15–20 kHz where the threshold of hearing is +60 dB SPL. Common implementations:

- **Lipshitz-Vanderkooy filters** (1991, AES Journal): 9-tap minimum-phase FIR, ~−18 dB perceived noise reduction
- **Sony Super Bit Mapping (SBM):** proprietary, used on 1990s–2000s CD masters
- **Apogee UV22 / UV22HR:** proprietary "high frequency placement"
- **POW-r (Pro Tools, others):** three modes — POW-r 1 (TPDF, no shape), POW-r 2 (mild shape), POW-r 3 (aggressive psychoacoustic shape)
- **iZotope MBIT+:** variable-order noise shaping
- **Wannamaker / Gerzon F-weighted, E-weighted:** psychoacoustic-curve-based

**When to use what:**
- **24-bit delivery (most streaming):** **No dither needed** — 24-bit noise floor (−144 dBFS) is below any analog noise
- **16-bit delivery (CD, legacy):** **TPDF + noise shaping** mandatory
- **32-bit float intermediate:** never dither — float quantization is non-uniform and dither makes no sense

### 8. Sample Rate, Bit Depth, and SRC

**Recommended internal precision:** 32-bit float (or 64-bit float) throughout processing chain. Quantize only at output. [24]

**Sample rate strategy:**

| Target | Recommended Master SR | Notes |
|---|---|---|
| CD (Red Book) | 44.1 kHz | Final SRC to 44.1 with high-quality algorithm |
| Streaming (most) | 44.1 kHz | Apple Digital Masters accepts up to 192 kHz |
| Apple Digital Masters | 24-bit, **≥44.1 kHz** (96 kHz preferred) | Required by spec |
| Broadcast (TV) | 48 kHz | Video sync; EBU R128 compliance |
| Vinyl pre-master | 88.2 / 96 kHz | Headroom for cutting EQ |
| Hi-res download | 96 / 192 kHz | Marketing tier |

**SRC algorithms (quality ranking):**
1. **SoX VHQ (soxr) / iZotope SRC** — polyphase, 200+ tap FIR, near-perfect impulse response
2. **r8brain free / SSRC** — high-quality open implementations
3. **Audacity / FFmpeg default** — adequate but audible artifacts on critical material
4. **Linear interpolation / nearest** — never use for mastering

**Aliasing prevention:** SRC must include anti-aliasing FIR with stopband attenuation ≥−100 dB; passband ripple ≤±0.01 dB; transition band 0.9·fs/2 to fs/2 [24].

### 9. Stereo Imaging & M/S Processing

[20][25]

**Mid-Side decomposition:**
```
M = (L + R) / √2   (or /2 for unity center; convention varies)
S = (L − R) / √2
```
Reconstruction:
```
L = (M + S) / √2
R = (M − S) / √2
```

**Mastering applications:**
- **M-channel compression:** glue center elements (vocal, kick, snare, bass) without pumping reverb tails
- **S-channel compression:** control reverb/ambience without affecting center punch
- **M-channel EQ:** clean up center muddiness, presence boost on vocals
- **S-channel EQ:** add air on sides (high shelf), remove muddy reverb (low cut)
- **Width control:** gain on S relative to M (>1.0 = widen, <1.0 = narrow, 0 = mono)

**Hard rules:**
- **Mono compatibility test mandatory.** Anything below 120 Hz should sum to mono cleanly (vinyl, club PA, phone speakers all sum)
- **Correlation meter ≥ 0** at all times; brief excursions to −0.5 acceptable on transient material; sustained negative correlation = phase problem
- **Goniometer/vectorscope:** should be roughly oval, slightly wider than tall; pure vertical = mono; pure horizontal = anti-phase

### 10. Saturation & Harmonic Enhancement

[7][26]

**Purpose:** Add even/odd harmonic content for perceived loudness, "warmth," "glue" without measurable gain reduction.

**Types:**
- **Tape saturation:** soft compression + high-frequency rolloff + low-frequency bump + 3rd harmonic + wow/flutter (Studer A800, Ampex ATR-102 emulations)
- **Tube saturation:** primarily even harmonics (2nd, 4th); soft clipping curve
- **Transformer saturation:** odd harmonics on low frequencies; "weight"
- **Console emulation:** crosstalk + subtle EQ + summing harmonics
- **Soft clip / wavefolding:** controllable harmonic generation

**Mastering use:**
- Subtle (≤1% THD measured at nominal level)
- Applied broadband or band-split (e.g., saturation only above 5 kHz for "presence")
- Typically before the limiter to add perceived loudness without further GR

### 11. Tonal Balance — Spectral Targets

[13][27]

**Pink noise reference:** −3 dB/octave (constant power per octave) is the historical "neutral" target. Modern productions often target a **4.5 dB/oct** slope (flatter mid response, brighter top).

**Genre-derived target curves** (from iZotope Tonal Balance Control, Mastering The Mix Reference, sonible, etc.) are corpus-extracted averages across:
- Pop / Modern
- Hip-hop / Trap
- Rock / Indie
- EDM / Dance
- Acoustic / Folk
- Orchestral / Cinematic
- Jazz / Vocal

For software, **train your tonal-target curves on at least 50–100 reference tracks per genre**, smoothed in 1/12-octave bands, after K-weighted normalization. This is what LANDR and eMastered do internally.

**Spectrum analyzer slopes:** Display can show flat (problematic — exaggerates lows), 3 dB/oct (classic), or 4.5 dB/oct (modern pop reference). Choice affects how the curve looks but not the underlying audio.

### 12. Reference Monitoring (For Quality Assurance)

[28]

**SPL calibration:**
- Cinema/large studio reference: **83 dB SPL C-weighted** at listening position with pink noise at −20 dBFS RMS per channel (Dolby spec)
- Small studio practical: **73–76 dB SPL C** (per Bob Katz, K-12 calibration)
- Calibration tool: SPL meter, C-weighted, slow response

**Bob Katz K-system:** [29]

| Scale | Reference (0 on meter = LUFS) | Headroom | Genre |
|---|---|---|---|
| **K-12** | −12 LUFS at 0 VU | 12 dB | Broadcast / heavily-compressed |
| **K-14** | −14 LUFS at 0 VU | 14 dB | Rock / Pop / mainstream |
| **K-20** | −20 LUFS at 0 VU | 20 dB | Classical / wide-dynamic |

Modern equivalent for mastering software: present LUFS-relative meters with target reference lines per genre.

### 13. Delivery Format Specifications

**Apple Digital Masters** (formerly Mastered for iTunes): [6]
- 24-bit minimum bit depth
- 44.1 kHz minimum; **96 kHz preferred**, up to 192 kHz accepted
- −1 dBTP true-peak maximum
- Verified with Apple's `afclip` tool (detects ISP > ceiling)
- Encoded by Apple to AAC-LC 256 kbps; preserves master headroom

**CD Red Book / DDP:** [30]
- 16-bit, 44.1 kHz, stereo PCM
- Up to 79:57 audio
- Delivery: **DDP 2.00 image** (folder with `IMAGE.DAT`, `PQDESCR`, `DDPID`, `DDPMS`, `CRC`)
- PQ subcodes: track starts, indices, gaps (default 2 sec), pre-emphasis flag (almost always off)
- ISRC codes per track (12 chars: CC-XXX-YY-NNNNN)
- UPC/EAN barcode for disc
- CD-TEXT (optional): album title, artist, track titles
- Tools: HOFA CD-Burn.DDP.Master, Sonoris DDP Player, WaveLab, SADiE

**Broadcast Wave (BWF):** [31]
- WAV extension; adds `<bext>` chunk with origination data, timecode, UMID
- `<iXML>` chunk for production metadata
- `<axml>` chunk for EBU Core XML (EBU Tech 3352 spec — ISRC embedding)
- Format used for: broadcast delivery, archival, post-production interchange

**Vinyl pre-master:** [20]
- Sample rate: 96 kHz / 24-bit (some cutters prefer 88.2)
- **Bass to mono below 200 Hz** (mandatory; eccentric/vertical groove modulation)
- True peak ≤ −1 dBTP, but **dynamic range higher** than streaming (LRA ≥ 8 LU typical)
- Avoid heavy limiting (cutting engineer needs headroom)
- Side length: ≤18–20 min for 12″ 33⅓ rpm; longer = quieter cut
- No de-essing required by lacquer (sibilance is fine), but extreme HF (>16 kHz) at high level can damage cutter heads
- Pre-emphasis: usually off (RIAA EQ is applied during cut/play, not encoded in source)

**MFiT / Apple's quality bar specifically:**
- Avoid clipping (any clip = automatic fail)
- Avoid inter-sample peaks above ceiling
- Use 24-bit float intermediate; deliver 24-bit WAV
- Embed ISRC, ISWC, composer credits in metadata

### 14. AI / Algorithmic Mastering Architectures

[14] What LANDR, eMastered, CloudBounce, Bandlab Mastering, AI Mastering, Cryo Mix actually do internally (inferred from product behavior + papers):

**Stage 1 — Analysis:**
- Spectral analysis (FFT, 1/12 or 1/24 octave bands, smoothed)
- LUFS-I, LUFS-S, LUFS-M, LRA
- Crest factor, peak-to-RMS ratio
- Stereo correlation per frequency band
- Transient density (onset detection)
- Tempo, key (sometimes)

**Stage 2 — Classification:**
- Genre classification (CNN on mel-spectrogram or hand-coded features)
- Sometimes: production style, era, "vibe"

**Stage 3 — Target selection:**
- Pull genre-appropriate target curve (spectral)
- Pull target LUFS (typically −14)
- Pull target dynamics (LRA)

**Stage 4 — Chain parameterization:**
- Map analysis vs target to plugin parameters (this is where ML provides value)
- EQ delta = target curve − measured curve (smoothed, dynamic-range-limited)
- Compression ratio derived from current vs target LRA
- Limiter threshold derived from target LUFS

**Stage 5 — Processing:**
- Standard DSP chain (EQ → comp → multi-band → limiter → dither)
- All conventional algorithms — the AI is in the *parameter selection*, not the processing

**Reference matching** (LANDR Pro, eMastered): user provides reference track; system computes spectral and dynamic delta between input and reference and biases the parameter selection toward matching. This is **not** style transfer (no neural waveform manipulation); it's targeted DSP toward a specific target curve.

**Implication for software:** the AI/ML component is bounded and tractable. The hard engineering work is DSP correctness (LUFS metering, true-peak limiting, SRC, dither). The "intelligence" layer can be a relatively simple set of mappings calibrated against a corpus.

### 15. Stem Mastering Workflow

[32] If the program supports stem input (highly recommended for differentiation):

- **Definition:** 4–8 sub-mixes (e.g., drums, bass, vocals, instruments, FX) instead of single stereo
- **Process:**
  1. Per-stem corrective EQ
  2. Per-stem compression
  3. Per-stem M/S processing
  4. Sum to stereo
  5. Bus EQ
  6. Bus compression
  7. Saturation / limiting / dither (as in stereo mastering)
- **Advantages:** problem isolation, surgical fixes, mix-rescue capability
- **Disadvantages:** more compute, more parameters, requires producer to deliver clean stems with no master bus processing

---

## Areas of Consensus

- **−14 LUFS / −1 dBTP** as universal streaming target (no need for per-platform masters unless going to broadcast or Apple-only)
- **Dither once, at output, TPDF minimum** — never multi-dither in chain
- **24-bit floating-point internal precision** throughout processing
- **True-peak limiting with ≥4× oversampling** mandatory for any modern master
- **Lookahead 1–5 ms** in brickwall limiters
- **Mid-side processing is now standard** (was specialist 20 years ago)
- **Mono compatibility below 120 Hz is non-negotiable**
- **Linear-phase EQ for broad tonal moves, minimum-phase for surgical/transient work**
- **No reverb, delay, or creative FX in mastering** — those are mix decisions

## Areas of Debate

- **Loudness target appropriateness:** Whether to deliver −14 LUFS "for streaming" vs. genre-appropriate dynamics (e.g., −10 LUFS for EDM, −18 LUFS for indie folk). Bob Katz and others argue against any artificial loudness inflation; many practitioners disagree on "competitive" loudness for specific genres.
- **Linear phase vs minimum phase:** No consensus. Bob Katz, Bob Ludwig, and many top engineers prefer minimum phase for "feel"; others (Mike Wells, Justin Perkins) routinely use linear phase. **Boost vs cut behavior of linear phase is genuinely controversial** (pre-ringing on boosts vs none on cuts).
- **Multiband compression frequency:** Some engineers use it on every master; others (Greg Calbi, Bob Ludwig) almost never. Genre-dependent.
- **Saturation/harmonic excitement:** "Glue" vs "coloration." Romantic preference rather than measurable consensus.
- **Sample rate for delivery:** 44.1 vs 48 vs 96. Some engineers argue 44.1 has audible artifacts from anti-imaging filters; others (and most blind tests) show no audible difference at modern SRC quality.
- **Clipper before limiter:** Aggressive practice (popular in EDM, hip-hop) — adds 1–3 dB perceived loudness via soft clipping before final brickwall. Controversial in classical/jazz mastering circles.
- **AI mastering quality:** LANDR/eMastered/CloudBounce comparative quality is hotly contested, with results highly genre- and source-dependent. Pro engineers generally view all current AI mastering as inferior to skilled human mastering on critical material; suitable for demos, lo-fi, casual releases.

## Sources

### Tier 1 — Authoritative Standards (Gold)
- [[1] Recommendation ITU-R BS.1770-5 (11/2023)](https://www.itu.int/dms_pubrec/itu-r/rec/bs/R-REC-BS.1770-5-202311-I!!PDF-E.pdf) — *primary loudness measurement specification*
- [[2] Recommendation ITU-R BS.1770-4 (10/2015)](https://assets.corusent.com/wp-content/uploads/2021/10/ITUR_BS_1770_4_Audio_Program_Loudness_En.pdf) — *widely-deployed prior revision, algorithm identical at the measurement level*
- [EBU R 128 — Loudness normalisation and permitted maximum level of audio signals](https://tech.ebu.ch/publications/r128) — *broadcast delivery standard*
- [EBU Tech 3341 / 3342 / 3343 / 3344](https://tech.ebu.ch/loudness/) — *EBU mode metering, LRA spec, production and distribution guidelines*
- [[31] Embedding Metadata in Broadcast WAVE Files — FADGI/Library of Congress](https://www.digitizationguidelines.gov/audio-visual/documents/Embed_Intro_090915.pdf) — *BWF metadata specification*

### Tier 2 — Manufacturer / Platform Specifications (Gold)
- [[6] Apple Digital Masters: Music as the Artist and Sound Engineer Intended (PDF)](https://www.apple.com/apple-music/apple-digital-masters/docs/apple-digital-masters.pdf)
- [Delivering Apple Digital Masters — iTunes Partner Support](https://itunespartner.apple.com/music/support/5217-delivering-apple-digital-masters)
- [[17] Loudness normalization on Spotify](https://support.spotify.com/us/artists/article/loudness-normalization/)
- [[4] FabFilter Pro-L 2 — True peak limiting documentation](https://www.fabfilter.com/help/pro-l/using/truepeaklimiting)
- [Netflix Partner — Loudness and True Peaks](https://partnerhelp.netflixstudios.com/hc/en-us/articles/360050414014-Loudness-and-True-Peaks-How-to-Measure-and-When-to-Flag)

### Tier 2 — Practitioner Authorities (Silver-Gold)
- [[29] Bob Katz — Mastering Audio (Sound on Sound review)](https://www.soundonsound.com/reviews/bob-katz-mastering-audio) — *the industry textbook; original K-system paper in JAES 2000*
- [Bob Katz Mastering Secrets — Tape Op #116](https://tapeop.com/interviews/116/bob-katz-bonus)
- [Bob Katz Level Practices Part 2 — digido.com](https://www.digido.com/portfolio-item/level-practices-part-2/)
- [[19] FabFilter Learn — Linear phase EQ](https://www.fabfilter.com/learn/equalization/linear-phase-eq) — *vendor-neutral filter design background*

### Tier 2 — Industry Education / Tools (Silver)
- [[7] iZotope — What is an ideal mastering signal chain?](https://www.izotope.com/en/learn/what-is-an-ideal-mastering-signal-chain)
- [iZotope — How to master for streaming platforms: LUFS guide](https://www.izotope.com/en/learn/mastering-for-streaming-platforms.html)
- [[22] iZotope — Multiband Compression Basics](https://www.izotope.com/en/learn/multiband-compression-basics-izotope-mastering-tips.html)
- [iZotope — Tonal Balance Control](https://www.izotope.com/en/learn/leveling-up-your-mastering-workflow-with-tonal-balance-control.html)
- [[20] Furnace Record Pressing — Vinyl 101: How to Prepare Your Audio for Vinyl](https://www.furnacemfg.com/vinyl-record-audio-preparation/)
- [Q. Should I master my material for vinyl? — Sound on Sound](https://www.soundonsound.com/sound-advice/q-should-master-my-material-vinyl)
- [Mastering for Vinyl — Chroma Mastering](https://www.chromamastering.com/mastering-for-vinyl-what-you-need-to-know/)
- [[30] DDP Image creation — Disc Description Protocol on Wikipedia](https://en.wikipedia.org/wiki/Disc_Description_Protocol)

### Tier 3 — Technical Reference (Silver)
- [[12] ITU 1770 RLB filter coefficients — music-dsp mailing list](https://music-dsp.music.columbia.narkive.com/Kl2BqIBk/itu-1770-rlb-filter-coefficients-and-biquad-iir-filter)
- [Cookbook formulae for audio EQ biquad filter coefficients](https://webaudio.github.io/Audio-EQ-Cookbook/audio-eq-cookbook.html) — *Robert Bristow-Johnson; standard reference*
- [[10] Dither — Wikipedia](https://en.wikipedia.org/wiki/Dither)
- [[11] Sonically Optimized Noise Shaping Techniques (Lukin) PDF](http://audio.rightmark.org/lukin/dither/dither.pdf)
- [Loudness Range (LRA) — Design and Evaluation (research paper)](https://www.researchgate.net/publication/321627735_Loudness_Range_LRA_-_Design_and_Evaluation)
- [[16] LRA values per genre — research paper context](https://www.researchgate.net/publication/321627735_Loudness_Range_LRA_-_Design_and_Evaluation)

### Tier 3 — Streaming Platform Comparative (Silver-Bronze)
- [[3] LUFS Targets for Every Streaming Platform in 2026 — UpTrack](https://uptrack.pro/blog/lufs-targets-for-every-streaming-platform)
- [[15] Loudness Targets for Streaming — Mat Leffler-Schulman Mastering](https://matlefflerschulman.com/mastering-articles/loudness-targets-and-mastering-for-streaming-platforms)
- [Mastering for Streaming — Sage Audio](https://www.sageaudio.com/articles/mastering-for-streaming-platform-loudness-and-normalization-explained)

### Tier 3 — Practitioner / How-To (Bronze, useful for technique)
- [[5] True Peak vs Inter-Sample Peaks — Mat Leffler-Schulman](https://matlefflerschulman.com/mastering-articles/true-peak-vs-inter-sample-peaks)
- [[8] What Is a True Peak Limiter? — iZotope](https://www.izotope.com/en/learn/true-peak-limiter)
- [[9] An introduction to limiters — iZotope](https://www.izotope.com/en/learn/an-introduction-to-limiters-and-how-to-use-them.html)
- [[18] What is mid/side processing? — iZotope](https://www.izotope.com/en/learn/what-is-midside-processing.html)
- [[21] Mastering Chain Order — Audiospectra](https://audiospectra.net/mastering-chain-order/)
- [[23] Multiband Compressor in Mastering — MasteringBox](https://www.masteringbox.com/learn/multiband-compression)
- [[24] Sample Rate Conversion — PS Audio](https://www.psaudio.com/blogs/copper/sample-rate-conversion)
- [[25] Mid/Side Processing — MasteringBox](https://www.masteringbox.com/learn/mid-side-processing)
- [[26] 3 Uses for Parallel Compression in Audio Mastering — iZotope](https://www.izotope.com/en/learn/3-uses-for-parallel-compression-in-audio-mastering.html)
- [[27] What is Tonal Balance? — sonible](https://www.sonible.com/blog/tonal-balance/)
- [[28] Calibrated Monitoring — Sonarworks](https://www.sonarworks.com/blog/learn/produce-consistent-mixes-with-calibrated-monitoring)
- [[32] Stereo vs Stem Mastering — Abbey Road](https://www.abbeyroad.com/news/stereo-mastering-vs-stem-mastering-whats-the-difference-2991)

### Tier 3 — AI Mastering Architecture Reference (Bronze)
- [[14] AI Mastering Comparison — GitHub (ai-mastering/mastering_comparison)](https://github.com/ai-mastering/mastering_comparison)
- [Automated Mastering Comparison: LANDR vs eMastered vs Aria](https://mastering.com/automated-mastering/)

---

## Gaps and Further Research

For this specific software project, the **highest-value items not fully exhausted in this pass**:

1. **ITU-R BS.1770-5 PDF full text** — get the official spec, not the Wikipedia summary. The 2023 revision tightens channel-weighting language and clarifies stereo vs surround handling.
2. **EBU Tech 3341, 3342, 3343, 3344** — these are the *implementation* documents. 3341 specifies meter behavior (refresh rate, scale, color zones), 3342 specifies LRA computation, 3343 is production guidelines, 3344 is distribution.
3. **AES standards** — particularly **AES17** (digital audio measurement procedure), **AES31** (audio file interchange), **AES77-2025** (recently published, immersive audio measurement). Accessing AES documents requires AES membership.
4. **libebur128 source code** ([GitHub: jiixyj/libebur128](https://github.com/jiixyj/libebur128)) — reference C implementation of BS.1770; you can use this directly or port it. License: MIT.
5. **pyloudnorm source** — Python implementation, easier to read for prototyping.
6. **r8brain-free-src** — high-quality SRC C++ library, public domain.
7. **JUCE audio framework** (if building plugins) — handles host integration, plugin formats (VST3, AU, AAX), provides DSP primitives.
8. **Codec-aware mastering research:** EBU Tech 3343 and AES papers on "Loudness Penalty" (how streaming codec + normalization interact). Spotify's own engineering blog has occasional posts.
9. **Psychoacoustic models for noise-shaped dither** — Wannamaker (1991), Gerzon (1992) AES papers; F-weighted and E-weighted curves are derivations of the Fletcher-Munson and ISO 226 equal-loudness contours.
10. **Listening tests / blind comparisons:** Audio Engineering Society Journal has many papers on SRC audibility, limiter transparency, loudness normalization perception. AES E-library access ($20/article or institutional subscription) is the gold standard for primary research.
11. **Current Apple Digital Masters Provider Tools** — Apple distributes `afclip`, `afconvert`, and the AURoundTripAAC AudioUnit, all free, all run on macOS. These let you verify exactly what Apple's encoder does to your master.
12. **CD-Text and ISRC encoding specifics** — IFPI ISRC handbook; EBU Tech 3352 for embedding in BWF.

---

## What to Prioritize for the Mastering Program — MVP Build Order

Given everything above, an honest "MVP build order" for a credible mastering program:

1. **BS.1770-5 LUFS meter** (M, S, I, LRA). Use libebur128 or implement from spec. **This is the single most important DSP block.** Without it, you cannot deliver a master to any platform's spec.
2. **True-peak meter** with 4× oversampling. Same module as #1 essentially.
3. **High-quality SRC** (r8brain free or soxr). Required at output.
4. **TPDF dither + noise shaping** (POW-r-class). Required at 16-bit output.
5. **Brickwall limiter with lookahead and true-peak detection.** Default ceiling −1 dBTP, target −14 LUFS.
6. **Parametric EQ** (biquad cookbook), both minimum-phase and linear-phase variants.
7. **Broadband compressor** with selectable detection (peak/RMS), VCA/opto curves.
8. **M/S encode/decode** routing.
9. **Spectrum analyzer + correlation meter** for QA.
10. **Multiband compressor** (Linkwitz-Riley or FIR crossovers, 18–24 dB/oct, 2/3/4 bands).
11. **Saturation** (tube/tape/transformer emulations — table-lookup waveshaping is fine to start).
12. **Reference-track analysis** (LUFS, LRA, spectrum) for A/B and auto-target.
13. **Genre-specific tonal targets** (corpus-trained, smoothed).
14. **Metadata embedding** (BWF/iXML/ID3) — required for Apple Digital Masters.
15. **DDP export** — only if targeting CD market.

---

## Strategic Insights for This Project

- **Inter-sample peak / lossy codec interaction is the single most undervalued technical detail.** Every streaming platform applies LUFS normalization *and then* encodes to AAC/Opus, in that order. A master that measures clean at 0 dBFS sample-peak will routinely measure +1 to +2 dBTP after AAC reconstruction, audibly distorting on every listener's device. Building a limiter that targets −1 dBTP using **8× or 16× oversampling** (not the 4× minimum) is the single highest-leverage quality choice you can make. Cheap mastering software gets this wrong; expensive software gets it right; that gap is what users hear when they say "the master sounds more pro."

- **The canonical signal chain is a feature, not a constraint.** Because the order has been stable for 20+ years, you can build a UI that maps directly to it (slots labeled "EQ 1 — corrective," "Compressor," "Saturation," "EQ 2 — tonal," "Stereo," "Limiter") and it will feel familiar to every working engineer who tries the software. Reinventing the order to be "smarter" is a known failure mode of mastering software (looking at early Ozone versions which auto-rearranged modules). Stay canonical; let users reorder if they want.

- **The K-weighting filter is the one mandatory DSP block.** If you implement nothing else from the BS.1770 spec, implement the two-biquad cascade with proper coefficient derivation at the working sample rate. Every loudness measurement, every platform delivery target, every metering display in the software depends on it being bit-exact. Test against libebur128's reference traces (the project ships test vectors).

- **For the AI/ML layer, the leverage point is parameter selection, not waveform processing.** Don't try to "neural network" the audio. Build excellent DSP, then train a small model on (analysis-features → DSP-parameters) pairs from a corpus of well-mastered tracks. This is the architecture LANDR / eMastered actually use, and it's the architecture that gives you transparent, debuggable, controllable results rather than a black-box guessing machine.

- **The most realistic competitive positioning** is not "beat the human" — it's "give the home producer a Spotify-compliant, codec-safe, technically clean master in 30 seconds, with a UI that teaches them what's happening." LANDR's market is huge and growing precisely because most uploads to streaming services come from people who do not have access to professional mastering. The technical bar for "Spotify-compliant" is *concrete and finite* (the table in §2), which is a gift for a software product.

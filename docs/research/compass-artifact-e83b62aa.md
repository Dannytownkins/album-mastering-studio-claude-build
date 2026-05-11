# Modern Audio Mastering — Technical Reference for an Automated, Streaming-First Mastering Application (2025–2026)

## TL;DR
- **Master one streaming-first deliverable per track and adapt format-by-format from it:** target an integrated loudness in the **−10 to −14 LUFS** range depending on genre, with a hard **−1 dBTP** true-peak ceiling (−2 dBTP for very loud/Amazon, and a −1 dBTP ceiling on the Atmos branch at −18 LUFS); deliver 24-bit/48 kHz (or native) WAV. Convergence around BS.1770/EBU R128 means a single master generally covers Spotify (−14), YouTube video (−14), Tidal (−14), Amazon (−14, −2 dBTP), Apple Music (−16 Sound Check), Deezer (−15) without rendering separate versions.
- **Codify the canonical mastering chain as a deterministic DSP graph:** input gain → corrective minimum-phase EQ (HPF, surgical cuts) → optional dynamic EQ / de-ess → broadband bus compression (1.2:1–2:1, 1–3 dB GR, slow attack, auto release) → multiband compression *only when needed* (2–4 bands, crossovers ~120 Hz / 800 Hz / 5 kHz) → harmonic enhancement (tape/tube/transformer, ≤2% added THD) → tonal/musical (mid/side) EQ → stereo imaging (mono-sum bass below ~120–200 Hz, side high-shelf for air) → soft-clip stage (optional, 0.5–1.5 dB) → true-peak brickwall limiter (FabFilter Pro-L 2 / Newfangled Elevate / Ozone Maximizer class; 4× oversampling, 1–5 ms lookahead, ITU-R BS.1770-4 true peak, ceiling −1 dBTP) → dither + noise-shaping **only** at final bit-depth reduction (TPDF for safety, MBIT+/POW-r/UV22HR if encoding straight to 16-bit).
- **Make the system reference-aware and format-adaptive:** use K-weighted loudness gating (ITU-R BS.1770-4: K-weighting + 400 ms 75 %-overlap gating blocks, −70 LUFS absolute and −10 LU relative thresholds) for measurement, then branch at the end to render (a) streaming master @ −10 to −14 LUFS / −1 dBTP, (b) Apple Digital Master @ ≤−1 dBTP 24-bit verified with `afclip`/`AURoundTripAAC`, (c) CD master @ 16-bit/44.1 kHz with TPDF dither, (d) vinyl pre-master with bass mono-summed <200 Hz, sibilance tamed, no brickwall, and (e) Atmos ADM BWF @ −18 LUFS / −1 dBTP. Modern academic foundation: differentiable-DSP / black-box DAFX gradient approximation (Martínez Ramírez et al., ICASSP 2021; Steinmetz et al., 2022).

---

## Key Findings

1. **The loudness war is functionally over on streaming.** Every major service except SoundCloud applies BS.1770/EBU-style normalization on playback; what differs is the target (−14 LUFS for Spotify/YouTube video/Tidal/Amazon, −16 LUFS for Apple Music, −15 LUFS for Deezer), whether quiet tracks are boosted, and how lossy codecs interact with true peaks.
2. **True-peak limiting is non-negotiable.** BS.1770-4 defines true peak as the upsampled (default 4× via a 48-tap polyphase FIR, 12 taps per phase) signal peak; consumer DACs and AAC/Ogg-Vorbis transcodes add ≥0.5 dB of intersample overshoot. −1 dBTP is the universal rule; −2 dBTP for Amazon and for any master integrated louder than −14 LUFS.
3. **The canonical mastering chain order is stable** across iZotope, Sound on Sound, Mastering The Mix, Sage Audio, Sonarworks, Audiospectra, and Beat Kitchen — encode it as default and add knobs for genre-specific reordering.
4. **Compression in mastering is light:** modern published practice converges on 1.2:1–2:1 ratios, 20–100 ms attack, 100–300 ms/auto release, and **1–3 dB of gain reduction** on the broadband bus. Multiband is a problem-solver, not a default.
5. **Limiter design is the single most consequential algorithm.** Look-ahead (1–5 ms), oversampled true-peak detection (4×–32×), program-dependent release, and a soft-clip pre-stage define how loud you can go before transient damage is audible. FabFilter Pro-L 2, iZotope Maximizer IRC 5, Sonnox Oxford, Newfangled Elevate, and Waves L2/L3 are the reference designs.
6. **AI mastering products are spectral-balance classifiers + preset selectors driving conventional DSP.** Where published, none are end-to-end neural networks producing waveforms. The relevant academic frontier (Martínez Ramírez / Reiss / Adobe / Sony) is differentiable signal processing or black-box gradient approximation (SPSA) driving conventional DSP.
7. **Format-adaptive delivery is mostly post-chain branching.** A single 24-bit/48 kHz mastered render fans out to Apple Digital Masters, CD, vinyl, and Atmos. The genuine exceptions worth the engineering effort are vinyl and Atmos.
8. **Djent/metal is genre-specifically loud and low-end-disciplined.** Nolly Getgood, Andy Sneap, Jens Bogren, Forrester Savell — and Periphery/Meshuggah/Gojira references — sit hotter than the streaming target by design, leaning on parallel-distorted bass, low-mid carving 200–500 Hz, mono-summed sub-100 Hz, 2–4 kHz bite, and aggressive transient/de-ess on cymbals.

---

## Details

### 1. Streaming-platform loudness/delivery specifications (2025–2026)

| Platform | Integrated target | True-peak ceiling | Normalization behavior | Codec(s) |
|---|---|---|---|---|
| **Spotify** | −14 LUFS (default "Normal"); user-selectable "Loud" −11, "Quiet" −23 | −1 dBTP (−2 if master >−14 LUFS) | Both directions; album-aware on album playback, track-mode on shuffle/playlist | Free: AAC 128 kbit/s (web); Premium up to ~320 kbit/s Ogg Vorbis on desktop/mobile; AAC 256 kbit/s on web; FLAC 16-bit/44.1 kHz lossless on Premium (rollout 2025) |
| **Apple Music** (Sound Check) | −16 LUFS | −1 dBTP (Apple Digital Masters) | **Turns DOWN only**; default-on for new iOS/macOS | AAC 256 kbit/s, ALAC lossless up to 24-bit/192 kHz; Dolby Atmos |
| **Tidal** | −14 LUFS | −1 dBTP | Negative-gain (album-aware) | AAC, FLAC, MQA legacy (now end-of-life), FLAC HiRes |
| **YouTube** (video) | −14 LUFS | −1 dBTP | Negative gain only | AAC, Opus |
| **YouTube Music** | Per Ian Shepherd's *Production Advice* article on YouTube Music, the app attenuates only tracks above roughly **−7 LUFS** — substantially more permissive than the video platform | −1 dBTP | Attenuation-only | AAC, Opus |
| **Amazon Music** | −14 LUFS (industry-cited; Amazon does not publish an official figure) | −2 dBTP | Negative gain only | AAC, FLAC up to 24-bit (HD/Ultra HD) |
| **SoundCloud** | No loudness normalization | n/a | None | Opus 64 kbit/s (free), AAC 256 kbit/s (Go+) |
| **Deezer** | −15 LUFS | −1 dBTP | Negative gain | MP3, FLAC |
| **Pandora** | Not LUFS-based (ReplayGain-style proprietary) | −1 dBTP | Both directions | AAC |
| **Apple Music Dolby Atmos** | **−18 LKFS (LUFS)** per ITU-R BS.1770-4 | **−1 dBTP** per BS.1770-4 | per-track integrated check | BWF ADM, 24-bit LPCM @ 48 kHz |

Authoritative spec underlying these is **AES TD1008 (2021)**, upgraded to **AES77 (2023)** — *Recommendations for Loudness of Internet Audio Streaming and On-Demand Distribution*: music target −16 LUFS integrated (track normalization) or −14 LUFS for the loudest album track (album normalization); speech −18 LUFS; max true peak −1 dBTP at codec input. Apple's −16 LUFS aligns with TD1008; Spotify's −14 LUFS aligns with the album-loudest-track rule.

**Engineering implications:**
- A master at −8 LUFS on Spotify "Normal" plays at the same level as a −14 LUFS master, only with 6 dB less dynamics. There is zero playback-level benefit beyond the platform target unless you target Spotify "Loud" mode (a single-digit-percent of listeners).
- Apple's "down-only" rule means very quiet masters (<−18 LUFS) sound weak everywhere; aim ≥−16 LUFS unless dynamics are the artistic point.
- Per Spotify's artist support article "Loudness normalization on Spotify": *"The limiter's set to engage at -1 dB (sample values), with a 5 ms attack time and a 100 ms decay time."* This in-line limiter runs on tracks Spotify has to boost to hit "Loud" mode — don't rely on it favorably.
- Lossy codecs add 0.3–1.5 dB of intersample peak post-transcode in practice; −2 dBTP for loud masters is empirically validated headroom.

**Codec notes:**
- AAC-LC 256 kbit/s (Apple): MDCT-based; can spike intersample peaks on cymbals/sibilance.
- Ogg Vorbis (Spotify desktop/mobile): MDCT + floor/residual; sensitive to high-frequency density and brickwall artifacts.
- Opus (YouTube/SoundCloud free): hybrid CELT/SILK; transparent at 128 kbit/s but harsh transients can pump.
- MP3 (Deezer Free, legacy): subband + MDCT; intersample peaks routinely 1–3 dB above sample peaks.
- Apple's `AURoundTripAAC` AU plugin and `afclip` CLI (both shipped in macOS) verify post-encode peaks; oversample ≥4× and check reconstruction overshoot before publishing.

### 2. Loudness standards and measurement

**ITU-R BS.1770-4** (and BS.1770-5, 2023, adding advanced/immersive channel weighting) defines the algorithm every major platform uses:

1. **K-weighting filter** — two cascaded biquads applied per channel:
   - **Stage 1 (head shelving):** b0 = 1.53512485958697, b1 = −2.69169618940638, b2 = 1.19839281085285; a1 = −1.69065929318241, a2 = 0.73248077421585 (at 48 kHz; non-48 kHz needs new coefficients matching the same response).
   - **Stage 2 (RLB HPF ~60 Hz):** b0 = 1, b1 = −2, b2 = 1; a1 = −1.99004745483398, a2 = 0.99007225036621.
2. **Channel weighting:** L, R, C weighted 1.0; Ls, Rs weighted 1.41 (≈+1.5 dB); LFE excluded. BS.1770-5 extends weights for 7.1.4/Atmos using azimuth + elevation tables.
3. **Gating:** absolute gate at −70 LUFS rejects silence; relative gate at −10 LU below the absolutely-gated loudness rejects quiet sections; measurement uses 400 ms blocks with 75 % overlap (100 ms hop); incomplete trailing blocks discarded.
4. **Loudness output:** L = −0.691 + 10 log10(Σᵢ Gᵢ · zᵢ). LKFS = LUFS; 1 LU = 1 dB.
5. **EBU R128** adds Short-term (3 s sliding, ungated), Momentary (400 ms sliding, ungated), and LRA (95th – 10th percentile of short-term LUFS over the gated program). Mastering rule of thumb: LRA 6–14 LU for produced music, 2 LU = squashed, 15+ LU = classical/jazz.
6. **ATSC A/85** (US broadcast/CALM) targets −24 LKFS ±2; **EBU R128** broadcast target is −23 LUFS ±1; **Netflix** ingest is −27 LKFS ±2 with dialog gating.

**True peak (dBTP):** BS.1770 specifies upsampling ≥4× via a 48-tap polyphase FIR (12 taps × 4 phases) and taking absolute maximum. Implementations vary; FabFilter and iZotope ship higher-order Kaiser-windowed FIR with 8×–32× linear-phase oversampling. **FabFilter Pro-L 2** documents that 4× oversampling + 0.1 ms minimum lookahead keeps intersample overshoot within ±0.1 dB. Cheap implementations (parabolic, 2× linear) undershoot by up to 1.5 dB — dangerous for codec survival. Detector chain: x → 4× FIR upsample → LPF at fs/2 → abs() → max → dBFS. Essentia's `TruePeakDetector` is an open-source BS.1770-conformant reference implementation.

**PLR / PSR** (peak-to-loudness / peak-to-short-term-loudness):
- PLR = peak − integrated LUFS (whole program).
- PSR = peak − short-term LUFS (smaller windows; "instantaneous dynamics").
- The MeterPlugs blog post *"Crest Factor, PSR and PLR"* (18 May 2017) records the working rule: *"Mastering engineer Ian Shepherd (co-creator of our Dynameter plugin) recommends going no lower than PSR 8 during the loudest parts of a song. This goes for music in any genre. Anything less than this will often sound crushed."*
- Sound on Sound's *Dynameter* review (Hugh Robjohns, August 2017) documents specific real-world examples: a heavily-limited modern remaster of *"'Samurai' by '80s metal band Grand Prix… With a PLR reading of 7 and a minimum PSR 5, it will sound quiet and feeble in a loudness-normalised regime"*; Supertramp's *"'Bloody Well Right'. With PLR 16 and minimum PSR 8 it works well in all loudness-normalised regimes"*; *"a very dynamic jazz track from Dave Brubeck with solo trumpet: PLR 20, minimum PSR 9."*
- SonaMetro's practical scale: **PLR 5 = squashed, PLR 12 = dynamic.** Translating to genre:
  - EDM / trap / modern metal: PLR 5–8
  - Pop / rock / hip-hop: PLR 8–11
  - Indie / acoustic / well-mastered rock: PLR 11–16
  - Classical / dynamic jazz: PLR 16–22

**LUFS vs RMS vs dBFS:** dBFS is sample amplitude only. RMS is closer to perceived loudness but unweighted and ungated; for typical music, −9 RMS ≈ −11 LUFS. Use LUFS for normalization decisions; RMS only for legacy K-system metering (Bob Katz K-12 broadcast, K-14 dance/rock, K-20 classical, with monitors calibrated to 83 dB SPL per speaker at 0 dB on the meter).

### 3. Mastering signal chain — DSP graph

Consensus across iZotope, Sound on Sound, Mastering The Mix, Sage Audio, Sonarworks, Audiospectra, and Beat Kitchen:

```
Input → [Gain Stage] → [Corrective EQ — min-phase, HPF + surgical cuts]
      → [Dynamic EQ / De-ess (optional)]
      → [Broadband Compressor] → [Multiband Compressor (conditional)]
      → [Saturation / Harmonic Enhancer] → [Tonal/Musical EQ (often M/S)]
      → [Stereo Imaging / M-S Width]
      → [Soft-clipper (optional, 0.5–1.5 dB)]
      → [True-Peak Brickwall Limiter]
      → [Dither + Noise Shaping → quantize to delivery bit depth]
      → Output (separate branches per delivery format)
```

**Rationale:**
- Corrective EQ before compression so the compressor responds to a tonally balanced signal.
- Saturation after compression so harmonics ride on already-controlled dynamics.
- Tonal EQ after saturation to taste the post-saturation timbre.
- Soft-clip just before the limiter shaves the worst peaks invisibly so the limiter does less work.
- Dither absolutely last; any processing after dither destroys it.

**Switchable templates:**
- **Aggressive/EDM/metal** — soft-clip on by default with 1.5–2 dB drive; multiband on with 4 bands.
- **Acoustic/classical** — no multiband, no soft clip; broadband comp ≤1 dB GR.
- **Loud hip-hop/trap** — soft-clip pre-limiter; limiter ceiling −1 dBTP with post-check.
- **Pultec-style EQ trick** — simultaneous low-shelf boost + cut at near-identical frequencies (60–100 Hz); resonant bump-then-dip; codify as a "Pultec Low" macro.

**Parallel routing** (encode as optional bus features):
- Parallel compression (NY-style) — heavily compressed copy mixed back at 10–30 % for metal glue.
- Parallel saturation — distorted copy summed back for grit without dirtying transients.
- M/S parallel — process mid and sides independently as the default M/S architecture inside EQ/comp modules.

### 4. EQ in mastering — specifics

**Filter topology:**
- **Minimum-phase IIR** — analog-modeled; frequency-dependent group delay (smearing around resonant filters) but no pre-ringing; preferred for general tone-shaping.
- **Linear-phase FIR** — flat group delay; introduces pre-ringing proportional to filter length and inversely proportional to frequency (more audible in low-end). FabFilter Pro-Q 3's "Low" mode introduces ~70 ms latency at 44.1 kHz. Pre-ringing audibility scales with Q (>5) and boost amount.
- **Natural-phase / mid-phase / dynamic-phase** (Pro-Q "Natural Phase"; iZotope mid-phase modes) — best of both for mastering.

**Surgical filter ranges** (subtractive moves preferred):
- 20–40 Hz: sub-sonic HPF, 12–24 dB/oct, pre-compression.
- 60–100 Hz: kick fundamental / bass body — Pultec bump+cut.
- 200–400 Hz: mud — 1–3 dB cut, Q 0.7–1.5.
- 400–800 Hz: boxy — narrow cut Q 1.5–3, 1–3 dB.
- 1–3 kHz: vocal presence; rarely boosted on bus, often dynamic-EQ'd.
- 2–5 kHz: harshness — 0.5–1.5 dB cut, Q 1–2.
- 5–10 kHz: sibilance — dynamic EQ or de-esser; static cuts are clumsy here.
- 10–20 kHz: air shelf — 0.5–2 dB shelf boost on sides (M/S).

**Mid/Side EQ patterns** (Mastering The Mix, Sonible, Production Expert):
- Side high-shelf +1 to +2 dB above 5–10 kHz for "air"/width.
- Side low cut below 200 Hz (or hard mono-sum below 120–200 Hz) to center the bass image.
- Mid low-shelf cut at 100–200 Hz to tighten mud while keeping width.

**Tilt EQ** — single control rotates the spectrum around 650 Hz–1 kHz; ideal as a top-level "brightness" macro.

**Dynamic EQ** — compresses or expands a band only when threshold is crossed; replaces de-essers and tames moving resonances.

**Typical mastering gain/Q:** ±0.5 to ±2 dB with Q 0.5–2.0 (mostly 0.7–1.4). Anything beyond ±3 dB is corrective work that belongs in the mix.

### 5. Compression in mastering — specifics

**Broadband bus settings** (Waves Audio quoting mastering engineer Yoad Nevo; Joey Sturgis Tones; Splice; eMastered):
- Ratio: 1.2:1–2:1 (rarely above 2:1; Waves quotes Yoad Nevo: *"most mastering engineers use… typically 1.25:1 or 1.5:1 — rarely anything more than 2:1"*).
- Threshold: high; **1–2 dB of gain reduction max** (*"no more than 2 dB"* per Splice / eMastered; *"1 to 2 dB max"* per Joey Sturgis Tones).
- Attack: 20–100 ms.
- Release: 100–300 ms or auto / program-dependent.
- Knee: soft.

**Topology character (emulations):**
- **VCA** (SSL G-bus, API 2500): fast, clean, slightly punchy attack; default mastering bus tool.
- **FET** (1176): ~20 µs attack; colors transients; used as a parallel/aggression layer.
- **Opto** (LA-2A): slow, program-dependent; smooth on vocals/dynamic acoustic.
- **Vari-mu / tube** (Manley, Fairchild): smoothest gain reduction, slight upward harmonic generation; default for "glue" on classic/jazz/rock masters.

**Multiband compression:**
- Use as a problem-solver. Default OFF unless a clear band-specific issue exists.
- 2-band: crossover 120–200 Hz (isolates low end).
- 3-band: 150 Hz + 5–6 kHz.
- 4-band (cautious): 80 Hz + 250 Hz + 5 kHz; per Sean Kim's mastering guide, *"if you find yourself reaching for 4+ bands, it's usually a sign that the mix itself needs revision."*
- Crossover slope ≥18 dB/oct for separation.
- Per-band ratios: low 2:1–3:1, mid 1.5:1, high 1.5:1–2:1.

**Parallel / NY-style** — 8:1, 10+ dB GR, blended 10–30 %; adds sustain and density without choking transients. Common in modern metal.

**Upward compression** — Sonnox Oxford Inflator (positive-bias bit emphasis), Waves MV2, Klanghelm DC8C upward mode.

**Transient shapers** — SPL Transient Designer, Boz +10 dB, NI Transient Master; pre-limiter, ±2 dB on bus.

### 6. Saturation and harmonic enhancement

- **Tape emulation** — even+odd harmonics, program-dependent soft compression, HF loss (head bump + LPF), wow/flutter. Settings: 15 ips, +3 dBu reference, low drive (≤2 dB needle). Plugins: UA Studer A800, IK Tape Machine, Softube Tape, Kazrog True Iron.
- **Tube** — predominantly even-order; "warmth/openness"; subtle bus drive 1–3 % THD.
- **Transformer** — third-order dominant on small drives, fifth+ on harder drive; "iron" sheen. Rupert Neve, Manley emulations.
- **Multiband saturation** — saturate highs only (Brainworx bx_saturator V2, Soundtoys Decapitator tone control, FabFilter Saturn 2); adds "air" without harshness.
- **Exciters/enhancers** — Aphex Aural Exciter / SPL Vitalizer / Sonnox Oxford Inflator: psychoacoustic + harmonic; even harmonics 2–8 kHz from low-mid input.

**Drive amounts** — ≤2 % added THD on the bus; multiband HF saturation 3–5 %.

### 7. Stereo imaging and M/S processing

**M/S math:** M = (L+R)/√2, S = (L−R)/√2; L = (M+S)/√2, R = (M−S)/√2. The √2 normalization preserves loudness through encode/decode; unnormalized matrices compensate at decode.

**Mono-compatibility & bass mono summing:**
- Mono-sum sides below 80–200 Hz (streaming masters: 120–150 Hz; vinyl: 200–300 Hz) to prevent stylus jumps and mono-playback phase loss. M/S EQ side low-cut at 120 Hz is the standard implementation.
- Correlation meter should sit between 0 and +1; brief excursions to 0/slightly negative on stereo effects are fine; sustained negative is a red flag.

**Width control:**
- Boost side level / side high-shelf to widen — keep ≤+3 dB on sides to avoid mono-collapse.
- Haas / all-pass-based wideners cause comb-filtering in mono — avoid in mastering.
- Mid boost narrows; tightens center.

### 8. Limiting — the most critical algorithm

**Modern architecture (Pro-L 2 / Ozone Maximizer / Newfangled Elevate / Waves L3):**
1. Input gain ("Drive") pushes signal into the limiter.
2. **Look-ahead delay** (1–5 ms): calculates gain envelope before audio reaches the gain stage; catches transients without distortion.
3. **Multi-stage gain reduction:** fast "transient" stage + slower "release" stage in parallel — fast catches without long-term pumping.
4. **Program-dependent release:** adapts to envelope; FabFilter Pro-L 2's 8 algorithms (Transparent, Punchy, Dynamic, Allround, Modern, Aggressive, Bus, Safe) are essentially different attack/release/saturation profiles.
5. **Oversampling** (2×–32×): minimizes aliasing and internal intersample peaks from the fast gain reduction itself.
6. **True-peak limiting:** oversampled-peak detector post-gain-reduction adds smoothing so reconstructed analog never exceeds the ceiling. Pro-L 2 documents ~5 ms additional latency.
7. **Channel linking:** separate transient-link (often 70–90 %) and release-link (often 100 %).

**One-click defaults:**
- Lookahead: 1–3 ms (default 1.2 ms).
- Release: auto / 50–200 ms.
- Oversampling: 4×.
- True-peak ON, ceiling −1.0 dBTP (−2.0 for Amazon-target or master >−14 LUFS).
- Algorithm: "Transparent" / "Modern" / "Allround" by default; "Aggressive" for metal/EDM templates; "Safe" for acoustic/classical.
- Max bus GR: 2–6 dB on master limiter; metal/EDM templates tolerate 6–10 dB if a soft-clipper carries 1–2 dB upstream.

**Limiter character:**
- **FabFilter Pro-L 2** — transparent default, 8 algorithms, K-system/EBU R128 metering, surround/Atmos 7.1.4.
- **iZotope Ozone Maximizer (IRC 5)** — multiple IRC variants; tight Master Assistant integration.
- **Newfangled Audio Elevate** — multiband spectral limiter with 26-band psychoacoustic gain reduction.
- **Sonnox Oxford Limiter v3** — Enhance control for transient emphasis; analog feel.
- **Brainworx bx_limiter XL** — saturated character.
- **Waves L2 / L3 / L3-LL** — industry standards; L3 multiband, L2 broadband; L2 has audible character (often used as flavor).
- **Sonnox Inflator** — saturator/upward processor, increases perceived loudness 1–3 dB without explicit GR.

**Hybrid clipping-before-limiting** (modern loud-master technique):
- Soft clipper (Kazrog KClip 3, SIR StandardCLIP, Tokyo Dawn Limiter 6 clipping stage, FabFilter Saturn 2 clip, Newfangled Saturate) before brickwall.
- Drive 0.5–2 dB into clipper; shaves the highest transients invisibly.
- Limiter then needs 1–3 dB less work; can stay "Transparent" instead of "Aggressive."
- Net: 1–2 dB more apparent loudness for the same audible damage.
- Black Ghost Audio's published workflow also supports clipper *after* limiter at −1 dBTP for true-peak catch — pick one approach per template.

### 9. Dithering and bit-depth reduction

- **TPDF (Triangular PDF):** sum of two independent uniform sequences in [-1 LSB, +1 LSB]; PDF triangular over [-2, +2 LSB]; eliminates quantization noise modulation entirely. Mathematically optimal "white" choice; safe through subsequent processing.
- **Noise-shaped dither** — TPDF feedback-filtered to move noise to less audible HF (~16 kHz+). Improves perceived SNR by ~14 dB at the cost of higher peak noise and worse behavior under further processing.
- **Algorithm families:**
  - **POW-r 1** — minimal shaping; for high-dynamic music.
  - **POW-r 2** — moderate shaping; general music.
  - **POW-r 3** — aggressive curve (dip 3–4 kHz, +35 dB lift above 16 kHz per Sage Audio); for large-dynamic material like orchestral.
  - **iZotope MBIT+** — proprietary psychoacoustic shaping; None/Light/Medium/Aggressive/Ultra levels; peak-limit option suppresses spurious peaks below −60 dBFS.
  - **Apogee UV22 / UV22HR** — flat 1–18 kHz at ~5 dB below flat TPDF, ~+30 dB lift above 18 kHz.
  - **FabFilter Pro-L 2 built-in** — three TPDF curves (basic + two shaped).
- **When to dither:** ONCE, at the absolute end of the chain, when reducing bit depth. Never twice. Never EQ/compress/limit after.
- **16-bit delivery (CD)**: always dither. Use TPDF unless non-classical and untransformed downstream — then moderate noise-shape (POW-r 2 / MBIT+ Light) buys a little perceived headroom.
- **24-bit streaming**: dithering from 32-bit float to 24-bit is mostly cosmetic (noise floor ~−144 dBFS). Use flat TPDF if you dither at all.
- **Avoid noise-shaped dither** if the master will be further lossy-encoded — codecs reinterpret shaped noise and can produce audible artifacts.

### 10. Reference-based mastering and matching

- **iZotope Tonal Balance Control** — bundled genre target curves; Audiolens (iZotope's reference-capture tool) analyzes audio playing in any application/streaming service. Ozone 12 Master Assistant builds an EQ chain to nudge your master into the target.
- **FabFilter Pro-Q 3/4 Match EQ** — analyzes a reference clip and generates corrective EQ.
- **Loudness-matched A/B** — essential. Mastering The Mix Reference, ADPTR Metric AB, HoRNet Reference do real-time level-matched switching.
- **AI matchering** (Ozone Master Assistant, LANDR, eMastered, CloudBounce, BandLab) — what they actually do, as publicly described:
  - Spectral envelope estimation of input.
  - Genre classification (or user-selected target).
  - Selection of a preset chain (EQ curve + compression preset + limiter target).
  - Iterative gain matching to a loudness target.
  - LANDR runs a cloud render through genre-specific models trained on a large mastered-track corpus; presents three intensity levels.
  - Ozone Master Assistant additionally exposes the underlying modules for manual edit — the only meaningful "tweakable AI" path on the market.
  - **None** are end-to-end neural networks producing waveforms; they are classifiers + preset-pickers + conventional DSP.

### 11. AI/ML mastering — academic literature

Papers worth implementing or referencing for any ML personality layer:

- **Martínez Ramírez, M. A.; Reiss, J. D. — "End-to-end equalization with convolutional neural networks."** DAFx-18, 2018. CNN learns to apply EQ given paired input/target audio.
- **Mimilakis, S. I. et al. — "Deep Neural Networks for Dynamic Range Compression in Mastering Applications."** 140th AES Convention, 2016. Predicts per-critical-band compression coefficients from a filter-bank decomposition.
- **Martínez Ramírez, M. A.; Wang, O.; Smaragdis, P.; Bryan, N. J. — "Differentiable Signal Processing with Black-Box Audio Effects."** ICASSP 2021. arXiv:2105.04752. Trains a deep encoder to drive non-differentiable FX plugins using SPSA gradient approximation; explicitly demonstrates **automatic music mastering** as one of three applications, with results that the authors state are *"comparable to a specialized, state-of-the-art commercial solution for music mastering."* **Single most directly relevant paper.**
- **Steinmetz, C. J.; Bryan, N. J.; Reiss, J. D. — "Style Transfer of Audio Effects with Differentiable Signal Processing."** arXiv:2207.08759, 2022. Predicts mastering-style parameters from a reference recording; compares TCN end-to-end, neural-proxy, SPSA, and auto-diff.
- **Martínez-Ramírez, M. A.; Liao, W.-H.; Fabbro, G.; Uhlich, S.; Nagashima, C.; Mitsufuji, Y. — "Automatic music mixing with deep learning and out-of-domain data."** ISMIR 2022. arXiv:2208.11428. Mixing counterpart.
- **DeepAFx / DeepAFx-ST** (Adobe Research, open source: GitHub `adobe-research/DeepAFx` and `DeepAFx-ST`) — reference implementations of black-box differentiable DSP including a mastering FX-chain example (compressor + limiter).
- **DDSP — Engel, J. et al. — Google Magenta DDSP toolkit (2020).** Differentiable additive/subtractive synthesis layers; foundation for the differentiable-DSP movement.

Perceptual losses commonly used: multi-resolution STFT (Yamamoto et al. 2020), Mel-spectrogram L1, log-magnitude spectral convergence. JND-based weighting from psychoacoustic models appears in thesis-level work but is not common in shipped products.

For a product, the practical move is **not** end-to-end synthesis: (a) train a genre/style classifier on a curated corpus, (b) classifier picks a parameter preset for a hand-built DSP chain, (c) optionally refine parameters via SPSA-style gradient estimation against a perceptual loss to a user-supplied reference. That is essentially what Ozone Master Assistant does.

### 12. Format-adaptive delivery

Branch the chain at the end; render the same upstream master to multiple targets:

| Format | Sample rate | Bit depth | Loudness target | Peak ceiling | Dither | Special |
|---|---|---|---|---|---|---|
| Streaming default (Spotify/YouTube/Tidal/Amazon) | 44.1/48 kHz | 24-bit WAV | −14 LUFS | −1 dBTP (−2 if loud) | Flat TPDF | none |
| Apple Digital Masters | Native ≥44.1 kHz; ideally 24-bit/96 kHz | 24-bit | −16 LUFS (Sound Check trims down only) | −1 dBTP, verified | TPDF | Audit with `afclip`, `AURoundTripAAC`; **do not upsample** |
| Spotify Loud | Same | 24-bit | −11 LUFS if targeting Loud | −1 dBTP | as above | Optional |
| CD | 44.1 kHz | 16-bit | −10 to −14 LUFS | −0.3 to −1 dBFS sample peak | **TPDF or POW-r 2 mandatory** | PQ codes, ISRC per-track, 2-sec default gaps, DDP image with MD5 |
| Vinyl pre-master | 24-bit/48 or 96 kHz | 24-bit | −10 to −14 LUFS, **no brickwall** | −3 to −6 dB headroom | Don't dither pre-vinyl | Mono-sum sides <200 Hz (LFX/elliptical EQ); de-ess 5–10 kHz; LPF >18 kHz; balance side lengths within 1 minute (softer material on longer side); 12" 33 RPM ≤22 min/side (loud), longer = quieter cut |
| Hi-res streaming (Apple Lossless, Amazon HD, Tidal HiRes) | Up to 192 kHz | 24-bit | Same as standard | Same | Flat TPDF if reducing | Maintain native rate |
| Dolby Atmos (Apple Music / Amazon / Tidal) | 48 kHz | 24-bit LPCM | **−18 LKFS integrated** per ITU-R BS.1770-4 | **−1 dBTP** per BS.1770-4 | n/a | **BWF ADM** file, channel-based bed + objects; Dolby Atmos Renderer |
| Broadcast (EBU R128) | 48 kHz | 24-bit | −23 LUFS ±1 | −1 dBTP | TPDF if reducing | LRA ~7 LU for typical music |
| US broadcast ATSC A/85 (CALM) | 48 kHz | 24-bit | −24 LKFS ±2 | −2 dBTP | TPDF if reducing | Dialog-anchored measurement on dialogue content |
| Cinema Atmos / theatrical | 48 kHz | 24-bit | −27 LKFS (Netflix); SMPTE/Dolby for theatrical | −2 dBTP | n/a | Separate workflow; out of scope |

**Apple Digital Masters tooling** (ships with macOS):
- `afconvert` — CLI codec wrapper using Apple's AAC encoder (the actual Apple Music encoder).
- `afclip` — CLI intersample clip checker.
- `AURoundTripAAC` — AU plugin that A/Bs source vs encoded AAC in real time.
- Apple Digital Masters droplets — drag-and-drop GUI wrappers.

Apple's spec **requires** 24-bit at the source resolution (minimum 44.1 kHz, ideally 24-bit/96 kHz). Do **not** upsample (Apple explicitly: *"if you create your masters at 24-bit, 44.1 kHz, you should not upsample to 96 kHz"*). MQA is end-of-life: MQA Ltd officially appointed administrators on 3 April 2023, and Tidal announced in June 2024 that it would discontinue MQA playback support the following month (July 2024). Ignore for new products.

### 13. Genre-specific conventions

**Universal**: balanced low end below 100 Hz; mono-compatibility; controlled sibilance; 5–8 LU short-term dynamic range. The below override defaults only in the relevant template.

**Djent / progressive metal / modern metal** (Periphery, Meshuggah, Gojira; engineers Nolly Getgood, Andy Sneap, Jens Bogren, Forrester Savell, Joel Wanasek, Adam Getgood):
- Integrated loudness target: −7 to −9 LUFS (genre aesthetic, not streaming target). Master is *known* to be turned down by Spotify; listeners benefit from the dense character that survives normalization.
- Low end: kick fundamental usually 50–80 Hz; bass fundamental 80–150 Hz (downtuned 7/8-string guitars and basses extend into this region).
- **Mono-sum below 150–200 Hz** is mandatory — downtuned guitars at 80–150 Hz; any stereo info there destroys mono playback (live, club, phone). Implement with M/S EQ low-cut on sides or a dedicated Bass Mono plugin.
- Mid scoop or boost is genre-political; modern Bogren/Sneap masters favor a slight **boost** at 1–2 kHz over the '90s scoop, balanced by a 400–600 Hz cut to keep low-mids clean.
- 2–4 kHz "bite": 1–2 dB bus boost brings guitar pick attack forward; pair with de-ess on cymbals at 6–9 kHz.
- Parallel compression (10:1, fast attack, 6–10 dB GR) blended 15–25 % glues kit and bass.
- Soft-clip 1.5–2 dB before limiter is standard; 6–10 dB master limiter GR is normal.
- Reference territory: Periphery *P3-Select Difficulty*, Meshuggah *Koloss*, Gojira *Fortitude*, Architects *For Those That Wish to Exist*.
- Sub-bass / drop-tune protection: HPF at 28–35 Hz, 12 dB/oct, to keep limiter headroom for audible fundamentals.

**EDM / electronic / techno / DnB:**
- Integrated −7 to −9 LUFS for club masters; −10 to −11 LUFS streaming-optimized.
- Sub-bass focus 30–80 Hz; mono-sum below 100 Hz strictly.
- Wide highs above 8–10 kHz via M/S side high-shelf.
- Sidechain pumping baked into mix; mastering compressor must use slow attack (>50 ms) to avoid double-pumping.
- Limiter: Aggressive algorithm; ceiling −1 dBTP.

**Hip-hop / trap:**
- Integrated −8 to −10 LUFS.
- 808s: HPF at 25–30 Hz; gentle multiband at 40–80 Hz to control sub-bass pumping; sometimes upward compression at 80–120 Hz for sustained 808 power.
- 808 transients create severe intersample peaks after AAC/Ogg encoding; **−2 dBTP ceiling recommended for any trap master**.

**Pop:**
- Integrated −9 to −11 LUFS.
- Vocal presence dialed via mid-channel EQ around 2–3 kHz, +0.5 to +1 dB.
- Air shelf at 12 kHz on sides, +1 to +1.5 dB.
- Limiter: Transparent; 3–5 dB GR.

**Acoustic / folk / singer-songwriter:**
- Integrated −14 to −16 LUFS.
- Preserve LRA ≥10 LU.
- Broadband comp ≤1 dB GR; no multiband; no soft clip.
- Gentle 10–12 kHz air shelf.
- Limiter in Safe mode, 1–2 dB GR max.

**Classical / jazz:**
- Integrated −16 to −23 LUFS.
- LRA 12–22 LU.
- Minimal processing: HPF, broad tilt EQ, gentle vari-mu at <0.5 dB GR.
- No noise-shaping on classical (extra HF noise audible on quiet passages).
- Limiter only as true-peak safety net at −1 dBTP.

**Rock / indie:**
- Integrated −9 to −12 LUFS.
- Vari-mu / SSL G-bus emulation; 1–2 dB GR.
- Often benefits from tape saturation 1–2 % THD.
- Limiter: Transparent / Modern; 3–6 dB GR.

### 14. Tools and analysis meters

Modern reference set:
- **Spectrum analyzers**: Voxengo SPAN (free), iZotope Insight 2, FabFilter Pro-Q 3/4 built-in, Toneboosters EQ Magnitude.
- **Loudness meters**: Youlean Loudness Meter 2 (free, EBU R128/ATSC/streaming presets), Klangfreund LUFS Meter, Waves WLM Plus, NUGEN VisLM-H2, MeterPlugs LCAST/Dynameter, FabFilter Pro-L 2 built-in.
- **Phase correlation / vectorscope / goniometer**: SPAN/Insight built-in, Voxengo Correlometer, Brainworx bx_meter, iZotope Insight vectorscope.
- **Dynamic range / PLR / PSR**: TT Dynamic Range Meter (legacy DR scale), MasVis (open-source), MeterPlugs Dynameter, Loudness Penalty Analyzer (Ian Shepherd's per-platform-playback simulator).
- **Reference tools**: Mastering The Mix Reference, ADPTR Metric AB, HoRNet Reference, iZotope Audiolens.
- **Codec preview**: Sonnox Pro-Codec, MeterPlugs Loudness Penalty, Apple AURoundTripAAC.

### 15. Quality control and deliverables

**Sample rate / bit depth per platform:**
- All major streaming: accept 24-bit WAV at any sample rate ≥44.1 kHz; deliver native.
- CD: 16-bit / 44.1 kHz; DDP image preferred (Sonoris DDP Player Free, HOFA CD-Burn.DDP.Master).
- Vinyl: 24-bit / 48 or 96 kHz pre-master; cutting engineer applies RIAA.
- Atmos: 24-bit / 48 kHz BWF ADM.

**Metadata:**
- **ISRC** per track, in CD-Text and distribution metadata.
- **ID3v2.4** for MP3, **Vorbis comments** for FLAC/Ogg, **MP4 atoms** for AAC.
- Embedded loudness metadata mostly doesn't pass through distributors; don't rely on ReplayGain tags surviving.
- **iXML/BWF** chunks for broadcast/post.

**Spacing / fades:**
- Inter-track CD: 2 s default; album-listening albums often 0–0.5 s for crossfades, programmed via PQ codes.
- Streaming: distributor handles gapless; embed if available.
- Fade-ins: 5–50 ms unless artistic; fade-outs: programmed inaudible at −60 dBFS within 1–3 s.

**DDP image** for physical CD/vinyl: full PQ + audio + ISRC + CD-Text + UPC/EAN with checksum.

**QC listening environments:**
- Reference SPL: Bob Katz K-system at 83 dB SPL per speaker (C-weighted, slow) from −20 dBFS pink noise at the listening position; 79 dB per speaker is the AES music-mastering reference.
- QC on: mains (full-range), near-fields, earbuds (AirPods, generic Bluetooth), phone speaker, car if available.
- Three SPL levels: ~65, 75, 85 dB.

---

## Recommendations

**Stage 1 — Build the deterministic DSP graph first (no ML).** Implement the canonical chain in §3 with per-genre presets (§13) and per-format branches (§12). Use FabFilter Pro-L 2's documented behavior as your limiter reference (1–3 ms lookahead, 4× oversampling, true-peak detection per BS.1770-4 Annex 2, 12-tap-per-phase 48-tap polyphase FIR). Use the two K-weighting biquads with the published coefficients (§2). Validate against Youlean Loudness Meter and a reference like MeterPlugs Dynameter on a 50-track corpus.

**Stage 2 — Add measurement-driven assistance.**
1. A spectrum-based genre classifier (MFCC + GBM is enough for "metal/EDM/acoustic/pop/hip-hop/classical").
2. A tonal-balance matcher against a per-genre target curve (Tonal Balance Control style).
3. A loudness-penalty pre-flight that shows the user what each platform will do to their delivered master.

**Stage 3 — Optional ML personality layer.** Use the Steinmetz et al. (2022) DeepAFx-ST architecture or the Martínez Ramírez et al. (2021) black-box DSP architecture to predict parameters of your conventional DSP chain from a user-supplied reference. Train on paired premaster/master files; don't do end-to-end waveform synthesis.

**Stage 4 — Format-adaptive output renderer.** Single source-master in 32-bit float at native sample rate, with branch renderers per delivery target. Each renderer handles: format-appropriate dither (TPDF default), bit-depth quantization, SRC (high-quality polyphase, e.g., libsoxr / SRC), encoded preview generation (`afconvert` for AAC, libvorbis for Ogg, libopus for Opus), and a loudness-conformance check that re-runs BS.1770-4 measurement on the rendered output (not just internal bus) and warns/auto-trims if non-compliant.

**Stage 5 — Continuous QC dashboard.** Every rendered master is automatically scored on: integrated LUFS, max true peak, LRA, PLR, PSR (worst-section), mono-compatibility correlation, codec-preview true peaks for AAC 256 / Ogg 160 / Opus 128, and platform-penalty simulation.

**Auto-correcting thresholds:**
- If max TP post-render >−1 dBTP → reduce limiter ceiling 0.5 dB and re-render, up to 3 attempts; flag for manual review if still failing.
- If LRA <4 LU and genre ≠ EDM/metal/hip-hop → reduce limiter drive 2 dB; add upward compressor.
- If PSR <8 anywhere → bypass soft-clip stage, reduce limiter drive 1.5 dB.
- If correlation <0 sustained → tighten side level 2 dB or engage mono-summing below 200 Hz.
- If integrated loudness post-render >2 dB off target → adjust input gain and re-render.

**Don't:**
- Don't try to replace human mastering for classical/jazz. Those need a person and a calibrated room.
- Don't ship MQA support; it's dead.
- Don't oversell what AI mastering does. One-click mastering is excellent for streaming demos, middling for masters that need taste.

---

## Caveats

1. **Streaming targets are not legally binding and platforms change them quietly.** Numbers above are current 2025–2026 per the cited sources, but YouTube has changed its target at least twice (−13 → −14), and Apple flipped Sound Check to default-on/LUFS in 2022. Build targets as a config file, not a constant.
2. **Apple Music's −18 LUFS Atmos / −16 LUFS stereo numbers are the *integrated* loudness;** momentary peaks routinely sit much higher, which is why the −1 dBTP rule is enforced separately. Don't conflate them.
3. **"−14 LUFS is the universal target" is partially wrong** for genres whose aesthetic depends on density (metal, EDM, hip-hop). Spotify will turn those down on playback, but the *character* still differs at the source. Don't auto-clamp every master to −14 LUFS.
4. **`afclip` / `AURoundTripAAC` only check Apple's AAC encoder.** Spotify Ogg Vorbis transcoding can clip differently. Use Sonnox Pro-Codec or do your own libvorbis preview for Spotify-specific checks.
5. **Spotify "Loud" / "Quiet" listener distribution is not published precisely.** Reports range from 7–10 % of listeners on "Loud" mode; treat as 9 % and design for the 91 %.
6. **PLR/PSR values are descriptive, not prescriptive.** Useful as soft alarms; some genres (sludge metal, ambient drone) intentionally violate PSR-8 and sound correct.
7. **All AI mastering products lose to a skilled human** on material that requires taste judgments. Position accordingly.
8. **Dither noise-shape interactions with lossy codecs are real and under-discussed.** A POW-r 3 / MBIT+ Ultra master that sounds great on CD may produce audible swirling after Ogg Vorbis transcode. When in doubt, flat TPDF.
9. **Apple's `afconvert` AAC encoder is the actual Apple Music encoder** so the preview is reliable; equivalent claims for Spotify cannot be verified independently.
10. **Vinyl mastering, broadcast mastering, and theatrical Atmos are not one-click problems.** They have human-in-the-loop steps you should not try to automate. Generate a *pre-master* file and a spec sheet for the specialist engineer.
# Mastering Settings Reference

> Consolidated technical settings reference for the **album-mastering-studio** project.
> Compiled 2026-05-11; revised through five integrated source documents:
> - `compass_artifact_wf-0dd25647-…_text_markdown.md`
> - `deep-research-report.md` (original)
> - `audio-mastering-technical-research.md`
> - `compass_artifact_wf-e83b62aa-…_text_markdown.md`
> - `deep-research-report.md` (Downloads, 2026-05-11 — "Modern audio mastering for mastered deliverables")
>
> Focus: **mastering settings, parameters, technical specifications, operational rules**.
> Excluded: build plans, market analysis, UX/product, tech-stack decisions, AI interstitial generation, app architecture.

---

## 1. Loudness Measurement Standards

### ITU-R BS.1770 — The Measurement Substrate

The ITU-R BS.1770 family (BS.1770-1 through BS.1770-5, current rev Nov 2023) is the foundation that **all** other loudness specs reference — EBU R128, ATSC A/85, AES TD1008 / AES77, every streaming-platform delivery target.

**Algorithm — four stages:**
1. **K-weighting filter** — two-stage biquad cascade per channel
2. **Mean-square calculation** per channel over sliding window
3. **Channel-weighted sum** (LFE excluded)
4. **Gating** with absolute and relative thresholds

**Loudness output equation:**

```
L = −0.691 + 10 · log₁₀(Σᵢ Gᵢ · zᵢ)

where:
  Gᵢ = channel weight
  zᵢ = mean-square of K-weighted channel i

LKFS ≡ LUFS;   1 LU = 1 dB
```

**K-weighting filter — canonical 48 kHz biquad coefficients** (used in libebur128, pyloudnorm; the de-facto reference implementations):

```
Stage 1 — Pre-filter (high-shelf, ~+4 dB at ~1681 Hz, models head/torso):
  b0 =  1.53512485958697
  b1 = -2.69169618940638
  b2 =  1.19839281085285
  a1 = -1.69065929318241
  a2 =  0.73248077421585

Stage 2 — RLB high-pass (~38–60 Hz corner, approximates LF rolloff of equal-loudness curve):
  b0 =  1.0
  b1 = -2.0
  b2 =  1.0
  a1 = -1.99004745483398
  a2 =  0.99007225036621
```

**Critical implementation note:** these coefficients are *only correct at 48 kHz*. BS.1770-5 publishes the analog prototype (poles/zeros in s-domain); for other rates (44.1, 88.2, 96, 192 kHz) implementers must use the bilinear transform with frequency pre-warping. Naive reuse causes audible measurement error.

**Channel weights (BS.1770-5 §3.1):**

| Channel | Weight |
|---|---|
| L, R | 1.0 |
| C | 1.0 |
| Ls, Rs (surround) | 1.41 (≈+1.5 dB) |
| LFE | 0.0 (mandatory exclusion) |
| 7.1.4 / Atmos | BS.1770-5 extends with azimuth + elevation tables |

**Gating:**
- Block size **Tg = 400 ms**, **75% overlap** (hop = 100 ms)
- **Absolute gate Γa = −70 LUFS** — discard blocks below
- **Relative gate Γr = ungated_mean − 10 LU** — drop blocks below this, recompute mean
- **Integrated loudness** = mean of surviving blocks (entire program, not fixed window)
- Incomplete trailing blocks discarded

### Metering Primitives an App Must Expose

| Metric | Window | What it tells the app |
|---|---|---|
| **Momentary (M)** | 400 ms sliding, ungated | Fast overload perception, transient density |
| **Short-term (S)** | 3 sec sliding, ungated | Musical phrase-level density |
| **Integrated (I)** | Full programme, gated | Global playback-normalization relevance |
| **LRA** (Loudness Range) | 10th–95th percentile over gated short-term | Macro-dynamic spread (EBU Tech 3342) |
| **dBTP** (Max True Peak) | Continuous, oversampled | Downstream clipping risk after DAC/SRC/codec |
| **PLR** (Peak-to-Loudness Ratio) | I − dBTP | Practical master density vs headroom |

**Loudness Range (LRA)** — EBU Tech 3342. Statistical dynamic range; difference between 95th and 10th percentile of short-term loudness values (gated). **LRA is intentionally distinct from crest factor or "dynamic range"** — it describes variation over a larger time scale and is computed from a gated distribution rather than single fast peaks.

Mastering rules of thumb:
- LRA 6–14 LU for produced music
- LRA 2 LU = squashed
- LRA 15+ LU = classical / jazz / wide-dynamic

Typical mastered values:

| Genre | LRA (LU) |
|---|---|
| EDM | 3–6 |
| Metal / djent | 3–6 |
| Pop | 3.7–12 (wide variance) |
| Rock | 6–10 |
| Indie folk | 8–14 |
| Jazz | 13–23 |
| Classical / orchestral | 20–32 |
| Ambient | 12–25+ |

### LUFS vs RMS vs dBFS

| Unit | Meaning | Use |
|---|---|---|
| **dBFS** | Sample amplitude only, unweighted | Peak detection, headroom |
| **RMS** | Average level, unweighted/ungated | Legacy K-system metering, rough loudness |
| **LUFS / LKFS** | K-weighted, gated | Normalization decisions, delivery compliance |

Rough conversion: **−9 RMS ≈ −11 LUFS** for typical music. Don't use RMS for delivery targets — use LUFS.

### True Peak (dBTP) Algorithm

Defined in BS.1770 Annex 2:
- **Upsample ≥4×** via polyphase FIR (BS.1770 default: 48-tap, 12 taps per phase, 4 phases)
- Apply low-pass at fs/2
- Take absolute maximum → dBFS reading
- High-end implementations (FabFilter, iZotope): higher-order Kaiser-windowed FIR with 8×–32× linear-phase oversampling

**FabFilter Pro-L 2 reference behavior:** 4× oversampling + 0.1 ms minimum lookahead keeps inter-sample overshoot within ±0.1 dB. Cheap implementations (parabolic interpolation, 2× linear) undershoot by up to 1.5 dB — dangerous for codec survival.

**Detector chain:** `x → 4× FIR upsample → LPF at fs/2 → abs() → max → dBFS`

**Reference open-source implementation:** Essentia's `TruePeakDetector` is BS.1770-conformant.

### EBU R 128 — Broadcast Compliance

| Parameter | Value |
|---|---|
| Integrated loudness target | **−23 LUFS** |
| Tolerance | ±0.5 LU (live: ±1 LU) |
| True-peak ceiling | **−1 dBTP** |
| Loudness Range | descriptor only, no max |

**Companion documents** (essential reading for implementing meters):
- **EBU Tech 3341** — meter behavior, M/S/I refresh rates (≥10 Hz short-term, ≥1 Hz integrated), scale, color zones
- **EBU Tech 3342** — Loudness Range (LRA) computation
- **EBU Tech 3343** — production guidelines / loudness normalization philosophy
- **EBU Tech 3344** — distribution / reproduction guidelines
- **EBU Tech 3352** — ISRC embedding in BWF axml chunk
- **EBU Tech 3285** — Broadcast Wave Format (BWF) specification v2 (2011 reissue)

### EBU R 128 s2 — Streaming Supplement

Distinct from R 128 proper. Covers streaming-specific guidance for broadcast organizations:
- **Stream unchanged at −23 LUFS when metadata/device gain is available**
- **Interim broadcaster-controlled distribution may sit around −20 to −16 LUFS**
- This is for broadcast-streaming workflows, **not general music streaming**

### AES TD1008 → AES77 — Audio Streaming Delivery

AES TD1008.1.21-9 (Sep 2021) was upgraded to **AES77 (Jul 2023)** — *Recommended Practice for Loudness of Internet Audio Streaming and On-Demand Distribution*:

| Mode | Target |
|---|---|
| Music, track-normalized | **−16 LUFS** |
| Music, album normalization (loudest track) | **−14 LUFS** |
| Speech | **−18 LUFS** |
| Assorted / interstitial content | −18 LUFS |
| Max true peak at codec input | **−1 dBTP** |

Apple's −16 LUFS Sound Check aligns with TD1008/AES77 track-norm; Spotify's −14 LUFS aligns with the album-loudest-track rule.

### AES Online Audio-Only Format-Specific Targets

For radio/podcast-style output more than album mastering:

| Format | Target |
|---|---|
| Pop music | **−16 LUFS** |
| Mixed format | **−17 LUFS** |
| News / talk | **−18 LUFS** |

### Reference Implementations

- **libebur128** (MIT) — canonical C implementation, ships test vectors. Use as reference oracle for any custom meter.
- **pyloudnorm** — pure-Python, ±0.1 dB ITU compliance, swappable filter classes (DeMan, Fenton/Lee).
- **Essentia `TruePeakDetector`** — BS.1770-conformant true-peak reference.
- **loudness-scanner** — CLI for batch measurement.

---

## 2. PLR / PSR — Dynamic Range Metrics

Beyond LUFS-I and LRA, two additional descriptors matter for mastering decisions:

| Metric | Definition | Use |
|---|---|---|
| **PLR** (Peak-to-Loudness Ratio) | sample/true peak − integrated LUFS (whole program) | Overall dynamic character |
| **PSR** (Peak-to-Short-term Loudness Ratio) | peak − short-term LUFS (smaller windows) | "Instantaneous dynamics"; detects transient damage |
| **Crest factor** | peak − RMS | Legacy unweighted version |

### Ian Shepherd's PSR ≥ 8 Rule

Per the MeterPlugs *"Crest Factor, PSR and PLR"* article (18 May 2017) — co-creator of the Dynameter plugin:

> *"Mastering engineer Ian Shepherd … recommends going no lower than PSR 8 during the loudest parts of a song. This goes for music in any genre. Anything less than this will often sound crushed."*

### PLR Scale (SonaMetro practical scale)

| PLR | Character |
|---|---|
| **5** | Squashed |
| **8** | Modern loud |
| **12** | Dynamic |
| **16** | Audiophile / classical |

### Genre PLR Ranges

| Genre cluster | PLR range |
|---|---|
| EDM / trap / modern metal | 5–8 |
| Pop / rock / hip-hop | 8–11 |
| Indie / acoustic / well-mastered rock | 11–16 |
| Classical / dynamic jazz | 16–22 |

### Real-World Examples (Sound on Sound, August 2017)

| Track | PLR | min PSR | Result on streaming |
|---|---|---|---|
| Grand Prix *Samurai* (heavily-limited remaster) | 7 | 5 | "quiet and feeble in a loudness-normalised regime" |
| Supertramp *Bloody Well Right* | 16 | 8 | "works well in all loudness-normalised regimes" |
| Dave Brubeck dynamic jazz (solo trumpet) | 20 | 9 | Reference dynamic |

### Implementation

Add PLR and PSR (worst-section) to every QA report. Use PSR < 8 as a soft alarm. PLR > 16 likely indicates undermastered material (for streaming genres). PSR/PLR are **descriptive, not prescriptive** — some genres (sludge metal, ambient drone) intentionally violate PSR-8 and sound correct.

---

## 3. Platform Delivery Targets (Merged)

| Platform | Integrated LUFS | True-peak ceiling | Normalization behavior | Codec | Notes |
|---|---|---|---|---|---|
| **Spotify** (Normal) | −14 | −1 dBTP (−2 if master > −14 LUFS) | Attenuates loud, boosts quiet with ~1 dB codec headroom; album-aware on album playback, track-mode on shuffle/playlist | Free: AAC 128 kbit/s (web); Premium: Ogg Vorbis up to ~320 kbit/s desktop/mobile, AAC 256 web, **FLAC 16-bit/44.1 kHz lossless (rollout 2025)** | Spotify's in-line limiter on tracks it must boost: "−1 dB sample values, 5 ms attack, 100 ms decay" — don't rely on it favorably |
| **Spotify** (Loud) | −11 | −1 dBTP | Attenuates, caps boost | same | Premium user-selectable; ~7–10% of listeners |
| **Spotify** (Quiet) | −19 to −23 | −1 dBTP | Attenuates, boosts | same | Premium user-selectable |
| **Apple Music** (Sound Check) | **−16 LUFS** | −1 dBTP (Apple Digital Masters) | **Turns DOWN only**; default-on for new iOS/macOS | AAC 256 kbit/s, ALAC lossless up to 24-bit/192 kHz; Dolby Atmos | Aligns with AES77 track-norm; preserves dynamics; user-disable-able |
| **Apple Music Dolby Atmos** | **−18 LKFS** per ITU-R BS.1770-4 | **−1 dBTP** per BS.1770-4 | Per-track integrated check | BWF ADM, 24-bit LPCM @ 48 kHz | Channel-based bed + objects via Dolby Atmos Renderer |
| **Tidal** | −14 (album normalization broadly used) | −1 dBTP | Negative-gain (album-aware) | AAC, FLAC, MQA legacy (end-of-life), FLAC HiRes | MQA: MQA Ltd appointed administrators April 2023; Tidal removed MQA July 2024 |
| **YouTube** (video) | −14 | −1 dBTP | **Turn-down only** (no boost) | AAC, Opus | 48 kHz / 24-bit recommended for video uploads |
| **YouTube Music** | Attenuates only tracks above **~−7 LUFS** (per Ian Shepherd *Production Advice*) | −1 dBTP | Attenuation-only | AAC, Opus | Substantially more permissive than YouTube video |
| **Amazon Music** | −14 (industry-cited; Amazon doesn't publish official) | **−2 dBTP** (stricter — Alexa/Echo ISP-prone) | Turn-down only | AAC, FLAC up to 24-bit (HD/Ultra HD) | |
| **Deezer** | −15 | −1 dBTP | Cannot be disabled | MP3, FLAC on HiFi | |
| **Pandora** | Not LUFS-based (proprietary ReplayGain-style) | −1 dBTP | Both directions | AAC | |
| **SoundCloud** | **No normalization** | −1 dBTP recommended | None | Opus 64 kbit/s free; AAC 256 kbit/s Go+ | Treat as no-normalization safety case |
| **Bandcamp** | **No normalization** | −2 dBTP recommended | None | FLAC/ALAC/MP3 | |
| **EBU R128 (broadcast)** | −23 ±0.5 LU | −1 dBTP | Strict compliance | varies | LRA ~7 LU for typical music |
| **EBU R128 s2 (streaming, broadcast-controlled)** | Unchanged −23 LUFS if metadata/device gain available; interim −20 to −16 LUFS | −1 dBTP | Distribution-aware | varies | Broadcast-streaming only |
| **ATSC A/85 (US broadcast / CALM)** | **−24 LKFS ±2** | −2 dBTP | Strict; dialog-anchored on dialogue | varies | |
| **Netflix originals** | **−27 LKFS ±2** | −2 dBTP | Strict compliance | AAC-LC / Dolby Digital Plus | Dialog gating |
| **AES77 — track-norm music** | −16 | −1 dBTP at codec input | spec recommendation | n/a | |
| **AES77 — album loudest** | −14 | −1 dBTP at codec input | spec recommendation | n/a | |
| **AES77 — speech / interstitial** | −18 | −1 dBTP | spec recommendation | n/a | |
| **CD (Red Book)** | −10 to −14 typical | Sample peak −0.3 to −1 dBFS | n/a | 16-bit / 44.1 kHz PCM | |
| **Vinyl premaster** | −10 to −14, no universal target | No brickwall; −3 to −6 dB headroom for cutter | n/a | 24-bit / 88.2 or 96 kHz | |

### Universal Master Heuristic

**−14 LUFS integrated, −1 dBTP, LRA ≥ 5 LU.** Satisfies every major streaming platform without separate masters. Only Apple Music (−16), Apple Atmos (−18), EBU R128 (−23) diverge enough to potentially want dedicated deliverables.

### The "Don't Master To A Number" Counterpoint

Major mastering engineers (Dave Kutch, Bob Ludwig, Pete Lyman) explicitly argue against mastering *to* a service target. The mastering app principle they recommend:

> Separate **analysis targets** from **creative targets**. Measure everything, but intervene only enough to achieve translation, technical compliance, and codec-safe output.

The logic:
1. First optimize **clarity, density, and distortion behavior** (sound quality)
2. Second ensure **true-peak safety** (codec survival)
3. Third **report what normalization will do on playback** (set expectations)

In an app: don't auto-clamp every master to −14 LUFS. Use loudness targets as **analysis constraints with warnings**, not as creative destinations.

### Important Implications

- **A master at −8 LUFS on Spotify "Normal" plays at the same level as a −14 LUFS master**, only with 6 dB less dynamics. Zero playback-level benefit beyond platform target unless targeting Spotify "Loud" mode (~9% of listeners).
- **Apple's "down-only" rule** means very quiet masters (<−18 LUFS) sound weak everywhere; aim ≥−16 LUFS unless dynamics are the artistic point.
- **Lossy codecs add 0.3–1.5 dB of intersample peak post-transcode** in practice; −2 dBTP for loud masters is empirically validated headroom.
- **AAC-LC 256 kbit/s (Apple):** MDCT-based; can spike intersample peaks on cymbals/sibilance.
- **Ogg Vorbis (Spotify desktop/mobile):** MDCT + floor/residual; sensitive to high-frequency density and brickwall artifacts.
- **Opus (YouTube/SoundCloud free):** hybrid CELT/SILK; transparent at 128 kbit/s but harsh transients can pump.
- **MP3 (Deezer Free, legacy):** subband + MDCT; intersample peaks routinely 1–3 dB above sample peaks.

### Caveat

Platform targets have shifted multiple times (YouTube changed −13 → −14; Apple flipped Sound Check to default-on/LUFS in 2022; Spotify changed default headroom). **Build targets as a config file, not constants.** Re-verify against published platform docs at release time.

---

## 4. Genre-Specific Mastering Targets

Working-engineer norms for streaming-first delivery (Ian Shepherd, Bob Katz school). All values are starting points; always editable per project.

| Genre | Integrated LUFS (competitive) | Integrated LUFS (album-safe) | LRA (LU) | PLR | True-peak | EQ tendencies | Compression / limiting | Multiband | Saturation / stereo | Vinyl / CD notes |
|---|---|---|---|---|---|---|---|---|---|---|
| **Acoustic folk / singer-songwriter** | −14 to −16 | −14 to −12 | 9–14 | 11–16 | −1 to −1.5 dBTP | Air shelf 10–16 kHz; warmth 200–400 Hz; vocal presence 2.5–4 kHz; HPF 30–40 Hz | Bus comp ≤1 dB GR, soft-knee, slow attack; minimal limiting 1–2 dB; **no multiband, no soft clip** | Off | Saturation low; natural width; ambience tails matter | Vinyl-friendly by default; CD only needs dither |
| **Indie folk / Americana** | −14 to −12 | −13 to −11 | 8–12 | 10–14 | −1 dBTP | + thicker low-mids 150–300 Hz | More glue comp; parallel comp for acoustic body | Off | Same | Same |
| **Indie rock / alt-rock** | −12 to −10 | −13 to −11 | 6–10 | 8–12 | −1 dBTP | Scoop 300–500 Hz for vocal space; lift 4–6 kHz for cymbals | Vari-mu / SSL G-bus; 1–2 dB GR; 3–6 dB limiter GR | Optional 2-band ~150 Hz | Tape sat 1–2% THD common | Vinyl: reduce side info |
| **Pop** | −10 to −8 (loud) / −9 to −11 (alt-pop) | −11 to −10 | 4–8 | 7–10 | −1 dBTP | Bright top 8–12 kHz; tight bass <100 Hz; vocal forward 2–4 kHz (mid +0.5–1 dB); side air shelf 12 kHz +1 to +1.5 dB | Heavy limiting; clipper-into-limiter chain; transient shaping | Low/mid/high; 150 Hz, 5 kHz | Harmonic enhancement useful | Vinyl: reduce top + sides |
| **Hip-hop / trap** | −8 to −10 | −11 to −9 | 4–7 | 6–9 | **−2 dBTP** (808 transients create severe ISPs after AAC/Ogg) | 808s: HPF 25–30 Hz; multiband 40–80 Hz to control sub-pumping; sometimes upward comp 80–120 Hz for sustained 808; top presence 8–12 kHz; vocal 1.5–3 kHz | Heavy limiting; transient-shaped 808s | 2 or 3-band; low 80–120 Hz | Bus saturation; vocal centered | |
| **EDM / electronic / techno / DnB** | −7 to −9 (club) / −10 to −11 (streaming) | −10 to −9 | 3–6 | 5–8 | −1 dBTP | Sub focus 30–80 Hz; **mono-sum strictly below 100 Hz**; wide highs > 8 kHz via M/S side shelf | Slow attack >50 ms to avoid double-pumping mix sidechain; Aggressive limiter | 2-band 100 Hz min; often 3-band | Pronounced saturation; wide top, mono bass | |
| **Metal / djent / post-metal** | −7 to −9 (often −5 djent) | −10 to −9 | 3–6 | 5–8 | −1 dBTP | Scoop or **modern boost at 1–2 kHz** (Bogren/Sneap school) with 400–600 Hz cut; 2–4 kHz "bite" +1–2 dB; **HPF 28–35 Hz @ 12 dB/oct** to protect limiter headroom for fundamentals; 6–9 kHz cymbal de-ess | Soft-clip 1.5–2 dB before limiter is standard; 6–10 dB master limiter GR normal | Low band for palm-mutes/kick bloom; low-mid for dense guitars | Subtle harmonic glue; **mono-sum < 150–200 Hz mandatory** (downtuned 7/8-string guitars at 80–150 Hz); parallel comp 10:1 fast attack 6–10 dB GR, blended 15–25% | Vinyl: relax limiting, mono lows more aggressively |
| **Rock / indie** | −9 to −12 | −11 to −10 | 6–10 | 8–12 | −1 dBTP | Standard | Vari-mu / SSL G-bus 1–2 dB GR; Transparent/Modern limiter 3–6 dB GR | Optional | Tape sat 1–2% THD | |
| **Classical / jazz** | −16 to −23 | −16 to −20 | 12–22 | 16–22 | −1 dBTP | Minimal processing: HPF, broad tilt EQ | Gentle vari-mu < 0.5 dB GR; limiter as true-peak safety only | Off | **No noise-shaping** (extra HF noise audible on quiet passages) | Vinyl-friendly |
| **Ambient / drone** | −18 to −14 | −16 to −14 | 12–25+ | 15–25+ | −1 to −1.5 dBTP | Preserve LF + HF; HPF only as needed | Almost no comp; gentle peak limiting | None | Saturation low; very wide OK | Vinyl-native |

### Djent / Modern Metal — Implementation-Specific Guidance

Beyond the table row, dense modern metal benefits from:

- **Preserve kick/snare transient definition** with slower compressor attack than the mix would use (transients survive limiting better)
- **Monitor low-mid accumulation** from guitars and bass around the "punch/wool" region (200–500 Hz)
- **Control intermittent upper-mid and treble harshness dynamically** rather than with large static cuts
- **Extra strict codec preview** — cymbals, clipped guitars, and dense limiter activity create audible AAC/MP3 edge
- **Small adaptive corrections** rather than large static "metal EQ curves"

### Engineer / Album References per Genre

For ear-training the target sound:

| Genre | Reference albums / engineers |
|---|---|
| Folk | Bon Iver *For Emma, Forever Ago* (Bob Ludwig); Nick Drake *Pink Moon*; Sufjan Stevens *Carrie & Lowell* |
| Indie rock | The National *Trouble Will Find Me* (Greg Calbi); Big Thief *U.F.O.F.* |
| Pop | Billie Eilish *When We All Fall Asleep* (John Greenham); Phoebe Bridgers *Punisher* |
| Metal / djent | Periphery *Periphery III / V / P3-Select Difficulty* (Adam "Nolly" Getgood); TesseracT *Sonder* (Acle Kahney); Meshuggah *Koloss / The Violent Sleep of Reason* (Vlado Meller); Gojira *Fortitude*; Architects *For Those That Wish to Exist* |
| Djent engineers | Nolly Getgood, Andy Sneap, Jens Bogren, Forrester Savell, Joel Wanasek |
| Post-metal | Cult of Luna *A Dawn to Fear*; Russian Circles *Gnosis* |
| Ambient | Brian Eno *Ambient 1: Music for Airports*; Stars of the Lid *And Their Refinement of the Decline* |
| Genre-bridging precedent | Opeth *Damnation* / *Deliverance* pair; Anathema post-2003 |

### Album-Spanning Approach: The Genre-Contrast Problem

When mastering an album that crosses wide genre territory (e.g., acoustic folk → djent → folk), three practical approaches:

1. **Master each track to its own genre target** — accept that folk lands at −14 to −16 LUFS and djent at −7 to −9 LUFS. Trust album-mode normalization on Apple/Tidal to preserve relationships.
2. **Match the loudest *moments* (Short-Term LUFS), not integrated** (Bob Katz / Ian Shepherd approach). If the loudest moment of each track peaks at, say, −10 LUFS Short-Term, tracks feel related even with very different integrated values. This is the **cohesion-without-flattening trick.**
3. **Tonal-balance-match the high and low ends** across all tracks. Use one reference (album track or external) and apply reference-matching (Matchering-style RMS/FR/peak/width matching) to bring all tracks into the same spectral envelope before final per-track mastering.

---

## 5. Operating Modes — Adaptive Classification

A modern one-click mastering app should classify input into **broad operating modes** based on measured spectral density and dynamic descriptors — **not genre tags alone.**

### The Four Universal Modes

| Mode | When triggered | Behavior |
|---|---|---|
| **Transparent (default)** | Most material; clean source; moderate dynamics | Conservative envelopes; minimal processing; default for unknown input |
| **Standard contemporary** | Modern pop / rock / mainstream with target loudness | Light glue compression; modest limiting; codec-safe ceilings |
| **Dense / loud** | Pop, rock, metal, EDM, hip-hop — high spectral density input | More peak shaving allowed; soft-clip pre-stage on; aggressive limiter algorithm; **stricter harshness detection, low-end mono discipline, codec preview/QC** |
| **Dynamic / acoustic** | Acoustic, jazz, classical, filmic, ambient | Limiter as true-peak safety only; broadband comp ≤1 dB GR; no soft clip; no multiband; no noise-shaping on output |

### Classification Logic

Classify by **measurement**, not metadata:
- **Spectral density** (FFT, smoothed; energy in 100–8 kHz band)
- **Transient density** (onset detection rate)
- **LRA** (high → dynamic mode; low → dense mode)
- **PLR / PSR distribution** (high → dynamic; low → dense)
- **Crest factor**
- **Low-frequency correlation** (stereo bass discipline)

User can override classification, but defaults bias toward conservative (Transparent) mode unless evidence for Dense or Dynamic is clear.

### Conservative Implementation Envelopes (One-Click App Synthesis)

This table is **not a standard** — it's a defensible conservative synthesis of standards, textbooks, and manuals for an automatic system. Editable per genre/mode.

| Stage | Conservative default envelope | Rationale |
|---|---|---|
| **Input conditioning** | Analyze native SR/bit depth; convert internally to 32- or 64-bit float; trim to comfortable headroom before processing | Preserves resolution; avoids premature clipping; platforms prefer highest-native delivery |
| **Corrective EQ** | Broad bells/shelves within **±0.5 to ±2 dB**; reserve larger cuts for obvious faults; HPF only when needed (20–35 Hz region) | Mastering EQ is "minutiae"; large moves indicate mix problems |
| **Broadband compression** | Ratio **1.1:1 to 2:1**; attack **10–80 ms**; release **50–300 ms or auto**; aim for **0.5–2 dB GR** in transparent mode | Glue and macrodynamics without flattening transients |
| **Dynamic EQ / de-ess** | Event-driven reduction **0.5–3 dB**; target only problem regions when triggered | Better than static EQ when harshness is intermittent |
| **Multiband compression** | Use only when band is unstable; ratios **1.2:1 to 2.5:1**; per-band GR **0.5–2 dB** | Powerful but easy to overdo |
| **Saturation / clipping** | **Off by default** in universal mode; if used, keep peak shaving modest; re-check TP and codec preview immediately | Can increase loudness efficiency but raises aliasing/harshness/codec-overs risk |
| **Width / M-S refinement** | Bass effectively mono or width-constrained **below 80–150 Hz**; prefer tiny side-only EQ shelves over aggressive widening | Low-frequency width reduces translation |
| **Final limiter** | Oversampling ON; true-peak ON; **streaming-safe ceiling ≈ −1 dBTP**; if single-stage limiting must exceed **~2–4 dB** often, consider staged strategy or back off | Prevents downstream codec/SRC overshoot; keeps limiter from becoming the sound of the record |
| **Dither** | **One pass only, last in chain, only when reducing bit depth**; prefer MBIT+; TPDF/Type 2 as fallback | Required for requantization; post-dither processing undermines it; stronger shaping can raise peaks |

### Limiter Distribution-Awareness

The limiter should be **distribution-aware**:
- **Streaming-safe mode:** prioritize true-peak protection + codec preview (default for most output)
- **CD-only / in-house reference mode:** can permit slightly tighter sample-peak ceilings if no lossy encode is expected

---

## 6. Modern Mastering Philosophy & Engineer Perspectives

These design principles come from published interviews and writing by working mastering engineers. They inform every default the app should ship with.

### Core Philosophy

> **"Problem solving first, taste second, loudness last."**

The chain order itself encodes this: corrective EQ → broad shaping → dynamics → final loudness/limiting. Once a limiter is working hard, any unresolved harshness or mud gets amplified perceptually. Fix problems before maximizing.

### A Conditional Chain, Not a Fixed Franchise

Every stage should be bypassable. The modern consensus is **least amount of processing needed to reach translation, cohesion, and technical compliance**. Default to bypass; engage only when measurement or listening identifies a need.

### Engineer Perspectives

**Dave Kutch** — masters for the record, not for service targets:
- Loudness targets are not the first thing he considers
- "Sound is" the priority
- Verify loudness and true peak **afterward**, not as a destination

**Pete Lyman** — minimal processing philosophy:
- WaveLab-based workflow: sequence → rough fades/spacing → passive whole-project listen → focused per-track corrections → peak limiting/selective de-essing → render continuous master → generate versions and metadata
- "Least processing necessary"
- Coarse moves first, fine moves second

**Bob Ludwig** — mastering as minutiae:
- "Totally dealing with minutiae"
- A **3 dB master EQ move is already a lot**
- Prior damage to the source is **irreversible** — don't try to fix a broken mix in mastering
- Apple's codec-check tools (`afclip`, `AURoundTripAAC`) are essential

### Hybrid Workflow

Hybrid (analog + digital) engineers commonly use this sequence:
1. **Coarse analog moves** (broad EQ, glue compression, summing)
2. **Fine digital moves** (surgical EQ, dynamic EQ, limiting)
3. **DAW/editor stage** for deliverable rendering and metadata

For a pure-digital app, the analogue: coarse passive moves first (broad EQ, light glue), surgical moves second (dynamic EQ, narrow corrections), maximizing/dithering/exporting last.

### Translation vs Loudness

The two competing goals of mastering:
- **Translation** — sound right on phone speakers, earbuds, club PA, car system, monitors
- **Loudness** — competitive level on streaming platforms

Translation wins when forced to choose. A master that sounds great everywhere at −15 LUFS beats one that sounds harsh on earbuds at −9 LUFS. Platforms normalize anyway.

### Implications for Software Defaults

- **Default to bypass** for every stage; engage only on evidence
- **Default to transparent** algorithms (limiter "Transparent" / "Modern" not "Aggressive")
- **Default to conservative envelopes** (Section 5 table)
- **Warn, don't enforce** — show user what each platform will do; don't auto-clamp
- **Surface measurement, not targets** — show LUFS-I, dBTP, LRA, PSR; let the user decide
- **A/B at matched loudness** — always level-match before comparing source vs processed

---

## 7. Canonical Mastering Signal Chain

Consensus across iZotope, Sound on Sound, Mastering The Mix, Sage Audio, Sonarworks, Audiospectra, Beat Kitchen, Bob Katz, Jonathan Wyner:

```
Input → [Gain Stage]
      → [Corrective EQ — min-phase, HPF + surgical cuts]
      → [Dynamic EQ / De-ess (optional, event-driven)]
      → [Broadband Compressor]                       (1.1:1–2:1, 0.5–2 dB GR transparent)
      → [Multiband Compressor (conditional)]          (2–4 bands, problem-solver only)
      → [Saturation / Harmonic Enhancer]              (tape/tube/transformer, ≤2% THD)
      → [Tonal / Musical EQ (often M/S)]
      → [Stereo Imaging / M-S Width]                  (bass mono < 80–200 Hz)
      → [Soft-clipper (optional, 0.5–1.5 dB)]
      → [True-Peak Brickwall Limiter]                 (4× oversampling, 1–5 ms LA, −1 dBTP)
      → [Dither + Noise Shaping]                      (ONLY at final bit-depth reduction)
      → Output (separate branches per delivery format)
```

### The Conditional Decision Tree

```
A. Ingest and QC
B. Gain trim and references
C. Corrective needs?
   YES → Corrective EQ or dynamic EQ → D
   NO  → D
D. Broad tonal shaping
E. Macro dynamics unstable?
   YES → Broadband compression → F
   NO  → F
F. Optional spectral control
G. Band-local instability?
   YES → Multiband or dynamic EQ → H
   NO  → H
H. Optional saturation or clipping
I. Mid-side or width refinement
J. Final true-peak limiting
K. Bit-depth reduction?
   YES → Dither last → L
   NO  → L
L. Export
M. Codec preview and final QC
```

### Rationale

- Corrective EQ **before** compression: compressor responds to a tonally balanced signal
- Saturation **after** compression: harmonics ride on already-controlled dynamics
- Tonal EQ **after** saturation: taste post-saturation timbre
- Soft-clip just **before** limiter: shaves the worst peaks invisibly so the limiter does less work
- Dither **absolutely last**: any processing after dither destroys it

### Switchable Templates

| Template | Soft-clip | Multiband | Comp GR | Limiter algorithm |
|---|---|---|---|---|
| **Aggressive / EDM / metal** | ON, 1.5–2 dB drive | ON, 4 bands | 2–3 dB | Aggressive |
| **Acoustic / classical** | OFF | OFF | ≤1 dB | Safe / Transparent |
| **Loud hip-hop / trap** | ON | 2–3 band | 2 dB | Modern (ceiling −1 dBTP, sometimes −2 for trap) |
| **Pop** | optional | 2–3 band | 2–3 dB | Transparent / Modern |
| **Rock / indie** | optional | optional | 1–2 dB | Transparent |
| **Pultec Low macro** | n/a | n/a | n/a | Pultec-style simultaneous low-shelf boost + cut at 60–100 Hz |

### Parallel Routing Options

- **Parallel compression (NY-style)** — heavily compressed copy mixed back at 10–30% for glue
- **Parallel saturation** — distorted copy summed back for grit without dirtying transients
- **M/S parallel** — mid and side processed independently (default M/S architecture inside EQ/comp modules)

### What Does NOT Belong in a Mastering Chain

Reverb, delay, gates, expanders (except occasional downward expansion for noise), heavy de-essing (mix problem), creative pitch shifting. Mastering processes the **whole programme**; mix processes individual elements.

---

## 8. EQ Settings

### Two Roles, Two EQs

**Corrective EQ (first in chain):**
- Surgical cuts, narrow Q (4–10), small attenuations (−1 to −3 dB)
- Phase mode: **minimum phase preferred** for transients

**Tonal / additive EQ (later in chain):**
- Broad boosts, wide Q (0.5–1.5), small lifts (+0.5 to +2 dB)
- Phase mode: **linear phase preferred** for symmetric impulse response

### Typical Mastering Gain / Q

**±0.5 to ±2 dB with Q 0.5–2.0** (mostly 0.7–1.4). Anything beyond ±3 dB is corrective work that belongs in the mix.

**Bob Ludwig:** *"A 3 dB master EQ move is already a lot."*

### Surgical Filter Ranges

| Frequency | Issue | Treatment |
|---|---|---|
| 20–40 Hz | Sub-sonic rumble | HPF 12–24 dB/oct (pre-compression) |
| 60–100 Hz | Kick fundamental / bass body | Pultec bump+cut |
| 200–400 Hz | Mud | −1 to −3 dB cut, Q 0.7–1.5 |
| 400–800 Hz | Boxy | Narrow cut Q 1.5–3, −1 to −3 dB |
| 1–3 kHz | Vocal presence | Rarely boosted on bus; usually dynamic-EQ |
| 2–5 kHz | Harshness | −0.5 to −1.5 dB cut, Q 1–2 |
| 5–10 kHz | Sibilance | Dynamic EQ or de-esser (static cuts clumsy here) |
| 10–20 kHz | Air | +0.5 to +2 dB shelf on sides (M/S) |

### Filter Topology Modes

| Mode | Latency | Pre-ring | Phase | Use |
|---|---|---|---|---|
| **Minimum-phase IIR** | Near-zero | None | Frequency-dependent group delay | General tone-shaping, low end, surgical |
| **Linear-phase FIR** | 3,000–66,000 samples (FabFilter Pro-Q 3 "Low" mode ~70 ms at 44.1 kHz) | Audible "smear" before transients (Q > 5 + boost = worst) | Identical delay all frequencies | Broad tonal shaping in mastering |
| **Natural-phase / mid-phase / dynamic-phase** | Variable | Reduced | Hybrid | Best of both — FabFilter "Natural Phase", iZotope mid-phase |

**Practical rule:** linear-phase **cuts** are safer than boosts. Pre-ringing scales with Q and boost amount. For low frequencies, prefer minimum phase or natural-phase to preserve transient feel.

### Special EQ Techniques

**Pultec EQ trick:** Simultaneous low-shelf **boost + cut at near-identical frequencies** (60–100 Hz). Creates a resonant bump-then-dip. Codify as a "Pultec Low" macro.

**Tilt EQ:** Single control rotates the spectrum around **650 Hz – 1 kHz**. Ideal as a top-level "brightness" macro.

**Dynamic EQ:** Compresses or expands a band only when threshold is crossed. **Event-driven reduction often 0.5–3 dB.** Replaces de-essers and tames moving resonances. **Preferred over static EQ** when the issue is transient or program-dependent.

**De-essing in mastering:** Treat as dynamic EQ at 5–10 kHz. Static cuts here are clumsy.

### Mid/Side EQ Patterns

| Move | Effect |
|---|---|
| Side high-shelf +1 to +2 dB above 5–10 kHz | "Air" / width |
| Side low cut below 200 Hz (or hard mono-sum below 120–200 Hz) | Center the bass image |
| Mid low-shelf cut at 100–200 Hz | Tighten mud while keeping width |
| Mid clean center buildup (vocals, kick, snare, bass) | Reduce competition with sides |

### Hard Rules

- **Never widen below 80–200 Hz** — bass must remain mono (vinyl, club PA, phone-speaker summing)
- Use **linear-phase M/S EQ** to preserve phase relationship on L-R reconstruction
- Keep cross-band curves smooth (phase distortion < 90°)
- **Prefer dynamic EQ over static EQ** for harshness/sibilance/low-mid bloom that's intermittent

### Unifying-Air Trick for Multi-Genre Albums

Apply the same gentle high-shelf (~+1.5 dB above 8 kHz, gentle Q) on all tracks across an album as a "unifying air." Eliminates "different studio" tonal mismatches without forcing different-source tracks to sound identical.

---

## 9. Compression Settings

Mastering compression is **glue, not control.**

### Broadband Bus Settings

Per Waves (Yoad Nevo), Joey Sturgis Tones, Splice, eMastered, Jonathan Wyner:

| Parameter | Setting |
|---|---|
| Ratio | **1.1:1 to 2:1** — Yoad Nevo: *"most mastering engineers use… typically 1.25:1 or 1.5:1 — rarely anything more than 2:1"* |
| Threshold | High |
| Gain reduction | **0.5–2 dB transparent mode; 1–3 dB max** — Splice/eMastered: *"no more than 2 dB"*; Joey Sturgis Tones: *"1 to 2 dB max"* |
| Attack | **10–80 ms** (10–30 ms typical; 50–80 ms preserves transients on dense material) |
| Release | **50–300 ms or auto / program-dependent** |
| Knee | Soft |
| Makeup gain | Bypass-matched (A/B at equal loudness mandatory) |
| Detection | Feedback topology preferred for mastering |

Mastering-oriented compressors are explicitly designed for low harmonic distortion and fast transient catching, but program dependency matters: too-fast time constants dull the source. A safe implementation: **low-ratio, soft-knee, auto-or-tempo-insensitive release** with hard caps on GR before the algorithm steps down or bypasses itself.

### Compressor Topology (Emulation) Choices

| Topology | Character | Reference emulations | Use |
|---|---|---|---|
| **VCA** | Fast, clean, slightly punchy attack | SSL G-bus, API 2500 | Default mastering bus tool — "glue" |
| **FET** | ~20 µs attack; colors transients | 1176 emulations | Parallel/aggression layer; rare in mastering main bus |
| **Opto** | Slow, program-dependent, smooth | LA-2A emulations | Vocals, dynamic acoustic |
| **Vari-Mu / tube** | Smoothest GR, slight upward harmonic generation | Manley, Fairchild | Default "glue" on classic/jazz/rock masters |

### Parallel ("NY-style") Compression

- Ratio: 8:1 or 10:1, fast attack
- GR: 10+ dB
- Blended back: 10–30% (15–25% common for modern metal)
- Effect: sustain + density without choking transients

### Upward Compression / Inflation Tools

- **Sonnox Oxford Inflator** — positive-bias bit emphasis; increases perceived loudness 1–3 dB without explicit GR
- **Waves MV2**
- **Klanghelm DC8C** (upward mode)

### Transient Shapers (Pre-Limiter)

- **SPL Transient Designer**
- **Boz +10 dB**
- **NI Transient Master**
- Typically ±2 dB on bus

### Manufacturer Manual References

For algorithm implementation benchmarks:
- **FabFilter Pro-C help** — modern transparent compression reference
- **Softube Weiss DS1-MK3 manual** — mastering de-esser reference
- **Softube Weiss DS5 Multiband Compressor manual** — multiband reference

---

## 10. Multiband Compression Settings

**Less is more.** Surgical tool for specific frequency-range problems, not a default insert.

Per Sean Kim's mastering guide: *"if you find yourself reaching for 4+ bands, it's usually a sign that the mix itself needs revision."*

Treat multiband and dynamic spectral control as **"secondary correctives"** — intervene only when and where the problem occurs.

### Common Crossover Setups

| Bands | Common Crossovers | Use Case |
|---|---|---|
| **2** | 120–200 Hz | Tame kick/bass vs everything else (most common) |
| **3** | 150 Hz, 5–6 kHz | Independent lows / mids / highs |
| **4** | 80, 250, 5 kHz | Surgical only — cautious use |
| **5+** | varies | Almost never in mastering |

### Per-Band Settings

| Parameter | Setting |
|---|---|
| Crossover slope | **≥18 dB/oct**, 24 dB/oct preferred for isolation |
| Per-band ratios | **1.2:1 to 2.5:1** (low 2:1–3:1, mid 1.5:1, high 1.5:1–2:1) |
| Glue/conservative ratios | 1.25:1–1.5:1 across all bands |
| Per-band attack | **5–10 ms minimum** to preserve transients |
| Per-band release | **50–150 ms** |
| **Per-band GR** | **0.5–2 dB typical** in transparent mode |
| Common bus comp on top | Ratio 1.3:1, attack 70–100 ms, release 100 ms |

### Hard Rules

- Don't split fundamental of lead vocal or main hook across bands
- For each crossover, find a point that **least obviously divides a primary instrument**
- Lower-band fast attacks kill kick punch — keep ≥5 ms
- **Always-on multiband is a smell** — should be event-engaged or genre-template-driven

---

## 11. Limiter Settings & Algorithm Internals

A brickwall limiter is a compressor with **∞:1 ratio + ultra-fast attack + lookahead**. **The single most consequential algorithm in the chain.**

### Modern Limiter Architecture (Pro-L 2 / Ozone Maximizer / Elevate / Waves L3)

1. **Input gain ("Drive")** pushes signal into the limiter
2. **Look-ahead delay** (1–5 ms) — calculates gain envelope before audio reaches gain stage
3. **Multi-stage gain reduction:** parallel fast "transient" stage + slower "release" stage — fast catches without long-term pumping
4. **Program-dependent release:** adapts to envelope
5. **Oversampling** (2×–32×) — minimizes aliasing and internal ISP from fast gain reduction
6. **True-peak limiting:** oversampled-peak detector post-GR adds smoothing so reconstructed analog never exceeds ceiling. Pro-L 2 documents ~5 ms additional latency.
7. **Channel linking:** separate **transient-link (70–90%)** and **release-link (100%)**

### One-Click Defaults

| Parameter | Value |
|---|---|
| Lookahead | 1–3 ms (default 1.2 ms) |
| Release | Auto / 50–200 ms |
| Oversampling | **4× minimum** (BS.1770 spec); 8× preferred |
| True-peak | **ON, ceiling −1.0 dBTP** (−2.0 for Amazon target or master > −14 LUFS, −2.0 for trap/808-heavy material) |
| Algorithm (default) | "Transparent" / "Modern" / "Allround" |
| Algorithm (metal/EDM) | "Aggressive" — *"works especially well on rock, metal, or pop"* per FabFilter Pro-L 2 docs |
| Algorithm (acoustic/classical) | "Safe" |
| Max bus GR | 2–6 dB; metal/EDM tolerate 6–10 dB if a soft-clipper carries 1–2 dB upstream |
| **Single-stage GR rule** | **If single-stage limiting must exceed ~2–4 dB often, consider a staged strategy or back off the drive** |

### FabFilter Pro-L 2's 8 Named Algorithms

Reference style profiles (different attack/release/saturation curves):
1. **Transparent** — universal default
2. **Punchy**
3. **Dynamic**
4. **Allround**
5. **Modern** — transparent all-purpose
6. **Aggressive** — *"especially well on rock, metal, or pop"*
7. **Bus**
8. **Safe** — for acoustic/classical/dynamic material

### Reference Limiter Implementations to Study

| Limiter | Character |
|---|---|
| **FabFilter Pro-L 2** | Transparent default, 8 algorithms, K-system/EBU R128 metering, surround/Atmos 7.1.4 — gold standard |
| **iZotope Ozone Maximizer (IRC 5)** | Multiple IRC variants; tight Master Assistant integration |
| **Newfangled Audio Elevate** | Multiband spectral limiter with 26-band psychoacoustic GR |
| **Sonnox Oxford Limiter v3** | Enhance control for transient emphasis; analog feel |
| **Brainworx bx_limiter XL** | Saturated character |
| **Waves L2 / L3 / L3-LL** | Industry standards; L3 multiband, L2 broadband; L2 has audible character (often used as flavor) |
| **Sonnox Inflator** | Saturator/upward processor — 1–3 dB perceived loudness without explicit GR |
| **NUGEN Audio ISL** | True peak limiter benchmark; ITU-R BS.1770 compliant |
| **LoudMax** (free) | Basic but solid |

### Hybrid Clipping-Before-Limiting (Modern Loud-Master Technique)

- **Soft clipper** before brickwall: shaves worst transients invisibly
- Drive **0.5–2 dB** into clipper
- Limiter then needs 1–3 dB less work; stays "Transparent" instead of "Aggressive"
- Net: **1–2 dB more apparent loudness for the same audible damage**

**Soft clipper plugins:**
- Kazrog KClip 3
- SIR StandardCLIP
- Tokyo Dawn Limiter 6 (clipping stage)
- FabFilter Saturn 2 (clip mode)
- Newfangled Saturate

**Alternative workflow (Black Ghost Audio):** Clipper *after* limiter at −1 dBTP for true-peak catch. Pick one approach per template — not both.

### Distribution-Aware Limiting

Make the limiter **distribution-aware**:

| Mode | Priority | Default ceiling |
|---|---|---|
| **Streaming-safe** (default) | True-peak protection + codec preview | −1 dBTP (−2 for trap/Amazon/loud) |
| **CD-only / in-house reference** | Tighter sample-peak; no lossy encode expected | Slightly tighter allowed |
| **Broadcast (EBU R128)** | Chain compliance | −1 dBTP (typical chain limit) |

### True-Peak / Inter-Sample Peak Detection

Why this matters: lossy codecs (AAC, Opus, MP3) reconstruct waveforms that can exceed sample-domain peaks by **+0.5 to +3 dB**. A 0 dBFS sample-peak master routinely measures **+1 to +2 dBTP after AAC**, audibly distorting on listener devices.

- **BS.1770 spec:** 4× oversampling minimum, 48-tap polyphase FIR (12 taps per phase)
- **High-end:** 8×–32× linear-phase Kaiser-windowed FIR
- **Cheap (avoid):** parabolic interpolation, 2× linear undershoots by up to 1.5 dB

---

## 12. Dithering & Noise Shaping

### The Rule

**Dither ONCE, at the absolute end of the chain, when reducing bit depth. Never twice. Never EQ/compress/limit after.**

If SRC is required, **SRC before dithering** — never the reverse.

### Dither Types

| Type | PDF | Amplitude | Spectral character | Notes |
|---|---|---|---|---|
| **RPDF** | Uniform | 1 LSB peak-to-peak | White | Modulates noise floor (audible) — **avoid for final** |
| **TPDF / Type 2** | Triangular (sum of 2 independent uniforms in [-1, +1 LSB]) | 2 LSB peak-to-peak | White, 4.77 dB louder than RPDF | **Industry default** — eliminates quantization noise modulation entirely. Safe through subsequent processing. Practical fallback when MBIT+ unavailable. |
| **Gaussian** | Normal | σ = 1 LSB | White | Analog-like; slightly louder noise floor |
| **High-pass TPDF (noise-shaped)** | Triangular feedback-filtered | varies | Noise moved to less audible HF (~16 kHz+) | ~14 dB perceived SNR improvement; higher peak noise; worse under further processing |

**Dither preference order** (per iZotope guidance): MBIT+ when available → TPDF/Type 2 as practical fallback → RPDF only for intermediate processing (never final).

### Noise Shaping Algorithm Families

| Algorithm | Curve / character | Use |
|---|---|---|
| **POW-r 1** | Minimal shaping | High-dynamic music |
| **POW-r 2** | Moderate shaping | General music |
| **POW-r 3** | Aggressive: **dip 3–4 kHz, +35 dB lift above 16 kHz** | Large-dynamic orchestral |
| **iZotope MBIT+** | Proprietary psychoacoustic shaping | Levels: **None / Light / Medium / Aggressive / Ultra** + peak-limit option (suppresses spurious peaks below −60 dBFS) |
| **Apogee UV22 / UV22HR** | Flat 1–18 kHz at ~5 dB below flat TPDF, **+30 dB lift above 18 kHz** | Proprietary "high frequency placement" |
| **FabFilter Pro-L 2 built-in** | Three TPDF curves (basic + two shaped) | Integrated with limiter |
| **Sony Super Bit Mapping (SBM)** | Proprietary | 1990s–2000s CD masters |
| **Lipshitz-Vanderkooy** (1991 AES) | 9-tap minimum-phase FIR, ~−18 dB perceived noise | Classic academic reference |
| **Wannamaker / Gerzon F-weighted, E-weighted** | Fletcher-Munson / ISO 226 derived | Psychoacoustic-curve-based |

### Critical Caveat: Shaping Can Raise Peaks

Stronger noise shaping can **raise the true peak** of the output. After applying any shaped dither, **re-measure true peak** and verify against ceiling. This is one reason flat TPDF is the safer streaming default.

### When to Use What

| Output | Dither | Noise Shaping |
|---|---|---|
| **24-bit streaming (most)** | Flat TPDF if dithering at all — mostly cosmetic (noise floor at −144 dBFS below any analog noise) | No |
| **16-bit CD** | **TPDF mandatory** | Yes recommended (POW-r 2 or MBIT+ Light); POW-r 3 / MBIT+ Ultra for orchestral |
| **32-bit float intermediate** | **Never dither** (float quantization is non-uniform) | No |

### Critical Warning: Noise-Shape × Lossy Codec Interaction

**Avoid noise-shaped dither if the master will be further lossy-encoded** — codecs reinterpret shaped noise and can produce audible artifacts:

> A POW-r 3 / MBIT+ Ultra master that sounds great on CD may produce audible swirling after Ogg Vorbis transcode. **When in doubt, flat TPDF.**

Specifically:
- **No noise-shaping on classical** — extra HF noise audible on quiet passages
- **Streaming delivery (any platform):** flat TPDF safer than shaped
- **CD-only delivery:** noise shaping safe and beneficial

---

## 13. Stereo Imaging & M/S Processing

### Mid-Side Math

```
Encoding:
  M = (L + R) / √2     (√2 normalization preserves loudness through encode/decode)
  S = (L − R) / √2

Decoding:
  L = (M + S) / √2
  R = (M − S) / √2
```

Unnormalized matrices compensate at decode.

### Mastering Applications

- **M-channel compression:** glue center elements (vocal, kick, snare, bass) without pumping reverb tails
- **S-channel compression:** control reverb/ambience without affecting center punch
- **M-channel EQ:** clean center muddiness, presence boost on vocals
- **S-channel EQ:** air on sides (high shelf), remove muddy reverb (low cut)
- **Width control:** gain on S relative to M (>1.0 = widen, <1.0 = narrow, 0 = mono)

M/S is a **surgical tool**, not a party trick. Prefer M/S EQ or selective dynamics over blind widener algorithms.

### Mono-Compatibility & Bass Summing

| Material | Mono-sum frequency |
|---|---|
| Streaming masters | **120–150 Hz** |
| Vinyl masters | 200–300 Hz |
| EDM | Below 100 Hz strictly |
| Djent / downtuned guitars | 150–200 Hz mandatory (guitar fundamentals 80–150 Hz) |
| **General default** | **80–150 Hz** (constrained or mono) |

Implementation: M/S EQ side low-cut at the crossover frequency, or dedicated Bass Mono plugin.

### Width Control Rules

- **Side level boost ≤ +3 dB** to avoid mono-collapse
- Side high-shelf for widening preferred over side gain
- **Haas / all-pass-based wideners cause comb-filtering in mono — avoid in mastering**
- Mid boost narrows; tightens center
- Prefer **small side-only EQ shelves** or mid-only control over aggressive widening

### Correlation / Vectorscope Rules

| Reading | Meaning |
|---|---|
| Correlation +1 | Mono |
| Correlation 0 | Uncorrelated stereo |
| **Correlation ≥ 0 sustained** | Required for stereo mastering |
| Correlation < 0 brief excursions | OK on transients/effects |
| **Correlation < 0 sustained** | Red flag — phase problem |
| Vectorscope shape | Roughly oval, slightly wider than tall; vertical = mono; horizontal = anti-phase |

---

## 14. Saturation & Harmonic Enhancement

### Purpose

Add even/odd harmonic content for perceived loudness, "warmth," "glue" without measurable gain reduction.

### Saturation Types

| Type | Harmonics | Character | Plugin emulations |
|---|---|---|---|
| **Tape** | Even + odd, program-dependent soft compression, HF loss (head bump + LPF), wow/flutter | Reference: **15 ips, +3 dBu, ≤2 dB needle drive** | UA Studer A800, IK Tape Machine, Softube Tape, Kazrog True Iron |
| **Tube** | Predominantly even-order | "Warmth/openness"; 1–3% THD | Various tube emulations |
| **Transformer** | 3rd-order on small drives, 5th+ on harder | "Iron" sheen | Rupert Neve, Manley emulations |
| **Multiband saturation** | Saturate HF only | "Air" without harshness | Brainworx bx_saturator V2, Soundtoys Decapitator (tone control), FabFilter Saturn 2 |
| **Exciters / enhancers** | Psychoacoustic + harmonic; even harmonics 2–8 kHz from low-mid input | Adds "presence" | Aphex Aural Exciter, SPL Vitalizer, Sonnox Oxford Inflator |
| **Console emulation** | Crosstalk + subtle EQ + summing harmonics | "Analog bus" feel | Various |

### Drive Amounts

- **Bus saturation: ≤2% THD** added
- **Multiband HF saturation: 3–5%**
- Applied broadband or band-split (e.g., saturation only above 5 kHz)
- Typically **before the limiter** to add perceived loudness without further GR

### Universal Default

**Off in universal mode.** Engage on dense/loud template or via user override. Re-check true peak and codec preview immediately after engaging.

---

## 15. Sample Rate, Bit Depth, Sample Rate Conversion

### Recommended Internal Precision

**32-bit or 64-bit float throughout processing chain.** Quantize only at output.

### Core Rule

**Master at native resolution, export at native resolution unless a deliverable requires otherwise, and avoid unnecessary SRC.** Apple Digital Masters asks for "highest native sample rate available"; Spotify says **don't downsample tracks mastered above 44.1 kHz before delivery**.

If a separate deliverable does require SRC: do it **once**, with a high-quality offline SRC, and **only then make the bit-depth reduction decision** (SRC → dither, never reverse).

### Sample Rate / Bit Depth Strategy

| Target | Recommended Master SR | Bit Depth | Notes |
|---|---|---|---|
| Native 24-bit WAV/AIFF/FLAC at 44.1–192 kHz | Keep native SR through mastering; export one native lossless master for distribution | 24-bit | |
| Native 16-bit source only | Process internally in float; if final 16-bit, dither on export; **don't "bit-pad" fake 24-bit source claims** | 16-bit out | |
| CD (Red Book) | 44.1 kHz | 16-bit + dither | Final SRC from native session rate |
| Streaming (most) | Native (44.1+) | 24-bit | Universal compatibility |
| **Apple Digital Masters** | **96 kHz preferred** (44.1/48/88.2/96/176.4/192 kHz accepted) | **24-bit minimum** | Apple explicit: *"if you create your masters at 24-bit, 44.1 kHz, you should not upsample to 96 kHz"* |
| Apple Hi-Res Lossless | 88.2/96/176.4/192 kHz | 24-bit | Native rate only |
| Broadcast (TV / EBU R128) | 48 kHz | 24-bit | Video sync |
| Atmos | 48 kHz | 24-bit LPCM | BWF ADM container |
| Vinyl premaster | **88.2 or 96 kHz** | 24-bit | Headroom for cutting EQ |
| Hi-res download | 96 / 192 kHz | 24-bit | Marketing tier |
| Distribution to aggregator | Native session rate | 24-bit | **Never deliver 32-bit float** — most aggregators reject; **never MP3** — double-encoding is audibly destructive |

**Modern 2026 default for new sessions:** 24-bit / 48 kHz; 24-bit / 96 kHz if source mixes are at that rate.

### SRC Algorithm Quality Ranking

1. **SoX VHQ (soxr) / iZotope SRC** — polyphase, 200+ tap FIR, near-perfect impulse response
2. **r8brain free / SSRC** — high-quality open implementations
3. **libsoxr / libsamplerate (SRC)** — good open-source defaults
4. **"Giant FFTs" approach** — modern offline SRC design (JAES, March 2023)
5. **Audacity / FFmpeg default** — adequate but audible artifacts on critical material
6. **Linear interpolation / nearest** — **never use for mastering**

### SRC Quality Requirements

- Stopband attenuation **≥ −100 dB**
- Passband ripple **≤ ±0.01 dB**
- Transition band 0.9·fs/2 to fs/2
- Anti-aliasing rolls off sharply between 20 kHz and 24 kHz at 48 kHz target
- At 96 kHz source, gentler filter slope possible (rolloff above 40 kHz is outside hearing)

### Brickwall Filter Note

Theoretical brickwall low-pass introduces pre-ringing and unnatural transient behavior. Modern minimum-phase SRC reduces this at the cost of phase distortion; linear-phase SRC preserves phase at the cost of pre-ringing.

---

## 16. Lossy vs Lossless Source Workflows

A mastering app should treat **lossless** and **lossy** intake very differently. Codec artifacts are irreversible; further clipping/SRC/encoding worsens them.

### Workflow Per Source Type

| Source type | Recommended workflow | Why |
|---|---|---|
| **WAV / AIFF / FLAC lossless** | Full mastering path at native resolution → derive secondary outputs from lossless final | Best source fidelity; aligns with platform ingest guidance |
| **MP3 / AAC intake, original lossless unavailable** | Decode once to float for processing → lighter corrective moves → leave **more true-peak headroom** → preview the final lossy encode → export a lossless master internally → encode the consumer lossy file **once** | Codec artifacts are irreversible; further clipping/SRC/encoding can worsen overshoot and edge artifacts |
| **MP3 / AAC intake, original lossless available elsewhere** | Request / reuse the lossless original; **bypass the lossy source entirely** | Prevents lossy-to-lossy degradation; avoids mastering "on top of damage" |

### Core Rule for Lossy Intake

> **Decode once for analysis/processing. Encode once for final consumer distribution.**

Never silently transcode lossy input multiple times during preview or export. Each transcode compounds codec artifacts.

### Specific Adjustments for Lossy Intake

- **More true-peak headroom:** ceiling at **−2 dBTP** instead of −1 (lossy source already pushes ISPs)
- **Lighter corrective moves:** the codec artifacts are baked in; aggressive EQ amplifies them
- **Codec preview mandatory:** verify the consumer-encoded output isn't compounding damage
- **No further bit-depth reduction without strong justification**
- **Bypass stages by default:** only engage processing that demonstrably improves translation

### Pre-Master Intake QC (Lossy Detection)

The app should detect and flag at intake:
- File type, sample rate, bit depth, channel count
- **Lossy/lossless status** (and if lossy, codec + bitrate)
- **Clipped-sample count**
- DC offset
- Already-overlimited indicators (very low LRA, very low PSR throughout)

Show user the intake report **before** processing. Surface "this is a lossy file" and "this file is already heavily limited" as warnings.

---

## 17. Tonal Balance & Spectral Targets

### Pink Noise Reference

**−3 dB/octave** spectrum (constant power per octave) is the historical "neutral" target. Modern productions often target a **4.5 dB/oct** slope (flatter mid response, brighter top).

### Genre-Derived Target Curves

Pulled from corpus analysis (50–100+ commercial masters per genre, smoothed in 1/12-octave bands, post-K-weighted normalization). Tools that ship genre target curves:

- **iZotope Tonal Balance Control** — bundled genre target curves
- **iZotope Audiolens** — analyzes audio playing in any application/streaming service (reference-capture tool)
- **iZotope Ozone Master Assistant** — builds EQ chain to nudge master into target curve
- **FabFilter Pro-Q 3/4 Match EQ** — analyzes a reference clip and generates corrective EQ
- **Mastering The Mix Reference** — real-time level-matched A/B
- **ADPTR Metric AB** — same category
- **HoRNet Reference**

For a mastering program building its own curves: train on **50–100 references per genre**, smoothed.

Standard genre categories:
- Pop / Modern
- Hip-hop / Trap
- Rock / Indie
- EDM / Dance
- Acoustic / Folk
- Orchestral / Cinematic
- Jazz / Vocal

### Spectrum Analyzer Slope Choices

| Slope | Display character | When used |
|---|---|---|
| Flat (0 dB/oct) | Exaggerates low end | Avoid as default; misleading |
| 3 dB/oct | Classic pink-noise reference | Most analyzers' default |
| 4.5 dB/oct | Flatter modern pop reference | Modern productions look "balanced" here |

Choice affects how curve **looks** but not the audio itself.

---

## 18. Reference Monitoring & K-System

### SPL Calibration

| Reference | SPL | Weighting | Source signal |
|---|---|---|---|
| Cinema / large studio (Dolby) | **83 dB SPL** per speaker | C-weighted slow | Pink noise at −20 dBFS RMS per channel |
| Bob Katz mastering reference | 83 dB SPL per speaker | C | Pink noise at −20 dBFS RMS |
| **AES music-mastering reference** | **79 dB SPL** per speaker | C | Pink noise at −20 dBFS RMS |
| Small studio practical | 73–76 dB SPL | C | Per Katz K-12 calibration |
| Bedroom studio compromise | ~79 dB SPL | C | Room-size adjusted |

Required tool: SPL meter with **C-weighted filter**, slow response.

### Bob Katz K-System

Integrated metering + monitoring + leveling protocol (JAES 2000):

| Scale | Reference (0 on meter = LUFS) | Headroom | Genre target |
|---|---|---|---|
| **K-12** | −12 LUFS at 0 VU | 12 dB | Broadcast / heavily-compressed |
| **K-14** | −14 LUFS at 0 VU | 14 dB | Rock / pop / mainstream |
| **K-20** | −20 LUFS at 0 VU | 20 dB | Classical / wide-dynamic |

When monitors are K-calibrated, 0 dB on the meter corresponds to **83 dB SPL** (single channel) or **85 dB SPL** (two channels).

The K-System emphasizes **average (RMS-related) levels over peaks**, because perceived loudness correlates with RMS more closely than with peak.

### QC Listening Levels

Test mastering decisions at **three SPL levels: ~65, 75, 85 dB.** Loudness perception varies with playback level (Fletcher-Munson); a master that balances at 85 dB may sound bass-light at 65 dB.

### Modern Software Equivalent

Present LUFS-relative meters with target reference lines per genre. Show M / S / I with color zones per EBU Tech 3341.

---

## 19. Delivery Format Specifications

### Apple Digital Masters

| Requirement | Value |
|---|---|
| Bit depth | **24-bit minimum** |
| Sample rate | 44.1 kHz minimum; **96 kHz preferred**; accepted: **44.1 / 48 / 88.2 / 96 / 176.4 / 192 kHz** |
| **Do NOT upsample** | Apple explicit: *"if you create your masters at 24-bit, 44.1 kHz, you should not upsample to 96 kHz"* |
| **Do NOT bit-pad** | Don't fake 24-bit from 16-bit source |
| True-peak ceiling | **−1 dBTP** |
| Delivered encoding | Apple encodes to AAC-LC 256 kbps |
| Required compliance | No clipping; no inter-sample peaks above ceiling |
| Internal pipeline | Apple uses 32-bit float intermediate; dither + SRC documented |
| Metadata required | ISRC, ISWC, composer credits |

**Apple Hi-Res Lossless:** 24-bit at 88.2/96/176.4/192 kHz. Native rate only — no upsampling.

**Apple Digital Masters tooling (ships with macOS):**

| Tool | Function |
|---|---|
| **`afconvert`** | CLI codec wrapper using Apple's actual AAC encoder (= the Apple Music encoder) |
| **`afclip`** | CLI intersample clip checker |
| **`AURoundTripAAC`** | AU plugin — A/B source vs encoded AAC in real time |
| **Apple Digital Masters droplets** | Drag-and-drop GUI wrappers |

These let you preview exactly what Apple's encoder does to a master.

### CD / Red Book / DDP

| Spec | Value |
|---|---|
| Format | **16-bit / 44.1 kHz / stereo PCM** |
| Max duration | 79:57 audio |
| Delivery | **DDP 2.00 image** (folder, not single file) |
| DDP folder contents | `IMAGE.DAT`, `PQDESCR`, `DDPID`, `DDPMS`, `CRC` (MD5 checksum) |
| PQ subcodes | Track starts, indices, gaps (default 2 sec), pre-emphasis flag |
| ISRC per track | 12 chars: `CC-XXX-YY-NNNNN` |
| UPC/EAN | Per disc, 12-digit |
| CD-Text (optional) | Album title, artist, track titles |
| Inter-track spacing | 2 s default; 0–0.5 s for crossfade/album-listening (via PQ codes) |
| Tools | HOFA CD-Burn.DDP.Master, Sonoris DDP Player (free), Steinberg WaveLab, SADiE |

### Broadcast Wave (BWF) — EBU Tech 3285

- WAV extension; adds chunks for metadata
- **`<bext>` chunk** — broadcast-audio extension; origination data, timecode, UMID
- **`<iXML>` chunk** — production metadata
- **`<axml>` chunk** — EBU Core XML (EBU Tech 3352 — ISRC embedding)
- Used for: broadcast delivery, archival, post-production interchange, **Atmos ADM container**

### Vinyl Premaster

| Spec | Value |
|---|---|
| Sample rate | **24-bit / 48 or 96 kHz** (some cutters prefer 88.2) |
| Bass treatment | **Mono below 200 Hz** (mandatory — eccentric/vertical groove modulation); 200–300 Hz mono-sum for vinyl (vs 120–150 for streaming) |
| True peak | No brickwall; **−3 to −6 dB headroom for cutter** |
| Dynamic range | **Higher than streaming** — LRA ≥ 8 LU typical |
| Loudness | −10 to −14 LUFS |
| Side length | **≤ 22 min/side for 12″ 33 RPM (loud)**; longer = quieter cut |
| Side balancing | Balance side lengths **within 1 minute** — softer material on the longer side |
| De-essing | 5–10 kHz tame for cutter head safety |
| Low-pass filter | **>18 kHz** (cutter head protection) |
| Special EQ | LFX / elliptical EQ for low-end mono summing |
| Pre-emphasis | Off (RIAA EQ applied at cut/play by hardware, not encoded in source) |
| Stereo info below 200 Hz | Should be 6 dB lower than mono info; stereo never louder than half of mono |
| Dither | **Don't dither pre-vinyl** |

**Vinyl mastering is NOT a one-click problem.** Generate a *pre-master* file and spec sheet for the specialist cutting engineer.

### RIAA Pre-Emphasis (For Cutting)

During vinyl cutting, high frequencies are boosted and low frequencies attenuated; reversed during playback with corresponding RIAA preamp. Done by cutting hardware, not master file.

### Dolby Atmos / Apple Music Atmos

| Spec | Value |
|---|---|
| Format | **BWF ADM** (Audio Definition Model in Broadcast WAV) |
| Sample rate | **48 kHz** |
| Bit depth | **24-bit LPCM** |
| Integrated loudness | **−18 LKFS** (LUFS) per BS.1770-4 |
| True-peak ceiling | **−1 dBTP** per BS.1770-4 |
| Structure | Channel-based bed + objects |
| Rendering | Dolby Atmos Renderer |

### MQA — End of Life

- MQA Ltd appointed administrators **3 April 2023**
- Tidal announced discontinuation **June 2024**, removed MQA playback **July 2024**
- **Ignore for new products**

### File Format Notes

- **WAV 24-bit** is the lingua franca; FLAC accepted everywhere
- **ALAC** only via Apple's ADM workflow
- **Never deliver 32-bit float** — most aggregators reject
- **Never deliver MP3** — double-encoding to Ogg/AAC is audibly destructive

### Manufacturer Manual for Delivery Workflow

**Steinberg WaveLab help** — reference for DDP creation, metadata embedding, loudness analysis, final render workflows. Pete Lyman's published workflow uses WaveLab end-to-end.

---

## 20. Lossy Codec Considerations (AAC, Opus, MP3, Vorbis)

### How Streaming Actually Works

Every streaming platform applies LUFS normalization **and then** encodes to AAC/Opus, in that order. Both stages can affect peak levels.

### Codec Behavior Summary

| Codec | Algorithm | ISP behavior on cymbals/transients |
|---|---|---|
| **AAC-LC 256** (Apple) | MDCT | Can spike intersample peaks on cymbals/sibilance |
| **Ogg Vorbis** (Spotify desktop/mobile) | MDCT + floor/residual | Sensitive to HF density and brickwall artifacts |
| **Opus** (YouTube / SoundCloud free) | Hybrid CELT/SILK | Transparent at 128 kbit/s; harsh transients can pump |
| **MP3** (Deezer Free, legacy) | Subband + MDCT | ISPs routinely **1–3 dB above sample peaks** |

### Inter-Sample Peak Behavior After Lossy Encoding

Lossy codecs discard "perceptually irrelevant" information and reconstruct via mathematical models at playback. Reconstruction introduces its own inter-sample behavior:

- A file measuring **+0.3 dBTP before encoding** can measure **+1.0 dBTP or higher after AAC**
- A master delivered at **0 dBFS sample-peak with ISPs at +0.5 dBTP** comes out of AAC with audible hardness/brittleness on transients
- Lossy codecs add **0.3–1.5 dB of intersample peak** in practice; trap 808s create severe ISPs
- Spotify recommendation: **−1 dBTP quieter masters, −2 dBTP for masters > −14 LUFS**
- Apple Digital Masters: **−1 dBTP** verified with `afclip`
- Amazon Music: **−2 dBTP** (stricter; Alexa/Echo ISP-prone)

### Codec Preview / Audit Workflow

Build into the mastering chain:
- **AAC round-trip** preview (Apple `AURoundTripAAC` AU; Sonnox Pro-Codec)
- **Ogg Vorbis preview** for Spotify (libvorbis; MeterPlugs Loudness Penalty)
- **Opus preview** for YouTube (libopus)
- **ISP detection** after each simulated encode
- **Null test** against original
- Compare bus output before and after lossy round-trip

### Encoder Caveats

- **`afconvert` is the actual Apple Music AAC encoder** — preview is reliable
- **Spotify's Ogg Vorbis encoder is not publicly distributed** — Sonnox Pro-Codec or open libvorbis preview is approximate, not exact
- **YouTube uses both Opus and AAC depending on context** — preview both

### MP3 / AAC Encoder Padding (Gapless Concatenation)

- **MP3 encoders add priming/padding** — LAME `--nogap` mode + proper Xing headers handle this
- **AAC** uses **`iTunSMPB` priming/padding metadata** for gapless
- **WAV / FLAC / ALAC** have zero padding — true gapless natively

---

## 21. Album-Level Mastering Concepts

### Album vs Track Normalization

| Platform | Album mode? | Behavior |
|---|---|---|
| **Tidal** | Always on (broadly across product) | Album-relative levels preserved everywhere |
| **Apple Music** | Yes, when songs play in sequence | Sound Check applied per-album in album mode |
| **Spotify** | Yes, when songs play in sequence as an album | Single normalization for the whole album |
| **YouTube Music** | Per-track | No album mode |
| **Streaming playlists** | Per-track on all platforms | Album relationships broken in playlists |

**Implication:** Mastering for "album mode" means preserving inter-track relative loudness. **Don't crush quieter tracks to compete with louder ones** — normalization will undo the effort while damaging dynamics permanently.

### Per-Track Album Gain Offsets

Build a per-track gain offset table on top of per-track mastering. After each track is mastered to its own genre-appropriate target, apply small static gain trims (typically **±1–3 dB**) to taste during album sequencing. Persist as project state.

### Inter-Track Cohesion Techniques

1. **Match Short-Term LUFS climaxes**, not integrated loudness. If each track's loudest moment lands near, say, −10 LUFS Short-Term, tracks feel related even with very different integrated levels.
2. **Tonal-balance match** the high and low ends (same "unifying air" shelf above 8 kHz; matched bass tightness).
3. **Reference-match per cluster:** for genre-spanning albums, one reference per genre cluster, not one reference across all.

### Continuous Album / Gapless Delivery

Historical precedent for continuous albums (Pink Floyd *Dark Side of the Moon*, Daft Punk *Discovery*, Beatles *Abbey Road* Side B):

- **Continuous master at production time** (one long file)
- **Sample-accurate cue point splits** at track boundaries
- Source files start and end inside what would be a continuous waveform (no padding)

For modern delivery:
- Render entire album as **one master WAV** at session SR / 24-bit
- Optionally export sample-accurate track-split version
- Embed each split file with correct metadata (`iTunSMPB` for AAC; nothing for WAV/FLAC/ALAC)
- Streaming platforms support gapless when source files are correctly delimited

### Album Package Deliverables

For label / client delivery, often include:
- **Final master** (primary deliverable)
- **Alt versions** as requested:
  - Mix master (un-mastered mix)
  - Vocal-up versions
  - Instrumental / no-lead-vocal variants
  - TV mix (no vocal)
  - Acapella

Reference AES/Recording Academy delivery recommendations for **naming/versioning logic**.

---

## 22. Metadata Standards

### Required IDs

| ID | Format | Scope | Notes |
|---|---|---|---|
| **ISRC** | 12-char `CC-XXX-YY-NNNNN` | Per individual recording | Permanent; new one on re-release kills streaming history. usisrc.org (US) or IFPI (intl). Distributors assign free if no registrant code. Follow **IFPI ISRC Handbook 4th Edition (2021)** assignment principles. |
| **UPC/EAN** | 12-digit | Per release (album/EP/single) | One UPC per album product, multiple ISRCs (one per song) |
| **ISWC** | International Standard Work Code | Per composition | Required for Apple Digital Masters |

### Embedding Standards by Container

| Container | Tag system | Notes |
|---|---|---|
| **WAV** | RIFF INFO chunks; some tools also support ID3 | Limited; prefer BWF |
| **BWF** | bext, iXML, axml (EBU Core XML) | Per EBU Tech 3285 (BWF) and Tech 3352 (ISRC) |
| **FLAC / OGG** | Vorbis comments | Native, well-supported |
| **MP3** | **ID3v2.4** | Most universal for distribution |
| **AAC / MP4** | **MP4 atoms** | iTunes-style metadata |

### Architecture Rule

**Separate audio rendering from metadata writing.** Keep a deterministic metadata manifest per export. Don't bake metadata writing into the audio pipeline — write metadata in a discrete post-render step using the manifest.

### Standard Fields

Title, artist, album, album artist, track number, disc number, ISRC, UPC/EAN, ISWC, composer credits, genre, release year, explicit flag, artwork, notes.

**Caveat:** embedded loudness metadata (ReplayGain tags) mostly doesn't pass through distributors; don't rely on it surviving.

### Spacing / Fades

| Item | Setting |
|---|---|
| Inter-track CD spacing | 2 s default; 0–0.5 s for crossfade/album-listening |
| Streaming gapless | Distributor handles; embed if available |
| Fade-ins | 5–50 ms unless artistic |
| Fade-outs | Programmed inaudible at **−60 dBFS within 1–3 s** |

---

## 23. Tools & Reference Meters

Modern QA / reference set:

### Spectrum Analyzers
- **Voxengo SPAN** (free)
- **iZotope Insight 2**
- **FabFilter Pro-Q 3/4 built-in**
- **Toneboosters EQ Magnitude**

### Loudness Meters (LUFS / EBU R128 / ATSC)
- **Youlean Loudness Meter 2** (free; EBU R128/ATSC/streaming presets)
- **Klangfreund LUFS Meter**
- **Waves WLM Plus**
- **NUGEN VisLM-H2**
- **MeterPlugs LCAST / Dynameter**
- **FabFilter Pro-L 2 built-in**

### Phase / Correlation / Vectorscope
- **SPAN / Insight built-in**
- **Voxengo Correlometer**
- **Brainworx bx_meter**
- **iZotope Insight vectorscope**

### Dynamic Range / PLR / PSR
- **TT Dynamic Range Meter** (legacy DR scale)
- **MasVis** (open-source)
- **MeterPlugs Dynameter** (Shepherd co-created)
- **Loudness Penalty Analyzer** (Ian Shepherd's per-platform-playback simulator)

### Reference Tools (Real-Time Level-Matched A/B)
- **Mastering The Mix Reference**
- **ADPTR Metric AB**
- **HoRNet Reference**
- **iZotope Audiolens** (captures from any application playing audio)

### Codec Preview
- **Sonnox Pro-Codec** (all major codecs)
- **MeterPlugs Loudness Penalty**
- **Apple AURoundTripAAC** (Apple's actual encoder)

### True-Peak Limiters (Benchmark References)
- **FabFilter Pro-L 2** (gold standard)
- **iZotope Ozone Maximizer**
- **NUGEN Audio ISL**
- **Newfangled Audio Elevate**
- **Sonnox Oxford Limiter v3**
- **Waves L2 / L3**

### DAW / Editor (Mastering-Specific)
- **Steinberg WaveLab** — DDP, metadata, loudness analysis, final render workflows
- **Sonoris DDP Creator**
- **HOFA CD-Burn.DDP.Master**
- **SADiE** (professional / broadcast)

---

## 24. AI / Algorithmic Mastering Architecture

Reference architecture for systems like LANDR, eMastered, CloudBounce, BandLab Mastering, AI Mastering, iZotope Ozone Master Assistant.

**The AI/ML is in parameter selection, not waveform processing.** None of the shipping products are end-to-end neural networks producing waveforms.

### Five-Stage Pipeline

**Stage 1 — Analysis:**
- Spectral envelope estimation (FFT, 1/12 or 1/24 octave bands, smoothed)
- LUFS-I, LUFS-S, LUFS-M, LRA, PLR, PSR
- Crest factor, peak-to-RMS ratio
- Stereo correlation per frequency band
- Transient density (onset detection)
- Tempo, key (optional)

**Stage 2 — Classification:**
- Genre classification (CNN on mel-spectrogram, or hand-coded features + GBM is sufficient for "metal/EDM/acoustic/pop/hip-hop/classical")
- **Adaptive mode classification** based on measurements, not just genre tags (see §5)
- Sometimes: production style, era

**Stage 3 — Target selection:**
- Pull genre-appropriate spectral target curve
- Pull target LUFS (typically −14)
- Pull target dynamics (LRA, PSR)

**Stage 4 — Chain parameterization** (where ML provides value):
- Map (analysis, target) → DSP parameters
- EQ delta = target curve − measured curve (smoothed, GR-limited)
- Compression ratio derived from current vs target LRA
- Limiter threshold derived from target LUFS
- Iterative gain matching loop

**Stage 5 — Processing:**
- Standard DSP chain (EQ → comp → multi-band → limiter → dither)
- All conventional algorithms; the "AI" is parameter selection

### Reference Matching (Style Transfer)

User provides reference track; system computes spectral and dynamic delta between input and reference, biases parameter selection toward matching. **Not** neural-waveform style transfer — targeted DSP toward a target curve.

### Commercial Implementation Specifics (as publicly described)

| System | Behavior |
|---|---|
| **LANDR** | Cloud render through genre-specific models trained on large mastered-track corpus; presents three intensity levels |
| **eMastered** | Reference-based mastering; user adjusts bass/treble/stereo/comp |
| **CloudBounce** | Per-genre presets |
| **iZotope Ozone Master Assistant** | Exposes underlying modules for manual edit — the only meaningful "tweakable AI" path on the market |
| **BandLab Mastering** | Free, algorithmic, black-box |

### Open-Source Reference Implementation

**Matchering 2.0** — matches RMS, FR, peak amplitude, stereo width to a reference. Built-in brickwall limiter. Note: opinionated; for genre-spanning albums, apply per-cluster.

### Honest Positioning

**All AI mastering products lose to a skilled human** on material requiring taste judgments. They excel at:
- Streaming demos
- Lo-fi releases
- Casual / personal-project finishing
- Batch normalization to spec

They struggle at:
- Classical / jazz nuance
- Translation across difficult playback systems
- "Sound of the record" decisions
- Anything requiring artist conversation

Position accordingly.

---

## 25. Academic / AI Mastering Literature

Papers worth implementing or referencing for any ML personality layer:

- **Martínez Ramírez, M. A.; Reiss, J. D.** — *"End-to-end equalization with convolutional neural networks."* DAFx-18, 2018. CNN learns to apply EQ given paired input/target audio.

- **Mimilakis, S. I. et al.** — *"Deep Neural Networks for Dynamic Range Compression in Mastering Applications."* AES Convention 140, 2016. Predicts per-critical-band compression coefficients from filter-bank decomposition.

- **Martínez Ramírez, M. A.; Wang, O.; Smaragdis, P.; Bryan, N. J.** — *"Differentiable Signal Processing with Black-Box Audio Effects."* ICASSP 2021. **arXiv:2105.04752.** Trains a deep encoder to drive non-differentiable FX plugins using SPSA gradient approximation. Explicitly demonstrates **automatic music mastering** with results *"comparable to a specialized, state-of-the-art commercial solution for music mastering."* **Single most directly relevant paper for this project.**

- **Steinmetz, C. J.; Bryan, N. J.; Reiss, J. D.** — *"Style Transfer of Audio Effects with Differentiable Signal Processing."* **arXiv:2207.08759**, 2022. Predicts mastering-style parameters from a reference recording; compares TCN end-to-end, neural-proxy, SPSA, and auto-diff.

- **Martínez-Ramírez, M. A.; Liao, W.-H.; Fabbro, G.; Uhlich, S.; Nagashima, C.; Mitsufuji, Y.** — *"Automatic music mixing with deep learning and out-of-domain data."* ISMIR 2022. **arXiv:2208.11428**. Mixing counterpart.

- **DeepAFx / DeepAFx-ST** — Adobe Research, open source: GitHub `adobe-research/DeepAFx` and `DeepAFx-ST`. Reference implementations of black-box differentiable DSP including a mastering FX-chain example (compressor + limiter).

- **DDSP** — Engel, J. et al. — Google Magenta DDSP toolkit (2020). Differentiable additive/subtractive synthesis layers; foundation for the differentiable-DSP movement.

### Perceptual Losses

Commonly used in training:
- **Multi-resolution STFT** (Yamamoto et al. 2020)
- **Mel-spectrogram L1**
- **Log-magnitude spectral convergence**
- JND-based weighting from psychoacoustic models — thesis-level work, not common in shipped products

### Practical Recommendation

**Don't** do end-to-end waveform synthesis. Instead:
1. Train a **genre/style classifier** on curated corpus
2. Classifier picks **parameter preset** for a hand-built DSP chain
3. Optionally refine parameters via **SPSA-style gradient estimation** against perceptual loss to user-supplied reference

This is essentially what Ozone Master Assistant does.

---

## 26. Auto-Correcting Thresholds (Software Automation)

Encode as automated checks in the rendering pipeline:

| Condition | Action |
|---|---|
| Max true peak post-render **> −1 dBTP** | Reduce limiter ceiling **0.5 dB** and re-render; **up to 3 attempts**; flag for manual review if still failing |
| **LRA < 4 LU** AND genre ≠ EDM/metal/hip-hop | Reduce limiter drive **2 dB**; add upward compressor |
| **PSR < 8** anywhere | Bypass soft-clip stage; reduce limiter drive **1.5 dB** |
| Correlation **< 0 sustained** | Tighten side level **2 dB** or engage mono-summing below **200 Hz** |
| Integrated loudness post-render **> 2 dB off target** | Adjust input gain and re-render |
| Codec preview (AAC/Ogg) ISP **> ceiling** | Reduce limiter ceiling 0.5 dB and re-render (treat as headroom regression) |
| **Single-stage limiter GR > 4 dB** | Suggest staged limiting strategy or back off drive |
| **Lossy intake detected** | Set TP ceiling to −2 dBTP; bypass aggressive processing; warn user |
| **Already-overlimited intake** (low LRA + low PSR throughout) | Flag at intake; bypass all dynamics processing; warn user |

### Multi-Format Branching

Single source-master in 32-bit float at native sample rate; branch renderers per delivery target. Each renderer handles:
- Format-appropriate dither (TPDF default; noise-shape only for CD)
- Bit-depth quantization
- SRC (high-quality polyphase: libsoxr, r8brain, SSRC)
- Encoded preview generation (`afconvert` for AAC, libvorbis for Ogg, libopus for Opus)
- **Loudness-conformance check that re-runs BS.1770-4 measurement on the rendered output** (not just internal bus); warn/auto-trim if non-compliant

---

## 27. Operational Checklists

Three discrete QC stages every render should pass through.

### Pre-Master Intake QC

Detect and report:
- **File type, sample rate, bit depth, channel count**
- **Lossy/lossless status** (and codec/bitrate if lossy)
- **Clipped-sample count** (flag any > 0)
- **DC offset**
- **Phase / correlation** (flag sustained negative)
- **Measure:** integrated loudness, short-term max, momentary max, max true peak, LRA, PLR, PSR
- **Flag likely codec-compromised or already-overlimited files BEFORE processing**

Present intake report to user. Don't silently process problem files.

### Processing QC

- **Gain-match all before/after comparisons** (mandatory — prevents "louder = better" bias)
- **Enforce per-stage intervention caps** in transparent mode (e.g., max 3 dB EQ move, max 3 dB compressor GR)
- **Re-check true peak after any stage that can create overshoot:**
  - Clipping
  - Limiting
  - SRC
  - Dithering with noise shaping
  - Lossy preview encode
- **Prefer bypass over stacked correction** if no stage materially improves translation
- **Run codec preview at key checkpoints**, not just final export

### Final Export QC

- **Verify final sample rate / bit depth** against intended destination
- **Verify no post-dither processing occurred**
- **Preview AAC / Ogg / MP3 derivative** for clipping / edge distortion
- **Verify metadata manifest**, ISRC mapping, file naming, folder structure, version consistency
- **For CD/DDP:** run conformity checks; ensure timing / PQ / text fields are finalized
- **Re-run BS.1770-4 measurement on the rendered output file** (not just the internal bus)
- **Null test** where applicable (regression test for unchanged settings)

### Minimal Adaptive Policy for a One-Click App

A practical implementation can be surprisingly disciplined:

1. **Analyze** native file → compute LUFS-I, max short-term, max momentary, max dBTP, LRA, PLR, PSR, peak histogram, clipped-sample count, low-frequency correlation, basic spectral balance
2. **Classify** into broad operating modes (Transparent / Standard / Dense-Loud / Dynamic-Acoustic) — by **spectral density and dynamic descriptors**, not genre tags alone
3. **Apply only necessary stages**, in order: corrective EQ → broad dynamics → adaptive spectral control → final TP limiting → optional dither
4. **Codec-preview and export** one primary lossless master, then derived deliverables

---

## 28. QA / Validation Settings

Every output master should be programmatically verified.

### Measurements

| Check | Threshold |
|---|---|
| **BS.1770 integrated LUFS** | ≤ target ± 0.5 LU |
| **Short-term LUFS** | track maxima |
| **Momentary LUFS** | track maxima |
| **True peak (dBTP)** | ≤ ceiling, oversampled ≥ 4× |
| **LRA** | within genre range |
| **PLR** | within genre range |
| **PSR (worst section)** | ≥ 8 (soft alarm if lower) |
| **Mono-compatibility correlation** | ≥ 0 sustained |
| **Codec round-trip true peaks** | AAC 256 / Ogg 160 / Opus 128 ≤ ceiling |
| **Platform-penalty simulation** | per-platform attenuation prediction |
| **Clipped-sample count** | 0 |
| **DC offset** | within tolerance |

### Process

- **Null test** where applicable (regression — unchanged settings produce sample-identical output)
- **Metadata validation** — required tags present and conformant
- **Album-gap continuity** — sample-accurate boundary checks for continuous albums
- **Level-matched A/B** — auto-equalize playback level before any subjective comparison

### Listening Environments

Test masters on:
- Treated monitors (room-corrected if possible)
- Consumer headphones
- Cheap earbuds (AirPods, generic Bluetooth)
- Laptop speakers
- Phone speaker (mono summing test)
- Car system
- Noisy-environment simulation

At **three SPL levels: 65, 75, 85 dB.**

### Regression Corpus

Maintain golden-master reference set spanning:
- Transient-heavy metal
- Bright pop
- Sparse acoustic
- Wide-dynamic classical
- Difficult transitions
- Known-loudness synthetic test signals (sine waves, pink noise, music tones at calibrated levels)
- Lossy-source examples (for intake QC validation)

---

## 29. Reference Library / Standards Index

### Tier 1 — Authoritative Standards

- **ITU-R BS.1770-5 (Nov 2023)** — primary loudness measurement spec; adds advanced/immersive channel weighting (7.1.4 / Atmos via azimuth + elevation)
- **ITU-R BS.1770-4 (Oct 2015)** — widely-deployed prior revision
- **EBU R 128** (Nov 2023 revision) — broadcast loudness normalization
- **EBU R 128 s2** (Nov 2023) — streaming supplement (unchanged −23 LUFS or interim −20 to −16 LUFS)
- **EBU Tech 3341** (Nov 2023) — meter behavior / M/S/I refresh / scale / gating
- **EBU Tech 3342** (Nov 2023) — LRA definition and computation
- **EBU Tech 3343** (Nov 2023) — production guidelines / loudness normalization philosophy
- **EBU Tech 3344** — distribution / reproduction guidelines
- **EBU Tech 3285** (v2, 2011 reissue) — Broadcast Wave Format (BWF) specification
- **EBU Tech 3352** — ISRC embedding in BWF axml chunk
- **AES TD1008.1.21-9 (Sep 2021)** — original streaming delivery rec
- **AES77 (Jul 2023)** — updated streaming delivery rec, replaces TD1008
- **AES17** — digital audio measurement procedure
- **AES31** — audio file interchange
- **AES77-2025** — immersive audio measurement
- **IEC 60908 (Red Book)** — CD audio
- **DDP 2.00** — Disc Description Protocol for CD pre-master delivery
- **ATSC A/85 (CALM)** — US broadcast loudness
- **Dolby ADM BWF** — Atmos master file format
- **IFPI ISRC Handbook 4th Edition (2021)** — ISRC assignment principles

### Tier 2 — Manufacturer / Platform Specifications

- **Apple Digital Masters technology brief** — mastering spec; native-resolution rule; no upsampling
- **Apple Video and Audio Asset Guide** — approved sample rates, 24-bit requirement, Hi-Res Lossless rules
- **Spotify Loudness Normalization** — −14 LUFS, −1 dBTP target, in-line limiter spec
- **Spotify file-format docs** — lossless ingest, native-rate preference
- **Netflix Originals Loudness Spec** — −27 LKFS ±2, −2 dBTP
- **YouTube Upload Specs** — 48 kHz / 24-bit recommended
- **FabFilter Pro-L 2 help** — true-peak limiting reference design
- **FabFilter Pro-Q help** — EQ reference (mode behavior, latency, pre-ring)
- **FabFilter Pro-C help** — compressor reference (program-dependency, time constants)
- **Softube Weiss DS1-MK3 manual** — mastering de-esser reference
- **Softube Weiss DS5 Multiband Compressor manual** — multiband reference
- **NUGEN Audio ISL manual** — true-peak limiter benchmark
- **iZotope dithering guide** — dither type recommendations
- **iZotope Ozone dithering support note** — when-to-dither rules
- **iZotope Ozone Documentation** — assistive mastering reference
- **Steinberg WaveLab help** — DDP, metadata, loudness analysis, final render

### Tier 3 — Practitioner Authorities & Textbooks

**Textbooks (Implementation-Relevant):**
- **Bob Katz, *Mastering Audio: The Art and the Science*** (3rd ed., 2015 copyright / 2014 release) — most complete workflow-oriented mastering text
- **Jonathan Wyner, *Audio Mastering: Essential Practices*** (2012; 2nd ed. referenced by Berklee 2025) — strong on modern practical workflow and education
- **Bobby Owsinski, *The Mastering Engineer's Handbook*** (5th ed., current) — concise applied overview with modern deliverable context

**Seminal Papers:**
- **Vanderkooy & Lipshitz** — *"Quantization and Dither: A Theoretical Survey"* (JAES, 1992) — canonical dither theory
- **Nielsen & Lund** — *"Overload in Signal Conversion"* (AES, 2003) — classic treatment of inter-sample peaks and codec/SRC overload
- **Katz** — *"Level Control in Digital Mastering"* (AES, 1999) — early critique of sample-peak-only thinking
- **"Giant FFTs for Sample-Rate Conversion"** (JAES, March 2023) — modern offline SRC design
- **Bob Katz "K-System" JAES paper (2000)** — calibrated monitoring + metering protocol

**Practitioner Authorities:**
- **Ian Shepherd** — Loudness Penalty tool; Sound on Sound interviews; PSR ≥ 8 rule; *Production Advice* blog
- **Pete Lyman** — minimal-processing philosophy; WaveLab workflow
- **Dave Kutch** — "don't master to a service target" philosophy
- **Bob Ludwig** — mastering-as-minutiae; codec-check-tool advocacy
- **Robert Bristow-Johnson, "Cookbook formulae for audio EQ biquad filter coefficients"** — biquad design reference
- **Yoad Nevo** (via Waves) — broadband mastering compression ratios
- **MeterPlugs blog** — PLR/PSR practical guidance
- **Sage Audio mastering blog** — dither curve specifics

### Tier 3 — Reference Implementations to Study or Port

- **libebur128** (MIT) — canonical BS.1770 C implementation
- **pyloudnorm** — pure-Python BS.1770-compliant meter
- **r8brain-free-src** (public domain) — high-quality SRC C++ library
- **SoX / soxr** — VHQ sample rate conversion
- **libsoxr / libsamplerate** — SRC C libraries
- **Matchering 2.0** — open-source reference-based mastering
- **Essentia `TruePeakDetector`** — BS.1770 true-peak open implementation
- **DeepAFx / DeepAFx-ST** (Adobe, GitHub) — black-box differentiable DSP
- **JUCE** — audio framework (if building plugins/desktop apps)

---

## 30. Open Questions / Caveats

- **There is no universal, official, genre-specific loudness target for commercial music platforms** in the way broadcast has an explicit −23 LUFS target. Platform normalization behavior is real and documented, but music-mastering loudness remains a hybrid of standards, platform behavior, and engineer judgment. Best modern app design emphasizes sound-first processing, true-peak safety, and playback prediction — **not a single numeric loudness destination**.
- **Streaming targets are not legally binding and platforms change them quietly.** YouTube changed −13 → −14; Apple flipped Sound Check to default-on/LUFS in 2022; Spotify changed default headroom multiple times. **Build targets as a config file, not constants.** Re-verify against published platform docs at release time.
- **Apple Music's −18 LUFS Atmos / −16 LUFS stereo numbers are *integrated*;** momentary peaks routinely sit much higher, which is why the −1 dBTP rule is enforced separately. Don't conflate.
- **"−14 LUFS is the universal target" is partially wrong** for genres whose aesthetic depends on density (metal, EDM, hip-hop). Platforms attenuate, but *character* still differs at source. Don't auto-clamp every master to −14 LUFS.
- **`afclip` / `AURoundTripAAC` only check Apple's AAC encoder.** Spotify Ogg Vorbis transcoding clips differently. Use Sonnox Pro-Codec or libvorbis preview for Spotify-specific checks.
- **Spotify "Loud" / "Quiet" listener distribution is not published precisely.** Estimates 7–10%; design for the 91% on default.
- **PLR/PSR are descriptive, not prescriptive.** Useful as soft alarms; some genres (sludge metal, ambient drone) intentionally violate PSR-8 and sound correct.
- **All AI mastering products lose to a skilled human** on material requiring taste judgments. Position accordingly.
- **Dither noise-shape interactions with lossy codecs are real and under-discussed.** POW-r 3 / MBIT+ Ultra master that sounds great on CD may produce audible swirling after Ogg Vorbis transcode. **When in doubt, flat TPDF.**
- **Apple's `afconvert` AAC encoder is the actual Apple Music encoder** — preview reliable; equivalent claims for Spotify cannot be verified independently.
- **Vinyl mastering, broadcast mastering, theatrical Atmos** are not one-click problems. Generate a *pre-master* file and spec sheet for the specialist engineer. **Restoration / remastering from damaged legacy media** also requires human review paths.
- **SoundCloud normalization behavior is unclear** from public docs. Treat as no-normalization safety case.
- **YouTube Music vs YouTube proper:** share −14 LUFS turn-down-only, but YouTube Music attenuates only above ~−7 LUFS, **much more permissive** than the video platform.
- **Apple Music's exact integrated target is not officially published.** Field observation places Sound Check near −16 LUFS; treat as observed, not contract-level guidance.
- **AAC encoding behavior varies by encoder.** Apple's encoder, libfdk-aac, and FFmpeg's default differ. Apple's tools are authoritative for Apple delivery.
- **Linear-phase EQ pre-ringing on boosts vs cuts is genuinely controversial.** Top engineers split; expose both modes.
- **MQA is end-of-life** (admin April 2023, Tidal removal July 2024). Ignore for new products.
- **Don't ship to classical/jazz fully automated.** Those need a person and a calibrated room.
- **The scope of this reference is modern stereo/digital mastering.** Vinyl cutting, immersive mastering, restoration, and remastering from damaged legacy media each require their own deeper format-specific rule sets and human review.

---

*End of consolidated settings reference.*
*For build plans, app architecture, AI interstitial generation, market analysis, and product/UX considerations, see the separate `deep-research-report.md`, `compass_artifact_wf-0dd25647-…_text_markdown.md`, and `compass_artifact_wf-e83b62aa-…_text_markdown.md` files in this repo.*

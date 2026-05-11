# Claude Build Brief

Last updated: 2026-05-11

This repo is a clean build from zero. The product mission is fixed by `docs/PRODUCT.md`; the implementation is open.

The existing Codex repo has working Python/Tauri code, but this repo should not inherit that code. Use it only as proof that the product can exist and as optional context if the user asks.

These docs were prepared from a Codex planning/grill session that may not be visible to Claude Code on another machine. Treat the docs themselves as the durable handoff, not the prior chat transcript.

## Mission For This Build

Build the best private Windows desktop app for mastering individual songs and full albums.

The app should feel serious to a musician/producer:

- Fast enough to use creatively.
- Honest enough to trust.
- Deep enough for real albums.
- Simple enough to get a good result without technical setup.

## Required Product Shape

### Track Master

Build this first.

Required final behavior:

- Drop/add one or more audio files.
- Analyze.
- Apply safe Universal settings.
- Show a large waveform.
- Play/pause/seek.
- Zoom waveform.
- Select and loop an audition region.
- Toggle Original/Mastered at the same playhead.
- Optional Volume Match, off by default.
- Preset tiles.
- Intensity macro.
- Low/Mid/High EQ.
- Advanced controls tucked away.
- Real-time or near-real-time audition for basic controls.
- Export non-overwriting mastered files.
- Run post-render quality checks.

### Album Master

Required near-term, not someday.

Required final behavior:

- Drop/add multiple tracks.
- Reorder.
- Analyze sequence.
- Show Track Roles / Story step.
- Use global album intent plus granular per-track adaptation.
- Export individual masters and continuous album WAV.
- Preserve original boundaries by default.
- Generated transitions off by default.
- Provide gap/crossfade/boundary primitives.
- Produce useful album report/cue/split data where appropriate.

## Architecture Requirements

Do not lock framework before research/spikes.

The chosen architecture must support:

- Serious Windows desktop app behavior.
- Native or near-native audio audition.
- Real-time/near-real-time controls.
- Export parity between audition and final render.
- Offline local processing.
- Non-destructive source handling.
- Versioned/non-overwriting exports.
- Tests and smoke checks.

Architecture candidates to consider:

- JUCE/native desktop app.
- Tauri UI plus Rust native audio engine.
- Rust-native UI plus CPAL or similar audio layer.
- Hybrid shell with separate native audio/DSP engine.
- Python for offline algorithm experimentation only if paired with a credible realtime path.

The build should include an early architecture decision record with evidence.

## Early Spikes

Before committing hard to a stack:

1. Competitive UX reference spike.
2. Native audio/realtime audition spike.
3. Offline render quality spike.
4. Project/export safety spike.

Spike outputs should include:

- What was tested.
- Latency/performance.
- Audio quality concerns.
- Packaging concerns.
- Recommendation.
- What remains reversible.

## Research Usage

Use `docs/research/` for mastering standards and technical decisions:

- Loudness and true peak.
- Signal-chain order.
- Limiter behavior.
- Dither.
- Codec-aware delivery.
- Preset settings.
- Platform/export targets.
- Album-mode behavior.

Do not turn research into UI clutter. Apply it under the hood and in plain-language quality checks.

## First Useful Vertical Slice

The first serious slice should prove:

- One real track can be imported.
- It can be analyzed.
- A waveform appears.
- Source playback works.
- Mastered audition works.
- Original/Mastered toggle preserves playhead.
- At least one realtime control works.
- Export creates a new mastered WAV.
- A basic quality check runs.

Do not declare victory here. This is only the first proof.

## Public Release Risk Notes

This is private for now. Use commercial tools as references. Do not copy logos, assets, exact branding, or proprietary art.

If this ever becomes public, revisit:

- UX similarity.
- Claims.
- Licenses.
- Audio tool redistribution.
- Research attribution.
- User-data/audio privacy.

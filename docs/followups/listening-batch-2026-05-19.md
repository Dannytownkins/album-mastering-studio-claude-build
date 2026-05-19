# Listening Batch — 2026-05-19

Purpose:

Keep subjective musical-taste checks out of chat memory. These are deferred
monitor-listening items only; they are not per-commit mechanical gates.

## New DSP Taste Checks

1. Per-preset subsonic HPF cutoff tuning.
   - Infrastructure is in place.
   - Current values are mechanically gated in the 20-40 Hz mastering range.
   - Dan should decide whether any preset wants a higher/lower cutoff on
     monitors.

2. Per-preset `transient_punch` verification.
   - Infrastructure and envelope behavior are mechanically gated.
   - Dan should confirm whether Punch, Loud, Warmth, Tape, and the rest feel
     right on real material.
   - Tune amounts only after listening notes name the preset and direction.

## Carried Forward From `HANDOFF_2026-05-15_evening.md`

1. B6 ceiling-bounded LUFS landing.
   - Does the Loudness Target slider feel responsive on quieter sources and
     lower-intensity material across track, album-simple, and album-plan render
     paths?

2. VM cap on aggressive settings.
   - Recheck Dan's reproducible case: Tape preset, Intensity 100%, +13 dB input
     gain, Volume Match on.
   - Expected subjective result: lands near source loudness, not dramatically
     over-attenuated.

3. Decode-stall fix end-to-end.
   - Select track, click Mastered: swap should feel effectively immediate.
   - Also try import-new-file, restart-app/autosave restore, and open-project
     auto-prewarm paths.

4. LoudnessTarget readout truthfulness.
   - Pick non-Custom delivery profiles and confirm the readout communicates the
     effective profile target clearly.

5. Preset character on real material.
   - Confirm each preset still delivers its named identity after the Phase A4
     retune and later mechanical DSP slices.

## Rule

If a finding is about correctness, write a mechanical test. If a finding is
about taste, capture Dan's listening note here or in `docs/progress.md` before
changing preset values.

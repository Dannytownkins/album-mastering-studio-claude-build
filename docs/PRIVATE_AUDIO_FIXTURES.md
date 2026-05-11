# Private Audio Fixtures

Use this convention when testing Album Mastering Studio with real user audio.

Private audio must never be committed. The repo ignores:

```text
private-audio-fixtures/
```

## Folder Shape

```text
private-audio-fixtures/
  manifest.json
  clean-full-mix.wav
  rough-problem-track.wav
  acoustic-quiet-track.wav
  heavy-dense-track.wav
  album-sequence-01.wav
  album-sequence-02.wav
```

The filenames above are examples. Use whatever local names are convenient.

## Manifest Template

```json
{
  "version": 1,
  "notes": "Private local fixtures for Album Mastering Studio. Do not commit audio.",
  "fixtures": [
    {
      "id": "clean-full-mix",
      "path": "clean-full-mix.wav",
      "purpose": "A finished mix that already sounds decent.",
      "mode": ["track"],
      "quick_test": true,
      "slow_test": true,
      "listening_focus": ["overall polish", "source/master A-B", "export checks"],
      "known_issues": []
    },
    {
      "id": "rough-problem-track",
      "path": "rough-problem-track.wav",
      "purpose": "A mix with harshness, mud, clipping, dullness, or other known problems.",
      "mode": ["track"],
      "quick_test": true,
      "slow_test": true,
      "listening_focus": ["quality warnings", "safe Universal behavior", "advanced controls"],
      "known_issues": ["describe what bothers you before the app touches it"]
    },
    {
      "id": "album-sequence",
      "paths": ["album-sequence-01.wav", "album-sequence-02.wav"],
      "purpose": "Adjacent album tracks for sequence, role, boundary, and consistency testing.",
      "mode": ["album"],
      "quick_test": false,
      "slow_test": true,
      "listening_focus": ["track-to-track loudness", "boundaries", "album cohesion"],
      "known_issues": []
    }
  ]
}
```

## Rules For Agents

- Use real fixtures for local automated analysis/render checks when available.
- Use real fixtures for manual listening notes when judging product quality.
- Do not commit private audio, rendered masters from private audio, waveform images derived from private audio, or fixture-specific generated artifacts.
- Do not assume fixture files exist. If missing, fall back to synthetic tests and say that real-audio verification is still pending.
- Prefer short clips for quick loops and full tracks/albums for slow verification.

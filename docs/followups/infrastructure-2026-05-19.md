# Infrastructure / Distribution Follow-ups — 2026-05-19

Purpose:

Keep non-listening project debt out of chat memory. These are mechanical or
distribution items, not monitor-time musical taste checks.

## Distribution

1. Apple Developer credentials and notarization.
   - Current macOS app bundle is locally/ad-hoc signed and works on this Mac.
   - Wider distribution requires Developer ID signing, Apple notarization, and
     the release workflow around those credentials.
   - Do not block local development on this; pick it up when YES Master needs
     to leave Dan's Mac.

## Cleanup

1. Legacy frontend album export hook.
   - Status: removed in the 2026-05-18 evening cleanup batch.
   - The visible UI now exports through the Album Plan path only.
   - The frontend API wrapper for `render_album_master` was also removed.

2. Backend simple-album render command.
   - `render_album_master` / `album_render_with_progress` still exist in Rust
     and are covered by backend contract tests.
   - Leave them until a deliberate Rust cleanup slice decides whether the old
     simple-album path is still useful as a test harness/back-compat surface.

3. Tests still co-located in `audio.rs`.
   - The 2026-05-18 audio split moved production code into `spectrum.rs`,
     `decode.rs`, and `sources.rs`.
   - Many related tests still live in `audio.rs`.
   - Future cleanup can move tests next to the lifted modules, but this is not
     urgent and should be a mechanical refactor with unchanged behavior.

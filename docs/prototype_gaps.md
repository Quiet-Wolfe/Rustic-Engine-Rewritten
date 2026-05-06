# Prototype Gaps

Temporary gaps and local fallback policy while the vertical gameplay slice is
still being brought up. These are not intentional fidelity deviations; move
items to `docs/fidelity_deviations.md` only if we decide to ship different
behavior on purpose.

## Runtime Fallbacks

- Audio output can be unavailable or can block during device probing on desktop
  remoting stacks such as Chrome Remote Desktop. The app must still launch and
  use the wall-clock preview cursor when CPAL output is disabled, errors, or
  times out. `RUSTIC_AUDIO=off` forces this path, and
  `RUSTIC_AUDIO_OPEN_TIMEOUT_MS` tunes the bounded probe.
- Text should use a fallback system monospace font until the OG VCR font asset
  is present in the asset tree. This keeps HUD/menu work unblocked, but the
  fallback font is not a fidelity match.

## Known Visual/Timing Gaps

- Conductor/receptor confirm timing is still approximate. The app currently
  holds confirm state for a fixed sample window, while base FNF plays
  lane-specific confirm animations from `NOTE_assets`
  (`PlayState.hx:1138-1153,2127`).
- Note splashes and hold/sustain splashes are not implemented yet. No splash
  atlas is present in the pinned base asset checkout, so any implementation
  needs an explicit asset source decision before it can be fidelity-tested.

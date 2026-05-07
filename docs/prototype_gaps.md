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

- Receptor hold-confirm behavior now extends while sustain ticks resolve, but
  it is still a simplified state model compared to v0.8.5's
  `StrumlineNote.holdConfirm()` transition into `confirm-hold`.
- Hold trails, hold covers, and note splashes use the v0.8.5 assets. Dropped
  and missed hold trails are still visually simplified; v0.8.5 hides or
  clips them based on `SustainTrail.missedNote`.

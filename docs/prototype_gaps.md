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
- OG VCR font assets are present in `assets/source/fonts`. Text rendering is
  not wired yet; until it is, any text UI should fall back to a system
  monospace font rather than blocking gameplay work.

## Known Visual/Timing Gaps

- Receptor hold-confirm behavior now follows active hold state instead of
  generated sustain children, but it is still a simplified state model compared
  to v0.8.5's `StrumlineNote.holdConfirm()` transition into `confirm-hold`.
- Hold trails, hold covers, and note splashes use the v0.8.5 assets. Hold
  trail clipping is still quad-tiled rather than a literal `drawTriangles`
  port of `SustainTrail`.

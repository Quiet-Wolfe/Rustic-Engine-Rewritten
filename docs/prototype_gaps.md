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
- OG VCR bitmap font assets are present in `assets/source/fonts` and are wired
  for the gameplay score HUD. Full `glyphon` text is still pending for broader
  menu/debug UI.

## Known Visual/Timing Gaps

- Receptor hold-confirm behavior follows active hold state and transitions into
  the v0.8.5 `confirm-hold` atlas frames after the confirm animation completes.
- Hold trails, hold covers, and note splashes use the v0.8.5 assets. Hold
  trails use the original wrapped UV math through sprite quads; a literal
  arbitrary-vertex `drawTriangles` path is still deferred.

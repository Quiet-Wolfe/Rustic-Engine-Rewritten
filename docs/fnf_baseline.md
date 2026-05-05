# FNF Baseline

This document pins the exact upstream Friday Night Funkin' baseline that
RusticV3 ports against. Phase 0 must fill in every TODO below before any
gameplay porting begins. The fidelity contract in `PLAN.md` Section 2 depends
on this file.

## Upstream source pin

- Upstream repository: `https://github.com/FunkinCrew/Funkin` (the legacy
  pre-Psych base FNF tree).
- Pinned commit: `50fccded66742a8117c898deb59bb0c2f14fb22d`
- Short SHA used in `// ref:` comments: `50fccded`
- Tag at this commit: `v0.2.7.1`
- Pin date: `2021-02-14`
- Branch context: tag tip (no active branch — this is the public legacy
  release before the Psych-era refactors).
- Local checkout: `references/Funkin/` (gitignored; not vendored).

All `// ref: <fnf-commit>:<file>:<line>` comments in ported gameplay code
resolve against this commit. Use the short SHA `50fccded`. Do not bump the
pin without updating this file and re-running the regression suite.

## Reference binary

- Build: `TODO: <official build name and version>`
- Platform captured on: `TODO: <OS + version>`
- Architecture: `TODO: <x86_64 / arm64>`
- Binary checksum (SHA-256): `TODO: <hash>`
- Source of binary: `TODO: <download URL or distribution>`

When the source is ambiguous, observed behavior of this binary is the truth.

## Capture environment

- Host OS used for captures: `TODO`
- Display: `TODO: <native resolution + refresh rate>`
- Graphics driver: `TODO`
- Audio device + sample rate: `TODO`
- Capture tool: `TODO: <e.g. OBS settings, ffmpeg command>`
- Frame capture format: `TODO: <e.g. PNG sequence at 60 Hz>`
- Audio capture format: `TODO: <e.g. WAV 48 kHz stereo>`

## Rendering reference mode

- Logical baseline resolution: 1280x720.
- Reference output mode renders to a 1280x720 texture and integer/letterbox
  upscales for capture. See `PLAN.md` Section 2 and Section 7.
- Native output mode is used for normal play and is **not** the comparison
  path for goldens.

## Fidelity rule

1. Match base FNF first.
2. Improve only after the mismatch is documented in
   `docs/fidelity_deviations.md`.
3. Keep intentional deviations listed there with a clear rationale.

## Workflow

- New gameplay port PRs:
  - Carry a `// ref:` comment for each ported function.
  - Cite the pinned commit above; never use `HEAD` or a branch name.
- Visual regression PRs:
  - Reference frames are captured against the reference binary above, not a
    live FNF build.
- Updates to this file:
  - Treat as a baseline change. Re-run the full regression suite and call
    out the diff in the PR description.

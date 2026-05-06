# FNF Baseline

This document pins the exact upstream Friday Night Funkin' baseline that
RusticV3 ports against. Phase 0 must fill in every TODO below before any
gameplay porting begins. The fidelity contract in `PLAN.md` Section 2 depends
on this file.

## Upstream source pin

- Upstream repository: `https://github.com/FunkinCrew/Funkin`.
- Pinned commit: `bdedc0aad2b93b3a7787357313ba662ba8d3173f`
- Short SHA for new `// ref:` comments: `bdedc0aa`
- Tag at this commit: `v0.8.5`
- Pin date: `2026-04-05`
- Branch context: `main` at release tag `v0.8.5`.
- Local checkout: `references/Funkin/` (gitignored; not vendored).
- Required submodules:
  - `assets` at `d1d027d4747aaba151c6df121ea736c31d6aed38`
  - `art` at `429ab728b19272dca834c83d97b1646455b22579`

Initialize or refresh the local reference with:

```sh
git -C references/Funkin checkout main
git -C references/Funkin pull --ff-only --recurse-submodules
git -C references/Funkin submodule update --init --recursive
```

New ported behavior should cite this pin with
`// ref: bdedc0aa:<path>:<line>`. Existing `50fccded` comments are legacy
citations from the original prototype baseline and must be audited before they
are treated as current-fidelity proof.

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
  - Do not copy old `50fccded` citations forward without checking the matching
    `v0.8.5` source path and line.
- Visual regression PRs:
  - Reference frames are captured against the reference binary above, not a
    live FNF build.
- Updates to this file:
  - Treat as a baseline change. Re-run the full regression suite and call
    out the diff in the PR description.

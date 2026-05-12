# CI

CI for RusticV3 has three jobs: build/check, lints, and visual regression.
The renderer is `wgpu`-first per `PLAN.md` Section 7, so visual regression
runs through Mesa lavapipe's Vulkan backend so CI does not depend on a vendor
GPU.

## Build / check

- `cargo check --workspace --all-targets` on Linux for every PR.
- `cargo check --workspace --target x86_64-pc-windows-gnu` when the toolchain
  is available.
- `cargo check --workspace --target x86_64-apple-darwin` when CI hardware is
  available.
- `cargo check --workspace --target aarch64-linux-android` when the Android
  NDK is available.
- `cargo test --workspace --all-targets` on Linux for every PR.

## Lints

Runs `tools/lint.sh`, which composes:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `tools/lint/file_size.sh` (400 soft / 800 hard line caps).
- Asset I/O whitelist grep.
- Backend API whitelist grep (`ash::`, `vk::`, `metal::`, `d3d12`, GLSL/SPIR-V
  paths).
- `// ref:` comment requirement for changed gameplay files.

If a grep check becomes noisy, replace it with a custom AST check or
`dylint`. See `PLAN.md` Section 17.

## Visual regression

- Runner: `cargo xtask regression`.
- Backend: `wgpu` on Mesa lavapipe (Vulkan).
- Pinned Mesa version: `TODO: <e.g. mesa 24.0.x>`.
- Pinned lavapipe driver options: `TODO`.
- Goldens: `tests/golden/` tracked in CI.
- Optional native-GPU developer goldens live in `tests/golden_dev/` and run
  when `RUSTIC_REGRESSION=1` is set.
- Per-image diff thresholds. Document any threshold above the global default
  in the golden directory next to the image.
- Backend/adapter info is written into regression artifacts so accidental
  backend changes are obvious.

### Current golden set

Tracked in `tests/golden/`:

1. `title_skipped_intro.png`.
2. `stage_idle_bopeebo.png`.
3. `stage_idle_tutorial.png`.
4. `bf_idle_two_beat_bopeebo.png`.
5. `stage_camera_bump_bopeebo.png`.
6. `gameplay_notes_crossing_bopeebo.png`.

### When lavapipe is too unstable

If a golden image is too noisy on lavapipe to be useful in CI, either:

- Raise the per-image threshold and document the reason next to the golden,
  or
- Move that case to `tests/golden_dev/` and exclude it from CI.

Do not silently mute regression jobs.

## Android

Native run smoke for Android is deferred until the desktop renderer, input,
and audio path are stable enough that real device issues are debuggable.
Until then, Android CI is `cargo check` only.

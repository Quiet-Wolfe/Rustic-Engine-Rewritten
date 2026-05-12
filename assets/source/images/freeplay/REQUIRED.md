# Freeplay source assets — required

Every file in this directory is required by `rustic-app::freeplay_assets`
to render the Freeplay menu at 1:1 fidelity with Funkin' v0.8.5.

DO NOT delete files here without first removing the matching call sites in
`crates/rustic-app/src/freeplay_assets.rs`. The
`freeplay_assets::tests::required_assets_present` unit test will fail
loudly if any required asset goes missing.

The authoritative inventory lives in `REQUIRED_FREEPLAY_ASSETS` in
`crates/rustic-app/src/freeplay_assets.rs`. Update that list when adding
or removing assets here.

Source upstream: `references/Funkin/assets/preload/images/freeplay/`

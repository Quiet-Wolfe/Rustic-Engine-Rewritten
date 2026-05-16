//! Runtime asset root discovery for app-owned loaders.

use rustic_asset::{resolver::DiskLayer, OverlayResolver};
use std::env;
use std::path::{Path, PathBuf};

pub const ASSET_OVERLAYS_ENV: &str = "RUSTIC_ASSET_OVERLAYS";

pub fn baked_assets_root() -> PathBuf {
    let cwd_relative = Path::new("assets/baked");
    if cwd_relative.exists() {
        return cwd_relative.to_path_buf();
    }
    workspace_root().join("assets/baked")
}

pub fn app_asset_resolver() -> OverlayResolver {
    let mut resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    for root in asset_overlay_roots() {
        resolver.push_overlay(DiskLayer::new(root));
    }
    resolver
}

fn asset_overlay_roots() -> Vec<PathBuf> {
    env::var_os(ASSET_OVERLAYS_ENV)
        .map(|value| env::split_paths(&value).collect())
        .unwrap_or_default()
}

fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Some(crates_dir) = manifest_dir.parent() else {
        return manifest_dir.to_path_buf();
    };
    let Some(workspace) = crates_dir.parent() else {
        return manifest_dir.to_path_buf();
    };
    workspace.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustic_asset::{AssetPath, AssetResolver};

    #[test]
    fn app_asset_resolver_falls_back_to_baked_assets() {
        let resolver = app_asset_resolver();
        let path = AssetPath::new("data/players/bf.json").unwrap();

        assert!(resolver.resolve(&path).is_ok());
    }
}

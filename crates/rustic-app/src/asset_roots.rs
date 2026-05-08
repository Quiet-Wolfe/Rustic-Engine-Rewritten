//! Runtime asset root discovery for app-owned loaders.

use std::path::{Path, PathBuf};

pub fn baked_assets_root() -> PathBuf {
    let cwd_relative = Path::new("assets/baked");
    if cwd_relative.exists() {
        return cwd_relative.to_path_buf();
    }
    workspace_root().join("assets/baked")
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

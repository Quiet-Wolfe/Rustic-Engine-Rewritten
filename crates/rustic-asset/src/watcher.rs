//! Hot-reload watchers. See `PLAN.md` Sections 6 and 14.
//!
//! V1 uses a simple polling watcher so hot-reload callers can observe file
//! changes without pulling in platform watcher dependencies yet. A later
//! `notify` backend can slot behind the same `Watcher::poll` contract.

use crate::error::{AssetError, AssetResult};
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug)]
#[non_exhaustive]
pub struct Watcher {
    path: Option<PathBuf>,
    last_modified: Option<SystemTime>,
    last_len: Option<u64>,
}

impl Watcher {
    pub(crate) fn placeholder() -> Self {
        Self {
            path: None,
            last_modified: None,
            last_len: None,
        }
    }

    pub(crate) fn file(path: PathBuf) -> AssetResult<Self> {
        let (last_modified, last_len) = metadata_state(&path)?;
        Ok(Self {
            path: Some(path),
            last_modified,
            last_len,
        })
    }

    pub fn poll(&mut self) -> AssetResult<bool> {
        let Some(path) = self.path.as_ref() else {
            return Ok(false);
        };
        let (modified, len) = metadata_state(path)?;
        let changed = modified != self.last_modified || len != self.last_len;
        self.last_modified = modified;
        self.last_len = len;
        Ok(changed)
    }
}

fn metadata_state(path: &PathBuf) -> AssetResult<(Option<SystemTime>, Option<u64>)> {
    let metadata = std::fs::metadata(path).map_err(|e| AssetError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let modified = metadata.modified().ok();
    Ok((modified, Some(metadata.len())))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_never_changes() {
        let mut watcher = Watcher::placeholder();
        assert!(!watcher.poll().unwrap());
    }

    #[test]
    fn polling_file_watcher_detects_len_change() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("asset.txt");
        std::fs::write(&file, b"one").unwrap();

        let mut watcher = Watcher::file(file.clone()).unwrap();
        assert!(!watcher.poll().unwrap());

        std::fs::write(&file, b"one-two").unwrap();
        assert!(watcher.poll().unwrap());
        assert!(!watcher.poll().unwrap());
    }
}

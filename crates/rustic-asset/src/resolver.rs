//! Asset resolver trait and the v1 overlay implementation.
//!
//! Lookup order (`PLAN.md` Section 6):
//! 1. Mod overlays, in registration order.
//! 2. Vanilla baked assets.
//!
//! In v1 there is no mod loader; overlays exist so the resolver shape is
//! the modding contract from day one.

use crate::error::{AssetError, AssetResult};
use crate::path::AssetPath;
use crate::source::AssetSource;
use crate::watcher::Watcher;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub trait AssetResolver: Send + Sync {
    fn resolve(&self, path: &AssetPath) -> AssetResult<AssetSource>;
    fn watch(&mut self, path: &AssetPath) -> AssetResult<Watcher>;
}

/// Layered resolver: in-memory overlays first, then a baked-on-disk root.
/// Either layer may be empty.
#[derive(Debug, Default)]
pub struct OverlayResolver {
    overlays: Vec<Box<dyn Layer>>,
    baked_root: Option<PathBuf>,
}

impl OverlayResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_baked_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.baked_root = Some(root.into());
        self
    }

    pub fn push_overlay<L: Layer + 'static>(&mut self, layer: L) {
        self.overlays.push(Box::new(layer));
    }

    fn resolve_baked(&self, path: &AssetPath) -> AssetResult<AssetSource> {
        let root = self
            .baked_root
            .as_deref()
            .ok_or_else(|| AssetError::NotFound(path.as_str().into()))?;
        let on_disk = root.join(path.as_str());
        if !on_disk.exists() {
            return Err(AssetError::NotFound(path.as_str().into()));
        }
        Ok(AssetSource::File(on_disk))
    }
}

impl AssetResolver for OverlayResolver {
    fn resolve(&self, path: &AssetPath) -> AssetResult<AssetSource> {
        for layer in &self.overlays {
            match layer.resolve(path) {
                Ok(src) => return Ok(src),
                Err(AssetError::NotFound(_)) => continue,
                Err(other) => return Err(other),
            }
        }
        self.resolve_baked(path)
    }

    fn watch(&mut self, path: &AssetPath) -> AssetResult<Watcher> {
        for layer in &self.overlays {
            match layer.resolve(path) {
                Ok(AssetSource::File(file)) => return Watcher::file(file),
                Ok(AssetSource::Bytes(_)) => return Ok(Watcher::placeholder()),
                Err(AssetError::NotFound(_)) => continue,
                Err(other) => return Err(other),
            }
        }

        match self.resolve_baked(path)? {
            AssetSource::File(file) => Watcher::file(file),
            AssetSource::Bytes(_) => Ok(Watcher::placeholder()),
        }
    }
}

/// A single overlay layer. Layers should return `NotFound` when they don't
/// own the path so the resolver falls through to the next layer.
pub trait Layer: Send + Sync + std::fmt::Debug {
    fn resolve(&self, path: &AssetPath) -> AssetResult<AssetSource>;
}

/// In-memory overlay used by tests and (later) by mod overlays that hold
/// blobs in memory.
#[derive(Debug, Default)]
pub struct InMemoryLayer {
    map: HashMap<String, Arc<[u8]>>,
}

impl InMemoryLayer {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert(&mut self, path: AssetPath, bytes: impl Into<Arc<[u8]>>) {
        self.map.insert(path.into_string(), bytes.into());
    }
}

impl Layer for InMemoryLayer {
    fn resolve(&self, path: &AssetPath) -> AssetResult<AssetSource> {
        match self.map.get(path.as_str()) {
            Some(bytes) => Ok(AssetSource::Bytes(bytes.clone())),
            None => Err(AssetError::NotFound(path.as_str().into())),
        }
    }
}

/// Disk-backed overlay (used by future mod folders and dev override roots).
#[derive(Debug)]
pub struct DiskLayer {
    root: PathBuf,
}

impl DiskLayer {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    fn resolved_path(&self, path: &AssetPath) -> PathBuf {
        self.root.join(path.as_str())
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Layer for DiskLayer {
    fn resolve(&self, path: &AssetPath) -> AssetResult<AssetSource> {
        let on_disk = self.resolved_path(path);
        if on_disk.exists() {
            Ok(AssetSource::File(on_disk))
        } else {
            Err(AssetError::NotFound(path.as_str().into()))
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn ap(s: &str) -> AssetPath {
        AssetPath::new(s).unwrap()
    }

    #[test]
    fn overlay_wins_over_baked_when_present() {
        let mut resolver = OverlayResolver::new();
        let mut overlay = InMemoryLayer::new();
        overlay.insert(ap("songs/bopeebo/inst.ogg"), b"OVERLAY".to_vec());
        resolver.push_overlay(overlay);

        let got = resolver.resolve(&ap("songs/bopeebo/inst.ogg")).unwrap();
        match got {
            AssetSource::Bytes(b) => assert_eq!(&*b, b"OVERLAY"),
            _ => panic!("expected bytes from overlay"),
        }
    }

    #[test]
    fn falls_through_to_next_overlay_on_not_found() {
        let mut resolver = OverlayResolver::new();
        resolver.push_overlay(InMemoryLayer::new());
        let mut second = InMemoryLayer::new();
        second.insert(ap("ui/title.png"), b"PNG".to_vec());
        resolver.push_overlay(second);

        let got = resolver.resolve(&ap("ui/title.png")).unwrap();
        assert!(matches!(got, AssetSource::Bytes(_)));
    }

    #[test]
    fn missing_asset_is_not_found_not_panic() {
        let resolver = OverlayResolver::new();
        let err = resolver.resolve(&ap("songs/missing.ogg")).unwrap_err();
        assert!(matches!(err, AssetError::NotFound(_)));
    }

    #[test]
    fn baked_root_is_used_when_overlays_miss() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("ui");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("title.png"), b"BAKED").unwrap();

        let resolver = OverlayResolver::new().with_baked_root(dir.path());
        let got = resolver.resolve(&ap("ui/title.png")).unwrap();
        match got {
            AssetSource::File(p) => assert!(p.ends_with("ui/title.png")),
            _ => panic!("expected file source from baked root"),
        }
    }

    #[test]
    fn watch_uses_resolved_disk_file() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("ui");
        std::fs::create_dir_all(&nested).unwrap();
        let file = nested.join("title.png");
        std::fs::write(&file, b"BAKED").unwrap();

        let mut resolver = OverlayResolver::new().with_baked_root(dir.path());
        let mut watcher = resolver.watch(&ap("ui/title.png")).unwrap();
        assert!(!watcher.poll().unwrap());

        std::fs::write(&file, b"BAKED2").unwrap();
        assert!(watcher.poll().unwrap());
    }
}

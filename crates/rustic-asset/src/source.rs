//! Concrete asset sources returned by the resolver.

use crate::error::{AssetError, AssetResult};
use std::path::PathBuf;
use std::sync::Arc;

/// What `AssetResolver::resolve` returns. Bytes mode is used for tests and
/// in-memory overlays; file mode is used for baked/disk assets so callers
/// can stream large files (audio in particular) without a full read.
#[derive(Debug, Clone)]
pub enum AssetSource {
    /// In-memory bytes. Cheap to clone (`Arc<[u8]>`).
    Bytes(Arc<[u8]>),

    /// On-disk file. The resolver guarantees the path exists at the time
    /// of return; consumers may stream it through their own file open.
    File(PathBuf),
}

impl AssetSource {
    pub fn from_bytes(bytes: impl Into<Arc<[u8]>>) -> Self {
        Self::Bytes(bytes.into())
    }
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self::File(path.into())
    }

    /// Fully read the asset as bytes. For `Bytes`, this is a clone of the
    /// shared `Arc<[u8]>`. For `File`, this performs a single blocking read.
    /// Streaming consumers (e.g. audio decoders) should match on
    /// `AssetSource` directly instead of calling this.
    pub fn read_all(&self) -> AssetResult<Arc<[u8]>> {
        match self {
            AssetSource::Bytes(b) => Ok(b.clone()),
            AssetSource::File(p) => {
                let bytes = std::fs::read(p).map_err(|e| AssetError::Io {
                    path: p.display().to_string(),
                    source: e,
                })?;
                Ok(bytes.into())
            }
        }
    }
}

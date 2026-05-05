use thiserror::Error;

pub type AssetResult<T> = Result<T, AssetError>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AssetError {
    #[error("asset not found: {0}")]
    NotFound(String),

    #[error("invalid asset path: {0}")]
    InvalidPath(String),

    #[error("invalid asset data: {0}")]
    InvalidData(String),

    #[error("io error reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("watcher unsupported in this build")]
    WatcherUnsupported,
}

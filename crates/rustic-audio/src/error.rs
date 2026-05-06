use thiserror::Error;

pub type AudioResult<T> = Result<T, AudioError>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AudioError {
    #[error("audio device unavailable: {0}")]
    DeviceUnavailable(String),

    #[error("audio stream failure: {0}")]
    Stream(String),

    #[error("decode failure: {0}")]
    Decode(String),

    #[error("invalid audio source: {0}")]
    InvalidSource(String),

    #[error("mix failure: {0}")]
    Mix(String),

    #[error("seek out of range: {0}")]
    SeekRange(String),
}

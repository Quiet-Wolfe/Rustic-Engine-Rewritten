use thiserror::Error;

pub type AnimateResult<T> = Result<T, AnimateError>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AnimateError {
    #[error("malformed atlas: {0}")]
    Atlas(String),

    #[error("json parse: {0}")]
    Json(#[from] serde_json::Error),
}

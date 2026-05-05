use thiserror::Error;

pub type RenderResult<T> = Result<T, RenderError>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RenderError {
    #[error("no compatible adapter available")]
    NoAdapter,

    #[error("device request failed: {0}")]
    Device(String),

    #[error("surface creation/configuration failed: {0}")]
    Surface(String),

    #[error("shader compile failed: {0}")]
    Shader(String),

    #[error("texture upload failed: {0}")]
    Texture(String),

    #[error("image decode failed: {0}")]
    Decode(String),
}

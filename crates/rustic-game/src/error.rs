use thiserror::Error;

pub type GameResult<T> = Result<T, GameError>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GameError {
    #[error("invalid chart: {0}")]
    Chart(String),

    #[error("invalid note: {0}")]
    Note(String),
}

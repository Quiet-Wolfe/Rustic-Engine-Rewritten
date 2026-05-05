//! Shared error scaffolding.
//!
//! Each library crate defines its own `thiserror` enum and returns
//! `Result<T, CrateError>`. `rustic-core` provides only a small wrapper
//! used by primitives that don't justify their own error type.

use thiserror::Error;

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CoreError {
    #[error("invalid time conversion: {0}")]
    Time(&'static str),

    #[error("invalid id: {0}")]
    Id(&'static str),
}

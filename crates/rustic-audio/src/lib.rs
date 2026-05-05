//! `rustic-audio` — decoder, mixer, conductor. See `PLAN.md` Section 8.
//!
//! Phase 0 ships only the type shape. The real `cpal`-backed mixer and
//! streaming decoder land in Phase 5.

#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(unsafe_code)]

pub mod conductor;
pub mod error;
pub mod mixer;
pub mod source;

pub use conductor::{Conductor, ConductorState};
pub use error::{AudioError, AudioResult};
pub use mixer::Mixer;
pub use source::{Decoder, SoundSource};

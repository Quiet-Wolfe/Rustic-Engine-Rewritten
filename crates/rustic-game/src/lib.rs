//! `rustic-game` — headless gameplay state. See `PLAN.md` Section 9.
//!
//! `PlayState` is serializable and contains gameplay state only — no
//! textures, no audio handles, no filesystem paths. Render output is
//! produced as render commands or simple view models so dev rewind and
//! regression tests can drive gameplay deterministically.

#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(unsafe_code)]

pub mod error;
pub mod judgment;
pub mod note;
pub mod scoring;
pub mod state;

pub use error::{GameError, GameResult};
pub use judgment::{health_delta, late_note_health_delta, score_value, Judgment, JudgmentWindows};
pub use note::{notes_from_chart, Lane, Note};
pub use state::{PlayState, DEATH_HEALTH, INITIAL_HEALTH, MAX_HEALTH};

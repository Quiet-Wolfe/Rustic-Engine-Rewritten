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
pub mod progress;
pub mod scoring;
pub mod state;
pub mod view;

pub use error::{GameError, GameResult};
pub use judgment::{
    ghost_miss_health_delta, ghost_miss_score_delta, health_delta, late_note_health_delta,
    note_miss_score_delta, score_for_timing, Judgment, JudgmentWindows,
};
pub use note::{notes_from_chart, Lane, Note};
pub use progress::ResolvedOpponentNote;
pub use state::{PlayState, DEATH_HEALTH, INITIAL_HEALTH, MAX_HEALTH};
pub use view::{note_x, HoldTrailView, NoteView};

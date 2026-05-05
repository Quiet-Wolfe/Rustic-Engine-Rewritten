//! `rustic-dev` — feature-gated dev tooling. See `PLAN.md` Section 14.
//!
//! Excluded from release builds. May depend on whatever it needs.
//! V0 ships only the rewind buffer scaffold so `PlayState` clone size can
//! be measured by tests once gameplay state grows.

pub mod rewind;

pub use rewind::RewindBuffer;

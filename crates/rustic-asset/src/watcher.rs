//! Hot-reload watchers. See `PLAN.md` Sections 6 and 14.
//!
//! V0 stub: provides the type so callers can compile and tests can wire
//! `dev` features. Real `notify`-backed implementation lands in Phase 0/1
//! once the resolver has concrete sources behind it.

#[derive(Debug)]
#[non_exhaustive]
pub struct Watcher {
    /// Reserved for the future watcher handle (e.g. `notify::RecommendedWatcher`).
    _private: (),
}

impl Watcher {
    pub(crate) fn placeholder() -> Self {
        Self { _private: () }
    }
}

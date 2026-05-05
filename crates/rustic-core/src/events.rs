//! Typed event bus. See `PLAN.md` Sections 5 and 13.
//!
//! Forward compatibility:
//! - Public event types are `#[non_exhaustive]`.
//! - Adding fields/variants is non-breaking.
//! - Removing or renaming requires a major version bump in
//!   `docs/event_bus_versions.md`.
//!
//! `rustic-core` only declares the trait and the union event enum. Each
//! crate produces variants relevant to its domain.

use crate::ids::{AssetId, SongId};
use crate::time::{Samples, Seconds};

/// Top-level bus event. New variants may be added without a major bump;
/// renaming or removing an existing variant is a major event-bus break.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum BusEvent {
    /// An asset finished (re)loading and any cached handles should rebind.
    AssetReloaded {
        id: AssetId,
    },

    /// A periodic conductor snapshot. Consumers read this instead of
    /// owning their own time source. See `PLAN.md` Section 8.
    ConductorTick {
        song: SongId,
        sample_cursor: Samples,
        position: Seconds,
    },

    /// Gameplay-level reload requested. Gameplay decides whether to honor.
    GameplayReloadRequested,

    /// Screen lifecycle changes. Screen names are static identifiers, not
    /// human-facing labels.
    ScreenEnter {
        screen: &'static str,
    },
    ScreenExit {
        screen: &'static str,
    },
}

/// Event bus contract. Implementations may queue, broadcast, or drop based
/// on subscription policy.
pub trait EventBus: Send + Sync {
    fn publish(&self, event: BusEvent);
}

/// No-op bus useful for tests and headless contexts.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullBus;

impl EventBus for NullBus {
    fn publish(&self, _event: BusEvent) {}
}

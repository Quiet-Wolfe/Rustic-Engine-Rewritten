//! `rustic-core` — shared primitives.
//!
//! See `PLAN.md` Section 5. Allowed: strong IDs, math primitives, time
//! types, error wrappers, event bus traits, input enums, serialization
//! traits. Forbidden: file I/O, GPU/audio/window types, JSON/XML/PNG
//! parsing, gameplay rules, screen/menu code, plugin traits.

#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(unsafe_code)]

pub mod error;
pub mod events;
pub mod ids;
pub mod input;
pub mod render;
pub mod time;

pub use error::{CoreError, CoreResult};
pub use events::{BusEvent, EventBus};
pub use ids::{AssetId, CameraId, NoteId, SongId};
pub use input::{InputAction, InputState, NormalizedInputEvent};
pub use render::RenderLayer;
pub use time::{Beat, Bpm, Milliseconds, Samples, Seconds, Step};

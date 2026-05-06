//! `rustic-audio` — decoder, mixer, conductor. See `PLAN.md` Section 8.
//!
//! The mixer is headless and deterministic; platform device callbacks wire
//! into it from `rustic-app`.

#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(unsafe_code)]

pub mod conductor;
pub mod error;
pub mod mixer;
pub mod source;
pub mod vorbis;

pub use conductor::{map_bpm_changes, BpmChangeEvent, Conductor, ConductorState};
pub use error::{AudioError, AudioResult};
pub use mixer::{MixStats, Mixer, Stem, VoiceId};
pub use source::{Decoder, SoundSource};
pub use vorbis::{streaming_vorbis_source, VorbisDecoder};

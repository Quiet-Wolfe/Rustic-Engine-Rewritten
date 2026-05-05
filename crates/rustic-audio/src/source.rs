//! Sound source model. See `PLAN.md` Section 8.

use crate::error::AudioResult;
use rustic_core::time::Samples;
use std::sync::Arc;

/// Streaming decoder contract. Songs use streaming sources by default.
/// Sample-aligned seek is required because the mixer cursor is the
/// authoritative time source.
pub trait Decoder: Send {
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u16;
    /// Pull interleaved f32 samples. Returns the number of frames written.
    fn read(&mut self, out: &mut [f32]) -> AudioResult<usize>;
    /// Seek to `position`, sample-aligned. Mixer asserts `position >= 0`.
    fn seek(&mut self, position: Samples) -> AudioResult<()>;
}

pub enum SoundSource {
    /// Short SFX decoded fully into memory.
    Pcm(Arc<[f32]>),
    /// Streaming source for songs.
    Streaming(Box<dyn Decoder + Send>),
}

impl std::fmt::Debug for SoundSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pcm(b) => f.debug_struct("Pcm").field("len", &b.len()).finish(),
            Self::Streaming(_) => f.debug_struct("Streaming").finish_non_exhaustive(),
        }
    }
}

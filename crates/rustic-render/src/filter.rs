//! Sprite filtering metadata. See `PLAN.md` Section 7.
//!
//! Filtering is asset metadata, not a global. HD assets default to
//! bilinear, pixel assets default to nearest. The renderer keeps one
//! sampler per mode and binds the right one at draw time.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FilterMode {
    /// Smooth bilinear; default for HD/vector-like FNF assets.
    #[default]
    Linear,
    /// Pixel-art-friendly nearest-neighbour; opt in via asset metadata.
    Nearest,
}

impl FilterMode {
    pub fn wgpu_filter(self) -> wgpu::FilterMode {
        match self {
            Self::Linear => wgpu::FilterMode::Linear,
            Self::Nearest => wgpu::FilterMode::Nearest,
        }
    }
}

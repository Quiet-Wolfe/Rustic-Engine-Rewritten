//! Cross-crate rendering enums. See `PLAN.md` Section 7.
//!
//! Only types that gameplay/asset code needs to talk about render order
//! live here. Pipeline objects, pipelines, atlases, GPU types stay in
//! `rustic-render`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RenderLayer {
    Background,
    Stage,
    Characters,
    Notes,
    Hud,
    Overlay,
    Debug,
}

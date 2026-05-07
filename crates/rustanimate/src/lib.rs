//! `rustanimate` — renderer-agnostic Rust port of `flxanimate`.
//!
//! Parses Adobe Animate texture atlases (`Animation.json` +
//! `spritemap1.json` + `spritemap1.png`) and yields flat 2D draw calls.
//! Required because the pinned base FNF baseline includes Week 7 /
//! Tankman-era assets that ship as Adobe Animate atlases rather than
//! Sparrow XML.
//!
//! Renderer-agnostic on purpose: depends only on `serde`, `serde_json`,
//! and `glam`. Consumed by `rustic-render` through its `animate` module.

#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(unsafe_code)]

pub mod animation;
pub mod atlas;
pub mod error;

pub use animation::{
    Animation, AnimationLabel, AtlasInstance, DrawPart, Element, ElementKind, Symbol,
    SymbolInstance, TimelineFrame, TimelineLayer,
};
pub use atlas::{Atlas, Frame, Sprite};
pub use error::{AnimateError, AnimateResult};

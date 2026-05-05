//! `rustic-render` — `wgpu`-backed 2D renderer. See `PLAN.md` Section 7.
//!
//! Render command flow:
//!   `RenderCommandList` (filled by gameplay/app)
//!     -> `SpriteBatcher::sort_and_batch` (sort by camera/layer/z, group by atlas)
//!     -> `SpritePipeline::draw` per camera into the reference 1280x720 target
//!     -> `Composite` blits reference target to native swapchain.

#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(unsafe_code)]

pub mod batch;
pub mod camera;
pub mod command;
pub mod composite;
pub mod error;
pub mod filter;
pub mod pipeline;
pub mod state;
pub mod texture;

pub use batch::{SpriteBatcher, SpriteInstance};
pub use camera::{Camera, CameraRegistry};
pub use command::{DrawCommand, RenderCommandList};
pub use composite::Composite;
pub use error::{RenderError, RenderResult};
pub use filter::FilterMode;
pub use pipeline::SpritePipeline;
pub use state::{RenderState, SurfaceConfig, REFERENCE_HEIGHT, REFERENCE_WIDTH};
pub use texture::Texture;

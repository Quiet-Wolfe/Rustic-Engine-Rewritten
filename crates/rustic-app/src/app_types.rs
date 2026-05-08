use rustic_render::{Composite, RenderState, SpritePipeline, SurfaceConfig};
use std::sync::Arc;
use winit::window::Window;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AppOptions {
    pub title: &'static str,
    pub width: u32,
    pub height: u32,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            title: "RusticV3",
            width: 1280,
            height: 720,
        }
    }
}

pub(crate) struct Runtime {
    pub(crate) window: Arc<Window>,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) surface_cfg: SurfaceConfig,
    pub(crate) rs: RenderState,
    pub(crate) pipeline: SpritePipeline,
    pub(crate) composite: Composite,
}

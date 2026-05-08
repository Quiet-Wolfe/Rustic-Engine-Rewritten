//! Window and renderer runtime creation for the app shell.

use crate::app_types::{AppOptions, Runtime};
use anyhow::Result;
use rustic_render::{Composite, RenderState, SpritePipeline};
use std::sync::Arc;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowAttributes;

pub(crate) fn create_runtime(
    options: &AppOptions,
    event_loop: &ActiveEventLoop,
) -> Result<Runtime> {
    let attrs = WindowAttributes::default()
        .with_title(options.title)
        .with_inner_size(winit::dpi::LogicalSize::new(options.width, options.height));
    let window = Arc::new(event_loop.create_window(attrs)?);
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let surface = instance
        .create_surface(window.clone())
        .map_err(|e| anyhow::anyhow!("create_surface: {e}"))?;
    let rs = pollster::block_on(RenderState::new_async(instance, Some(&surface)))?;
    let inner = window.inner_size();
    let surface_cfg = rs.configure_surface(
        &surface,
        inner.width,
        inner.height,
        wgpu::PresentMode::AutoVsync,
    )?;
    let pipeline = SpritePipeline::new(&rs.device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let composite = Composite::new(&rs, surface_cfg.format);
    Ok(Runtime {
        window,
        surface,
        surface_cfg,
        rs,
        pipeline,
        composite,
    })
}

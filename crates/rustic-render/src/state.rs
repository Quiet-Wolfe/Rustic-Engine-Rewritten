//! `RenderState` — owns wgpu instance/adapter/device/queue/surface, plus
//! the reference 1280x720 render target and shared samplers.
//!
//! See `PLAN.md` Section 7. This is the only place wgpu types are
//! constructed; everything else borrows references.

use crate::error::{RenderError, RenderResult};
use crate::filter::FilterMode;

/// Logical baseline. Gameplay HUD/coordinates live here; the surface can
/// be any size and the composite pass scales the reference target up.
pub const REFERENCE_WIDTH: u32 = 1280;
pub const REFERENCE_HEIGHT: u32 = 720;

/// Surface wrapper because we need to support both the windowed path
/// (with a real `wgpu::Surface`) and the headless path (regression /
/// tests).
#[derive(Debug)]
pub struct SurfaceConfig {
    pub format: wgpu::TextureFormat,
    pub width: u32,
    pub height: u32,
    pub present_mode: wgpu::PresentMode,
}

#[derive(Debug)]
pub struct RenderState {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// Reference render target. The composite pass blits from this into
    /// the swapchain (or into `regression_target` headlessly).
    pub reference_view: wgpu::TextureView,
    pub reference_texture: wgpu::Texture,
    /// Two pre-built samplers for the two `FilterMode` cases.
    pub sampler_linear: wgpu::Sampler,
    pub sampler_nearest: wgpu::Sampler,
    pub backend: wgpu::Backend,
    pub adapter_info: wgpu::AdapterInfo,
}

impl RenderState {
    /// Async-aware constructor. Callers wrap with `pollster::block_on`
    /// when they need a sync entry point.
    pub async fn new_async(
        instance: wgpu::Instance,
        compatible_surface: Option<&wgpu::Surface<'_>>,
    ) -> RenderResult<Self> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderError::NoAdapter)?;

        let adapter_info = adapter.get_info();
        let backend = adapter_info.backend;
        tracing::info!(
            target: "rustic.render",
            "wgpu adapter: {} ({:?}) backend={:?}",
            adapter_info.name,
            adapter_info.device_type,
            backend
        );

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("rustic.device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .map_err(|e| RenderError::Device(e.to_string()))?;

        let (reference_texture, reference_view) =
            create_reference_target(&device, REFERENCE_WIDTH, REFERENCE_HEIGHT);

        let sampler_linear = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("rustic.sampler.linear"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let sampler_nearest = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("rustic.sampler.nearest"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            reference_view,
            reference_texture,
            sampler_linear,
            sampler_nearest,
            backend,
            adapter_info,
        })
    }

    /// Sync helper for callers that don't want to manage an executor.
    pub fn new_blocking(
        instance: wgpu::Instance,
        compatible_surface: Option<&wgpu::Surface<'_>>,
    ) -> RenderResult<Self> {
        pollster::block_on(Self::new_async(instance, compatible_surface))
    }

    /// Headless variant for tests/regression. Uses no surface.
    pub async fn headless() -> RenderResult<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        Self::new_async(instance, None).await
    }

    /// Pick the matching pre-built sampler for the given filter metadata.
    pub fn sampler_for(&self, mode: FilterMode) -> &wgpu::Sampler {
        match mode {
            FilterMode::Linear => &self.sampler_linear,
            FilterMode::Nearest => &self.sampler_nearest,
        }
    }

    /// Configure a real `wgpu::Surface` for windowed presentation. The
    /// caller owns the `Surface`. Returns the picked surface format.
    pub fn configure_surface(
        &self,
        surface: &wgpu::Surface<'_>,
        width: u32,
        height: u32,
        present_mode: wgpu::PresentMode,
    ) -> RenderResult<SurfaceConfig> {
        let caps = surface.get_capabilities(&self.adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .ok_or_else(|| RenderError::Surface("no surface formats".into()))?;
        let chosen_present = if caps.present_modes.contains(&present_mode) {
            present_mode
        } else {
            caps.present_modes
                .first()
                .copied()
                .ok_or_else(|| RenderError::Surface("no present modes".into()))?
        };
        surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: width.max(1),
                height: height.max(1),
                present_mode: chosen_present,
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 1,
            },
        );
        Ok(SurfaceConfig {
            format,
            width,
            height,
            present_mode: chosen_present,
        })
    }
}

fn create_reference_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("rustic.reference_target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

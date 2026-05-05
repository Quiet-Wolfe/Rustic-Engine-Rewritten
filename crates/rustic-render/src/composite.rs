//! Composite pass: reference 1280x720 -> any output target.
//!
//! Uses a full-screen triangle. The triangle covers the entire NDC,
//! which the GPU clips to the viewport — so letterboxing is implemented
//! by setting the viewport to the largest integer multiple of the
//! reference that fits in the current output, centered.
//!
//! See `PLAN.md` Section 7.

use crate::state::{RenderState, REFERENCE_HEIGHT, REFERENCE_WIDTH};

const SHADER_SRC: &str = include_str!("../shaders/composite.wgsl");

#[derive(Debug)]
pub struct Composite {
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl Composite {
    pub fn new(rs: &RenderState, swapchain_format: wgpu::TextureFormat) -> Self {
        let module = rs
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("rustic.composite.shader"),
                source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
            });
        let bgl = rs
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("rustic.composite.bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let layout = rs
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("rustic.composite.layout"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });
        let pipeline = rs
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("rustic.composite.pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: swapchain_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });
        let bind_group = rs.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rustic.composite.bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&rs.reference_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&rs.sampler_linear),
                },
            ],
        });
        Self {
            pipeline,
            bgl,
            bind_group,
        }
    }

    /// Largest letterbox viewport that preserves the reference aspect
    /// ratio inside `(out_w, out_h)`. Centered.
    pub fn letterbox(out_w: u32, out_h: u32) -> (f32, f32, f32, f32) {
        let ref_aspect = REFERENCE_WIDTH as f32 / REFERENCE_HEIGHT as f32;
        let out_aspect = out_w as f32 / out_h.max(1) as f32;
        let (w, h) = if out_aspect > ref_aspect {
            (out_h as f32 * ref_aspect, out_h as f32)
        } else {
            (out_w as f32, out_w as f32 / ref_aspect)
        };
        let x = (out_w as f32 - w) * 0.5;
        let y = (out_h as f32 - h) * 0.5;
        (x, y, w, h)
    }

    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        out_w: u32,
        out_h: u32,
        clear: wgpu::Color,
    ) {
        let (x, y, w, h) = Self::letterbox(out_w, out_h);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("rustic.composite.pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_viewport(x, y, w, h, 0.0, 1.0);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Reuse the bind group layout so a caller can rebuild the bind
    /// group (e.g., after swapping the reference target on resize).
    pub fn bgl(&self) -> &wgpu::BindGroupLayout {
        &self.bgl
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn letterbox_preserves_aspect_horizontal() {
        // 1920x1080 surface with 1280x720 reference -> full fit.
        let (x, y, w, h) = Composite::letterbox(1920, 1080);
        assert!(x.abs() < 1.0);
        assert!(y.abs() < 1.0);
        assert!((w - 1920.0).abs() < 1.0);
        assert!((h - 1080.0).abs() < 1.0);
    }

    #[test]
    fn letterbox_pillarboxes_when_too_wide() {
        // Tall window: letterbox top/bottom, full width.
        let (x, y, _w, _h) = Composite::letterbox(640, 720);
        // 640x720 has aspect 0.888..., narrower than 16:9 -> pillarboxed in y.
        assert!(x.abs() < 1.0);
        assert!(y > 0.0);
    }
}

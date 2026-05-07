//! Sprite batcher. See `PLAN.md` Section 7.
//!
//! Sort key: `camera.order -> layer -> z -> insertion_index`. After
//! sort, runs of commands sharing (camera, atlas, sampler) become
//! single instanced draws.

use crate::camera::CameraRegistry;
use crate::command::DrawCommand;
use crate::filter::FilterMode;
use crate::pipeline::{CameraUniform, SpritePipeline};
use crate::state::{RenderState, REFERENCE_HEIGHT, REFERENCE_WIDTH};
use crate::texture::Texture;
use bytemuck::{Pod, Zeroable};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use std::collections::HashMap;

/// GPU instance row matching `INSTANCE_LAYOUT` in `pipeline.rs`.
/// Keep field order in sync with `ATTRIBUTES`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SpriteInstance {
    pub world_pos: [f32; 2],
    pub size: [f32; 2],
    pub pivot: [f32; 2],
    pub scale: [f32; 2],
    pub rotation: f32,
    pub _pad0: f32,
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub uv_rotated: f32,
    pub _pad1: f32,
    pub color: [f32; 4],
}

impl SpriteInstance {
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 9] = [
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 0,
            shader_location: 1,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 8,
            shader_location: 2,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 16,
            shader_location: 3,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 24,
            shader_location: 4,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32,
            offset: 32,
            shader_location: 5,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 40,
            shader_location: 6,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 48,
            shader_location: 7,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32,
            offset: 56,
            shader_location: 8,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 64,
            shader_location: 9,
        },
    ];
}

impl From<&DrawCommand> for SpriteInstance {
    fn from(c: &DrawCommand) -> Self {
        Self {
            world_pos: c.world_pos.to_array(),
            size: c.size.to_array(),
            pivot: c.pivot.to_array(),
            scale: c.scale.to_array(),
            rotation: c.rotation,
            _pad0: 0.0,
            uv_min: c.uv_min.to_array(),
            uv_max: c.uv_max.to_array(),
            uv_rotated: if c.uv_rotated { 1.0 } else { 0.0 },
            _pad1: 0.0,
            color: c.color.to_array(),
        }
    }
}

/// Stable sort key used for batching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SortKey {
    camera_order: i32,
    layer: u8,
    z: i32,
    insertion: u32,
}

impl SortKey {
    fn new(reg: &CameraRegistry, cmd: &DrawCommand, insertion: u32) -> Self {
        Self {
            camera_order: reg.order_key(cmd.camera),
            layer: layer_to_byte(cmd.layer),
            z: cmd.z,
            insertion,
        }
    }
}

fn layer_to_byte(l: RenderLayer) -> u8 {
    match l {
        RenderLayer::Background => 0,
        RenderLayer::Stage => 1,
        RenderLayer::Characters => 2,
        RenderLayer::Notes => 3,
        RenderLayer::Hud => 4,
        RenderLayer::Overlay => 5,
        RenderLayer::Debug => 6,
        // RenderLayer is `#[non_exhaustive]`. New layers added by core
        // must opt in to a sort position here; until then they sort
        // last under Debug.
        _ => 7,
    }
}

/// One contiguous run of instances sharing camera + atlas + sampler.
struct Run {
    camera: CameraId,
    atlas: AssetId,
    filter: FilterMode,
    instance_offset: u32,
    instance_count: u32,
}

#[derive(Debug, Default)]
pub struct SpriteBatcher {
    instances: Vec<SpriteInstance>,
    instance_buf: Option<wgpu::Buffer>,
    instance_capacity: u32,
    camera_uniform_buf: Option<wgpu::Buffer>,
}

impl SpriteBatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sort `cmds` and group them into atlas runs. Pure CPU work; no GPU
    /// state changes.
    fn sort_runs(reg: &CameraRegistry, cmds: &[DrawCommand]) -> (Vec<u32>, Vec<Run>) {
        let mut keyed: Vec<(SortKey, u32)> = cmds
            .iter()
            .enumerate()
            .map(|(i, c)| (SortKey::new(reg, c, i as u32), i as u32))
            .collect();
        keyed.sort_unstable_by_key(|(k, _)| *k);

        let mut order: Vec<u32> = Vec::with_capacity(keyed.len());
        let mut runs: Vec<Run> = Vec::new();
        for (_, idx) in &keyed {
            let cmd = &cmds[*idx as usize];
            let last = runs.last_mut();
            match last {
                Some(run)
                    if run.camera == cmd.camera
                        && run.atlas == cmd.texture
                        && run.filter == cmd.filter =>
                {
                    run.instance_count += 1;
                }
                _ => runs.push(Run {
                    camera: cmd.camera,
                    atlas: cmd.texture,
                    filter: cmd.filter,
                    instance_offset: order.len() as u32,
                    instance_count: 1,
                }),
            }
            order.push(*idx);
        }
        (order, runs)
    }

    /// Encode draws for the given commands into the reference target.
    /// Texture lookup is provided by `atlases`, which the app populates
    /// from `AssetResolver` reads.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_to_reference(
        &mut self,
        rs: &RenderState,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &SpritePipeline,
        cameras: &CameraRegistry,
        atlases: &HashMap<AssetId, Texture>,
        cmds: &[DrawCommand],
        clear_color: wgpu::Color,
    ) {
        let (order, runs) = Self::sort_runs(cameras, cmds);
        self.instances.clear();
        self.instances.reserve(order.len());
        for idx in &order {
            self.instances
                .push(SpriteInstance::from(&cmds[*idx as usize]));
        }
        self.upload_instances(rs);

        // Camera uniform buffer big enough for one Mat4 — we rebind per
        // run via a fresh BindGroup. (For v1 this is fine; gameplay
        // typically has 3 cameras and a few hundred sprites.)
        self.ensure_camera_uniform_buf(rs);
        let Some(cam_buf) = self.camera_uniform_buf.as_ref() else {
            return;
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("rustic.sprite.pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &rs.reference_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_vertex_buffer(0, pipeline.quad_vb.slice(..));
        if let Some(ib) = self.instance_buf.as_ref() {
            pass.set_vertex_buffer(1, ib.slice(..));
        }

        let mut current_camera: Option<CameraId> = None;
        let mut current_cam_bg: Option<wgpu::BindGroup> = None;

        for run in &runs {
            // Rebind camera if it changed.
            if current_camera != Some(run.camera) {
                if let Some(camera) = cameras.get(run.camera) {
                    let m = camera.view_proj(REFERENCE_WIDTH as f32, REFERENCE_HEIGHT as f32);
                    let uniform = CameraUniform {
                        view_proj: m.to_cols_array_2d(),
                    };
                    rs.queue
                        .write_buffer(cam_buf, 0, bytemuck::bytes_of(&uniform));
                    let bg = rs.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("rustic.sprite.camera_bg"),
                        layout: &pipeline.camera_bgl,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: cam_buf.as_entire_binding(),
                        }],
                    });
                    current_cam_bg = Some(bg);
                    current_camera = Some(run.camera);
                }
            }
            let Some(cam_bg) = current_cam_bg.as_ref() else {
                continue;
            };
            let Some(tex) = atlases.get(&run.atlas) else {
                continue;
            };

            let atlas_bg = rs.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("rustic.sprite.atlas_bg"),
                layout: &pipeline.atlas_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&tex.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(rs.sampler_for(run.filter)),
                    },
                ],
            });
            pass.set_bind_group(0, cam_bg, &[]);
            pass.set_bind_group(1, &atlas_bg, &[]);
            pass.draw(
                0..4,
                run.instance_offset..(run.instance_offset + run.instance_count),
            );
        }
    }

    fn ensure_camera_uniform_buf(&mut self, rs: &RenderState) {
        if self.camera_uniform_buf.is_none() {
            self.camera_uniform_buf = Some(rs.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("rustic.sprite.camera_ub"),
                size: std::mem::size_of::<CameraUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
    }

    fn upload_instances(&mut self, rs: &RenderState) {
        let needed = self.instances.len() as u32;
        if needed == 0 {
            return;
        }
        if needed > self.instance_capacity {
            let new_cap = needed.next_power_of_two().max(64);
            self.instance_buf = Some(rs.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("rustic.sprite.instance_buf"),
                size: (new_cap as u64) * std::mem::size_of::<SpriteInstance>() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.instance_capacity = new_cap;
        }
        if let Some(buf) = self.instance_buf.as_ref() {
            rs.queue
                .write_buffer(buf, 0, bytemuck::cast_slice(&self.instances));
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use glam::{Vec2, Vec4};
    use rustic_core::ids::{AssetId, CameraId};

    fn cmd(camera: u32, layer: RenderLayer, z: i32, atlas: u64) -> DrawCommand {
        DrawCommand {
            camera: CameraId(camera),
            layer,
            z,
            texture: AssetId(atlas),
            filter: FilterMode::Linear,
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
            uv_rotated: false,
            world_pos: Vec2::ZERO,
            size: Vec2::ONE,
            pivot: Vec2::splat(0.5),
            scale: Vec2::ONE,
            rotation: 0.0,
            color: Vec4::ONE,
        }
    }

    #[test]
    fn sort_key_orders_by_camera_then_layer_then_z() {
        let mut reg = CameraRegistry::new();
        reg.add(crate::camera::Camera::new(CameraId(1), "second", 1));
        reg.add(crate::camera::Camera::new(CameraId(0), "first", 0));

        let cmds = vec![
            cmd(1, RenderLayer::Hud, 0, 1),
            cmd(0, RenderLayer::Stage, 5, 1),
            cmd(0, RenderLayer::Stage, -3, 1),
            cmd(0, RenderLayer::Background, 0, 1),
        ];
        let (order, _) = SpriteBatcher::sort_runs(&reg, &cmds);
        // Camera 0 first (background -> stage z=-3 -> stage z=5), then camera 1 hud.
        assert_eq!(order, vec![3, 2, 1, 0]);
    }

    #[test]
    fn batches_share_runs_when_atlas_matches() {
        let mut reg = CameraRegistry::new();
        reg.add(crate::camera::Camera::new(CameraId(0), "g", 0));
        let cmds = vec![
            cmd(0, RenderLayer::Stage, 0, 7),
            cmd(0, RenderLayer::Stage, 1, 7),
            cmd(0, RenderLayer::Stage, 2, 9),
            cmd(0, RenderLayer::Stage, 3, 7),
        ];
        let (_, runs) = SpriteBatcher::sort_runs(&reg, &cmds);
        // Atlas changes break the run: 7 (2) -> 9 (1) -> 7 (1).
        let counts: Vec<u32> = runs.iter().map(|r| r.instance_count).collect();
        assert_eq!(counts, vec![2, 1, 1]);
    }
}

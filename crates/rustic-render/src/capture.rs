//! Headless reference-target capture. See `PLAN.md` Section 14.
//!
//! Used by `cargo xtask regression` and visual-regression tests. Renders
//! sprite + text commands to the 1280x720 reference target and reads back
//! the texels as RGBA8 bytes for PNG encoding or pixel diffing.

use crate::batch::SpriteBatcher;
use crate::camera::CameraRegistry;
use crate::command::DrawCommand;
use crate::error::{RenderError, RenderResult};
use crate::pipeline::SpritePipeline;
use crate::state::{RenderState, REFERENCE_HEIGHT, REFERENCE_WIDTH};
use crate::text::{TextCommand, TextSystem};
use crate::texture::Texture;
use rustic_core::ids::AssetId;
use std::collections::HashMap;

/// Capture the reference target as a tightly-packed RGBA8 row-major byte
/// buffer of length `REFERENCE_WIDTH * REFERENCE_HEIGHT * 4`.
#[allow(clippy::too_many_arguments)]
pub fn capture_reference_rgba(
    rs: &RenderState,
    pipeline: &SpritePipeline,
    batcher: &mut SpriteBatcher,
    text: Option<&mut TextSystem>,
    cameras: &CameraRegistry,
    atlases: &HashMap<AssetId, Texture>,
    sprite_cmds: &[DrawCommand],
    text_cmds: &[TextCommand],
    clear_color: wgpu::Color,
) -> RenderResult<Vec<u8>> {
    let mut encoder = rs
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("rustic.capture.encoder"),
        });
    let text_ref: Option<&TextSystem> = if let Some(text) = text {
        text.prepare(rs, text_cmds)?;
        Some(text)
    } else {
        None
    };
    batcher.draw_to_reference(
        rs,
        &mut encoder,
        pipeline,
        cameras,
        atlases,
        sprite_cmds,
        clear_color,
        text_ref,
    )?;

    let bytes_per_pixel: u32 = 4;
    let unpadded_bytes_per_row = REFERENCE_WIDTH * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
    let readback_size = u64::from(padded_bytes_per_row) * u64::from(REFERENCE_HEIGHT);

    let readback = rs.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("rustic.capture.readback"),
        size: readback_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &rs.reference_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &readback,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(REFERENCE_HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: REFERENCE_WIDTH,
            height: REFERENCE_HEIGHT,
            depth_or_array_layers: 1,
        },
    );

    rs.queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    rs.device.poll(wgpu::Maintain::Wait);
    receiver
        .recv()
        .map_err(|e| RenderError::Texture(format!("readback channel closed: {e}")))?
        .map_err(|e| RenderError::Texture(format!("buffer map failed: {e:?}")))?;

    let padded = slice.get_mapped_range();
    let mut tight = Vec::with_capacity((unpadded_bytes_per_row * REFERENCE_HEIGHT) as usize);
    for row in 0..REFERENCE_HEIGHT {
        let start = (row * padded_bytes_per_row) as usize;
        let end = start + unpadded_bytes_per_row as usize;
        tight.extend_from_slice(&padded[start..end]);
    }
    drop(padded);
    readback.unmap();
    Ok(tight)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn capture_buffer_is_reference_size() {
        let len = (REFERENCE_WIDTH * REFERENCE_HEIGHT * 4) as usize;
        assert_eq!(len, 1280 * 720 * 4);
    }
}

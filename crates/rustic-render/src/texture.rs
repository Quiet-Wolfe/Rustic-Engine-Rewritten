//! Texture upload path. See `PLAN.md` Section 7.
//!
//! Textures are renderer-owned: gameplay never touches `wgpu::Texture`.
//! The app loads typed image data via `rustic-asset`, passes RGBA pixels
//! here, and gets back an opaque `Texture` it can name with an `AssetId`.

use crate::filter::FilterMode;
use rustic_asset::PngImage;

#[derive(Debug)]
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub filter: FilterMode,
}

impl Texture {
    /// Upload decoded PNG RGBA8 pixels as an sRGB 2D texture.
    pub fn from_png_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &PngImage,
        filter: FilterMode,
        label: Option<&str>,
    ) -> Self {
        Self::from_rgba8(
            device,
            queue,
            &image.rgba,
            image.width,
            image.height,
            filter,
            label,
        )
    }

    pub fn from_rgba8(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rgba: &[u8],
        width: u32,
        height: u32,
        filter: FilterMode,
        label: Option<&str>,
    ) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            width,
            height,
            filter,
        }
    }
}

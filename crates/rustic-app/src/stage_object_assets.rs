//! Stage prop texture loading and render command wiring.

use anyhow::{Context, Result};
use rustic_asset::{
    load_png, load_sparrow, AssetPath, OverlayResolver, SparrowAtlas, SparrowFrame, StageObject,
};
use rustic_core::ids::AssetId;
use rustic_render::{DrawCommand, FilterMode, RenderCommandList, Texture};
use std::collections::HashMap;

pub(crate) fn load_stage_object(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    object: &StageObject,
    textures: &mut HashMap<AssetId, Texture>,
    commands: &mut RenderCommandList,
) -> Result<()> {
    if let Some(animation) = &object.animation {
        load_sparrow_stage_object(
            device,
            queue,
            resolver,
            object,
            &animation.prefix,
            textures,
            commands,
        )
    } else {
        load_png_stage_object(device, queue, resolver, object, textures, commands)
    }
}

fn load_png_stage_object(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    object: &StageObject,
    textures: &mut HashMap<AssetId, Texture>,
    commands: &mut RenderCommandList,
) -> Result<()> {
    let image = load_png(resolver, &object.image)
        .with_context(|| format!("load {}", object.image.as_str()))?;
    let texture_id = asset_id_for_path(&object.image);
    let filter = filter_for_antialiasing(object.antialiasing);
    let size = glam::vec2(
        image.width as f32 * object.scale.x,
        image.height as f32 * object.scale.y,
    );
    let texture =
        Texture::from_png_image(device, queue, &image, filter, Some(object.image.as_str()));
    textures.insert(texture_id, texture);
    commands.push(base_stage_command(
        object,
        texture_id,
        filter,
        glam::vec2(object.position.x, object.position.y),
        size,
    ));
    Ok(())
}

fn load_sparrow_stage_object(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    object: &StageObject,
    prefix: &str,
    textures: &mut HashMap<AssetId, Texture>,
    commands: &mut RenderCommandList,
) -> Result<()> {
    let atlas_path = stage_object_atlas_path(&object.image)?;
    let atlas = load_sparrow(resolver, &atlas_path)
        .with_context(|| format!("load {}", atlas_path.as_str()))?;
    let frame = atlas
        .first_animation_frame(prefix, &[])
        .with_context(|| format!("resolve stage prop frame {}:{prefix}", object.id))?;
    let texture_path = atlas_texture_path(&atlas_path, &atlas)?;
    let image = load_png(resolver, &texture_path)
        .with_context(|| format!("load {}", texture_path.as_str()))?;
    let texture_id = asset_id_for_path(&texture_path);
    let filter = filter_for_antialiasing(object.antialiasing);
    let texture =
        Texture::from_png_image(device, queue, &image, filter, Some(texture_path.as_str()));
    textures.insert(texture_id, texture);

    let mut cmd = base_stage_command(
        object,
        texture_id,
        filter,
        stage_frame_pos(object, frame),
        frame_draw_size(frame) * glam::vec2(object.scale.x, object.scale.y),
    );
    (cmd.uv_min, cmd.uv_max) = frame_uv(frame, image.width, image.height);
    cmd.uv_rotated = frame.rotated;
    commands.push(cmd);
    Ok(())
}

fn base_stage_command(
    object: &StageObject,
    texture_id: AssetId,
    filter: FilterMode,
    world_pos: glam::Vec2,
    size: glam::Vec2,
) -> DrawCommand {
    let mut cmd = DrawCommand::sprite(texture_id, world_pos, size);
    cmd.pivot = glam::Vec2::ZERO;
    cmd.layer = object.layer;
    cmd.z = object.z;
    cmd.filter = filter;
    cmd.scroll_factor = glam::vec2(object.scroll_factor.x, object.scroll_factor.y);
    cmd
}

fn stage_object_atlas_path(image: &AssetPath) -> Result<AssetPath> {
    let base = image
        .as_str()
        .strip_suffix(".png")
        .with_context(|| format!("stage prop image is not a png: {}", image.as_str()))?;
    Ok(AssetPath::new(format!("{base}.xml"))?)
}

fn atlas_texture_path(atlas_path: &AssetPath, atlas: &SparrowAtlas) -> Result<AssetPath> {
    if atlas.image_path.contains('/') {
        Ok(AssetPath::new(atlas.image_path.clone())?)
    } else {
        Ok(atlas_path.sibling(&atlas.image_path)?)
    }
}

fn stage_frame_pos(object: &StageObject, frame: &SparrowFrame) -> glam::Vec2 {
    glam::vec2(object.position.x, object.position.y)
        - glam::vec2(frame.frame_x as f32, frame.frame_y as f32)
            * glam::vec2(object.scale.x, object.scale.y)
}

fn frame_draw_size(frame: &SparrowFrame) -> glam::Vec2 {
    if frame.rotated {
        glam::vec2(frame.height as f32, frame.width as f32)
    } else {
        glam::vec2(frame.width as f32, frame.height as f32)
    }
}

fn frame_uv(
    frame: &SparrowFrame,
    texture_width: u32,
    texture_height: u32,
) -> (glam::Vec2, glam::Vec2) {
    let width = texture_width.max(1) as f32;
    let height = texture_height.max(1) as f32;
    (
        glam::vec2(frame.x as f32 / width, frame.y as f32 / height),
        glam::vec2(
            (frame.x as f32 + frame.width as f32) / width,
            (frame.y as f32 + frame.height as f32) / height,
        ),
    )
}

fn filter_for_antialiasing(antialiasing: bool) -> FilterMode {
    if antialiasing {
        FilterMode::Linear
    } else {
        FilterMode::Nearest
    }
}

fn asset_id_for_path(path: &AssetPath) -> AssetId {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in path.as_str().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    AssetId::new(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustic_asset::StageDefinition;

    #[test]
    fn stage_object_atlas_path_replaces_png_extension() {
        let path = AssetPath::new("images/erect/crowd.png").unwrap();

        assert_eq!(
            stage_object_atlas_path(&path).unwrap().as_str(),
            "images/erect/crowd.xml"
        );
    }

    #[test]
    fn animated_stage_prop_world_pos_applies_trim_offset() {
        let stage = StageDefinition::parse(
            br##"{
              "name": "test",
              "props": [{
                "name": "crowd",
                "assetPath": "erect/crowd",
                "position": [682, 290],
                "scale": [2, 3],
                "scroll": [1, 1],
                "zIndex": 5,
                "startingAnimation": "idle",
                "animations": [{
                  "name": "idle",
                  "prefix": "idle0",
                  "frameRate": 12,
                  "looped": true
                }]
              }]
            }"##,
        )
        .unwrap();
        let atlas = SparrowAtlas::parse(
            br#"<TextureAtlas imagePath="crowd.png">
              <SubTexture name="idle0000" x="0" y="0" width="100" height="80"
                frameX="-5" frameY="7" frameWidth="120" frameHeight="100"/>
            </TextureAtlas>"#,
        )
        .unwrap();
        let object = &stage.objects[0];
        let frame = atlas.first_animation_frame("idle0", &[]).unwrap();

        assert_eq!(stage_frame_pos(object, frame), glam::vec2(692.0, 269.0));
    }
}

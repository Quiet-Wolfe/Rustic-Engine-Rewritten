//! Adobe Animate atlas data shapes.

use crate::error::{AnimateError, AnimateResult};
use glam::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Frame {
    pub name: String,
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub size: Vec2,
    pub rotated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct Sprite {
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Atlas {
    pub sprites: Vec<Sprite>,
}

impl Atlas {
    pub fn parse_spritemap(bytes: &[u8]) -> AnimateResult<Self> {
        let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
        let raw: RawSpritemap = serde_json::from_slice(bytes)?;
        let meta = raw
            .atlas
            .meta
            .or(raw.meta)
            .ok_or_else(|| AnimateError::Atlas("spritemap metadata is missing".into()))?;
        let width = meta.size.w;
        let height = meta.size.h;
        if width <= 0.0 || height <= 0.0 {
            return Err(AnimateError::Atlas("spritemap texture size is zero".into()));
        }

        let mut frames = Vec::with_capacity(raw.atlas.sprites.len());
        for entry in raw.atlas.sprites {
            let sprite = entry.sprite;
            if sprite.name.trim().is_empty() {
                return Err(AnimateError::Atlas("sprite frame name is empty".into()));
            }
            if sprite.w <= 0.0 || sprite.h <= 0.0 {
                return Err(AnimateError::Atlas(format!(
                    "sprite frame {} has non-positive size",
                    sprite.name
                )));
            }
            frames.push(Frame {
                name: sprite.name,
                uv_min: Vec2::new(sprite.x / width, sprite.y / height),
                uv_max: Vec2::new(
                    (sprite.x + sprite.w) / width,
                    (sprite.y + sprite.h) / height,
                ),
                size: Vec2::new(sprite.w, sprite.h),
                rotated: sprite.rotated,
            });
        }

        Ok(Self {
            sprites: vec![Sprite { frames }],
        })
    }

    pub fn frame(&self, name: &str) -> Option<&Frame> {
        self.sprites
            .iter()
            .flat_map(|sprite| sprite.frames.iter())
            .find(|frame| frame.name == name)
    }
}

#[derive(Debug, Deserialize)]
struct RawSpritemap {
    #[serde(rename = "ATLAS")]
    atlas: RawAtlas,
    meta: Option<RawMeta>,
}

#[derive(Debug, Deserialize)]
struct RawAtlas {
    #[serde(rename = "SPRITES")]
    sprites: Vec<RawSpriteEntry>,
    meta: Option<RawMeta>,
}

#[derive(Debug, Deserialize)]
struct RawSpriteEntry {
    #[serde(rename = "SPRITE")]
    sprite: RawSprite,
}

#[derive(Debug, Deserialize)]
struct RawSprite {
    name: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    #[serde(default)]
    rotated: bool,
}

#[derive(Debug, Deserialize)]
struct RawMeta {
    size: RawSize,
}

#[derive(Debug, Deserialize)]
struct RawSize {
    w: f32,
    h: f32,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const SPRITEMAP: &[u8] = br#"{
      "ATLAS": {
        "SPRITES": [
          { "SPRITE": { "name": "0", "x": 10, "y": 20, "w": 30, "h": 40, "rotated": false } },
          { "SPRITE": { "name": "head", "x": 40, "y": 60, "w": 20, "h": 10, "rotated": true } }
        ],
        "meta": {
          "image": "spritemap1.png",
          "size": { "w": 100, "h": 200 }
        }
      }
    }"#;

    const TOP_LEVEL_META_SPRITEMAP: &[u8] = b"\xef\xbb\xbf{\"ATLAS\":{\"SPRITES\":[{\"SPRITE\":{\"name\":\"0\",\"x\":0,\"y\":0,\"w\":10,\"h\":20}}]},\"meta\":{\"size\":{\"w\":100,\"h\":200}}}";

    #[test]
    fn parses_spritemap_frames_and_uvs() {
        let atlas = Atlas::parse_spritemap(SPRITEMAP).unwrap();
        assert_eq!(atlas.sprites.len(), 1);
        assert_eq!(atlas.sprites[0].frames.len(), 2);

        let first = atlas.frame("0").unwrap();
        assert_eq!(first.uv_min, Vec2::new(0.1, 0.1));
        assert_eq!(first.uv_max, Vec2::new(0.4, 0.3));
        assert_eq!(first.size, Vec2::new(30.0, 40.0));
        assert!(!first.rotated);

        let rotated = atlas.frame("head").unwrap();
        assert_eq!(rotated.uv_min, Vec2::new(0.4, 0.3));
        assert_eq!(rotated.uv_max, Vec2::new(0.6, 0.35));
        assert!(rotated.rotated);
    }

    #[test]
    fn parses_top_level_meta_with_bom() {
        let atlas = Atlas::parse_spritemap(TOP_LEVEL_META_SPRITEMAP).unwrap();
        assert_eq!(atlas.frame("0").unwrap().size, Vec2::new(10.0, 20.0));
    }

    #[test]
    fn rejects_zero_texture_size() {
        let bad = br#"{
          "ATLAS": {
            "SPRITES": [],
            "meta": { "size": { "w": 0, "h": 200 } }
          }
        }"#;
        assert!(matches!(
            Atlas::parse_spritemap(bad),
            Err(AnimateError::Atlas(_))
        ));
    }

    #[test]
    fn rejects_empty_frame_names() {
        let bad = br#"{
          "ATLAS": {
            "SPRITES": [
              { "SPRITE": { "name": "", "x": 0, "y": 0, "w": 10, "h": 10 } }
            ],
            "meta": { "size": { "w": 100, "h": 100 } }
          }
        }"#;
        assert!(matches!(
            Atlas::parse_spritemap(bad),
            Err(AnimateError::Atlas(_))
        ));
    }
}

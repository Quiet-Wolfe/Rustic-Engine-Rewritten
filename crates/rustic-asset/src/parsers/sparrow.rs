//! Sparrow texture-atlas XML parser. See `PLAN.md` Sections 6 and 15.
//!
//! Format (base FNF / FlxAtlasFrames.fromSparrow):
//!
//! ```xml
//! <TextureAtlas imagePath="characters/BOYFRIEND.png">
//!   <SubTexture name="BF idle dance0001" x="0" y="0" width="221" height="271"
//!               frameX="-19" frameY="-15" frameWidth="240" frameHeight="290"/>
//!   ...
//! </TextureAtlas>
//! ```
//!
//! `frameX/frameY/frameWidth/frameHeight` are optional. When present
//! they describe the original (untrimmed) sprite bounds, with the
//! trimmed packed rect at `x,y,width,height`. The animation system uses
//! both to correctly position trimmed frames.
//!
//! Frame names typically end in a 4-digit zero-padded suffix that
//! identifies an animation index. Splitting names by suffix is the
//! caller's responsibility (the animation system needs a stable order
//! anyway), so the parser only returns a flat frame table.

use crate::error::{AssetError, AssetResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct SparrowFrame {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub frame_x: i32,
    pub frame_y: i32,
    pub frame_width: u32,
    pub frame_height: u32,
}

#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct SparrowAtlas {
    pub image_path: String,
    pub frames: Vec<SparrowFrame>,
}

impl SparrowAtlas {
    /// Parse Sparrow XML bytes. Errors only on malformed XML or
    /// non-numeric attributes. Missing optional `frame*` attributes
    /// default to the trimmed rect.
    pub fn parse(bytes: &[u8]) -> AssetResult<Self> {
        use quick_xml::events::Event;
        use quick_xml::reader::Reader;

        let mut reader = Reader::from_reader(bytes);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut atlas = SparrowAtlas::default();

        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => return Err(AssetError::InvalidPath(format!("sparrow xml: {e}"))),
                Ok(Event::Eof) => break,
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let name = e.name();
                    let local = name.as_ref();
                    if local == b"TextureAtlas" {
                        if let Some(p) = read_attr(&e, b"imagePath")? {
                            atlas.image_path = p;
                        }
                    } else if local == b"SubTexture" {
                        let frame = parse_subtexture(&e)?;
                        atlas.frames.push(frame);
                    }
                }
                _ => {}
            }
            buf.clear();
        }
        Ok(atlas)
    }

    pub fn animation_frames<'a>(&'a self, prefix: &str, indices: &[u16]) -> Vec<&'a SparrowFrame> {
        let mut frames: Vec<&SparrowFrame> = self
            .frames
            .iter()
            .filter(|frame| frame.name.starts_with(prefix))
            .collect();
        frames.sort_by_key(|frame| (frame_index(&frame.name).unwrap_or(u16::MAX), &frame.name));

        if indices.is_empty() {
            return frames;
        }

        let mut selected = Vec::with_capacity(indices.len());
        for index in indices {
            if let Some(frame) = frames
                .iter()
                .copied()
                .find(|frame| frame_index(&frame.name) == Some(*index))
            {
                selected.push(frame);
            }
        }
        selected
    }

    pub fn first_animation_frame(&self, prefix: &str, indices: &[u16]) -> Option<&SparrowFrame> {
        self.animation_frames(prefix, indices).into_iter().next()
    }
}

fn parse_subtexture(e: &quick_xml::events::BytesStart<'_>) -> AssetResult<SparrowFrame> {
    let name = read_attr(e, b"name")?
        .ok_or_else(|| AssetError::InvalidPath("SubTexture missing name".into()))?;
    let x = read_int(e, b"x", 0)?;
    let y = read_int(e, b"y", 0)?;
    let width = read_uint(e, b"width", 0)?;
    let height = read_uint(e, b"height", 0)?;
    let frame_x = read_int(e, b"frameX", 0)?;
    let frame_y = read_int(e, b"frameY", 0)?;
    let frame_width = read_uint(e, b"frameWidth", width)?;
    let frame_height = read_uint(e, b"frameHeight", height)?;
    Ok(SparrowFrame {
        name,
        x,
        y,
        width,
        height,
        frame_x,
        frame_y,
        frame_width,
        frame_height,
    })
}

fn read_attr(e: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> AssetResult<Option<String>> {
    for attr in e.attributes().with_checks(false) {
        let attr = attr.map_err(|err| AssetError::InvalidPath(format!("xml attr: {err}")))?;
        if attr.key.as_ref() == key {
            let v = attr
                .unescape_value()
                .map_err(|err| AssetError::InvalidPath(format!("xml unescape: {err}")))?;
            return Ok(Some(v.into_owned()));
        }
    }
    Ok(None)
}

fn read_int(e: &quick_xml::events::BytesStart<'_>, key: &[u8], default: i32) -> AssetResult<i32> {
    match read_attr(e, key)? {
        None => Ok(default),
        Some(s) => s.parse::<i32>().map_err(|_| {
            AssetError::InvalidPath(format!("non-int {}", String::from_utf8_lossy(key)))
        }),
    }
}

fn read_uint(e: &quick_xml::events::BytesStart<'_>, key: &[u8], default: u32) -> AssetResult<u32> {
    match read_attr(e, key)? {
        None => Ok(default),
        Some(s) => s.parse::<u32>().map_err(|_| {
            AssetError::InvalidPath(format!("non-uint {}", String::from_utf8_lossy(key)))
        }),
    }
}

fn frame_index(name: &str) -> Option<u16> {
    let digit_count = name
        .as_bytes()
        .iter()
        .rev()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digit_count == 0 {
        return None;
    }
    name[name.len() - digit_count..].parse().ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_atlas() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
        <TextureAtlas imagePath="characters/BF.png">
          <SubTexture name="BF idle dance0001" x="0" y="0" width="221" height="271"
                      frameX="-19" frameY="-15" frameWidth="240" frameHeight="290"/>
          <SubTexture name="BF idle dance0002" x="221" y="0" width="221" height="271"/>
        </TextureAtlas>"#;
        let a = SparrowAtlas::parse(xml).unwrap();
        assert_eq!(a.image_path, "characters/BF.png");
        assert_eq!(a.frames.len(), 2);

        let f0 = &a.frames[0];
        assert_eq!(f0.name, "BF idle dance0001");
        assert_eq!((f0.x, f0.y, f0.width, f0.height), (0, 0, 221, 271));
        assert_eq!(
            (f0.frame_x, f0.frame_y, f0.frame_width, f0.frame_height),
            (-19, -15, 240, 290)
        );

        // Optional frame* defaults: frame_width/height fall back to
        // packed width/height; frame_x/frame_y default to 0.
        let f1 = &a.frames[1];
        assert_eq!((f1.frame_x, f1.frame_y), (0, 0));
        assert_eq!((f1.frame_width, f1.frame_height), (221, 271));
    }

    #[test]
    fn selects_animation_frames_by_prefix_and_indices() {
        let xml = br#"<TextureAtlas imagePath="gf.png">
          <SubTexture name="GF Dancing Beat0015" x="0" y="0" width="10" height="10"/>
          <SubTexture name="GF Dancing Beat0000" x="10" y="0" width="10" height="10"/>
          <SubTexture name="GF Dancing Beat0030" x="20" y="0" width="10" height="10"/>
          <SubTexture name="GF Cheer0000" x="30" y="0" width="10" height="10"/>
        </TextureAtlas>"#;
        let atlas = SparrowAtlas::parse(xml).unwrap();

        let frames = atlas.animation_frames("GF Dancing Beat", &[]);
        let names: Vec<_> = frames.iter().map(|frame| frame.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "GF Dancing Beat0000",
                "GF Dancing Beat0015",
                "GF Dancing Beat0030"
            ]
        );

        let frames = atlas.animation_frames("GF Dancing Beat", &[30, 0]);
        let names: Vec<_> = frames.iter().map(|frame| frame.name.as_str()).collect();
        assert_eq!(names, vec!["GF Dancing Beat0030", "GF Dancing Beat0000"]);
    }

    #[test]
    fn rejects_subtexture_without_name() {
        let xml = br#"<TextureAtlas><SubTexture x="0"/></TextureAtlas>"#;
        assert!(SparrowAtlas::parse(xml).is_err());
    }

    #[test]
    fn rejects_non_numeric_attributes() {
        let xml = br#"<TextureAtlas><SubTexture name="x" x="abc"/></TextureAtlas>"#;
        assert!(SparrowAtlas::parse(xml).is_err());
    }
}

//! BMFont XML parser for bitmap fallback fonts.

use crate::error::{AssetError, AssetResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BitmapFont {
    pub face: String,
    pub size: i32,
    pub line_height: u32,
    pub base: i32,
    pub scale_w: u32,
    pub scale_h: u32,
    pub pages: Vec<BitmapFontPage>,
    pub glyphs: Vec<BitmapGlyph>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BitmapFontPage {
    pub id: u32,
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BitmapGlyph {
    pub id: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub xoffset: i32,
    pub yoffset: i32,
    pub xadvance: i32,
    pub page: u32,
    pub chnl: u32,
}

impl BitmapFont {
    pub fn parse(bytes: &[u8]) -> AssetResult<Self> {
        use quick_xml::events::Event;
        use quick_xml::reader::Reader;

        let mut reader = Reader::from_reader(bytes);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut font = BitmapFont::default();

        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => return Err(AssetError::InvalidPath(format!("bmfont xml: {e}"))),
                Ok(Event::Eof) => break,
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => match e.name().as_ref() {
                    b"info" => {
                        font.face = read_attr(&e, b"face")?.unwrap_or_default();
                        font.size = read_int(&e, b"size", 0)?;
                    }
                    b"common" => {
                        font.line_height = read_uint(&e, b"lineHeight", 0)?;
                        font.base = read_int(&e, b"base", 0)?;
                        font.scale_w = read_uint(&e, b"scaleW", 0)?;
                        font.scale_h = read_uint(&e, b"scaleH", 0)?;
                    }
                    b"page" => font.pages.push(BitmapFontPage {
                        id: read_uint(&e, b"id", 0)?,
                        file: read_attr(&e, b"file")?.unwrap_or_default(),
                    }),
                    b"char" => font.glyphs.push(parse_glyph(&e)?),
                    _ => {}
                },
                _ => {}
            }
            buf.clear();
        }

        Ok(font)
    }

    pub fn glyph(&self, id: u32) -> Option<&BitmapGlyph> {
        self.glyphs.iter().find(|glyph| glyph.id == id)
    }
}

fn parse_glyph(e: &quick_xml::events::BytesStart<'_>) -> AssetResult<BitmapGlyph> {
    Ok(BitmapGlyph {
        id: read_uint(e, b"id", 0)?,
        x: read_uint(e, b"x", 0)?,
        y: read_uint(e, b"y", 0)?,
        width: read_uint(e, b"width", 0)?,
        height: read_uint(e, b"height", 0)?,
        xoffset: read_int(e, b"xoffset", 0)?,
        yoffset: read_int(e, b"yoffset", 0)?,
        xadvance: read_int(e, b"xadvance", 0)?,
        page: read_uint(e, b"page", 0)?,
        chnl: read_uint(e, b"chnl", 0)?,
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn parses_bitmap_font_metrics_and_glyphs() {
        let font = BitmapFont::parse(
            br#"<font>
              <info face="vcr-bmp" size="16"/>
              <common lineHeight="18" base="14" scaleW="122" scaleH="122"/>
              <pages><page id="0" file="vcr-bmp.png"/></pages>
              <chars count="2">
                <char id="32" x="0" y="0" width="0" height="0"
                  xoffset="0" yoffset="0" xadvance="10" page="0" chnl="15"/>
                <char id="65" x="27" y="32" width="11" height="14"
                  xoffset="-1" yoffset="2" xadvance="10" page="0" chnl="15"/>
              </chars>
            </font>"#,
        )
        .unwrap();

        assert_eq!(font.face, "vcr-bmp");
        assert_eq!(font.line_height, 18);
        assert_eq!(font.pages[0].file, "vcr-bmp.png");
        assert_eq!(font.glyph(65).unwrap().xoffset, -1);
    }

    #[test]
    fn rejects_non_numeric_glyph_attributes() {
        let bad = br#"<font><chars><char id="A"/></chars></font>"#;
        assert!(BitmapFont::parse(bad).is_err());
    }
}

//! PNG decoder for image assets.
//!
//! `rustic-asset` owns image decoding so app/game code only deals in typed
//! asset data and renderer upload code only deals in raw RGBA pixels.

use crate::error::{AssetError, AssetResult};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PngImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl PngImage {
    pub fn parse(bytes: &[u8]) -> AssetResult<Self> {
        let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
            .map_err(|e| AssetError::InvalidData(format!("png: {e}")))?
            .to_rgba8();
        let (width, height) = image.dimensions();
        Ok(Self {
            width,
            height,
            rgba: image.into_raw(),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use image::ImageEncoder;

    fn tiny_png() -> Vec<u8> {
        let rgba = [
            255, 0, 0, 255, //
            0, 255, 0, 255,
        ];
        let mut out = Vec::new();
        image::codecs::png::PngEncoder::new(&mut out)
            .write_image(&rgba, 2, 1, image::ColorType::Rgba8.into())
            .unwrap();
        out
    }

    #[test]
    fn decodes_png_as_rgba8() {
        let decoded = PngImage::parse(&tiny_png()).unwrap();
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 1);
        assert_eq!(
            decoded.rgba,
            vec![
                255, 0, 0, 255, //
                0, 255, 0, 255
            ]
        );
    }

    #[test]
    fn rejects_non_png_bytes() {
        assert!(PngImage::parse(b"not a png").is_err());
    }
}

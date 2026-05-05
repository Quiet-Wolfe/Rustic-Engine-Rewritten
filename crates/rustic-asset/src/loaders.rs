//! Typed loaders that go through the resolver. Callers in release crates
//! load assets via these helpers; only `rustic-asset`/`xtask`/`rustic-dev`
//! touch the filesystem directly.

use crate::error::AssetResult;
use crate::parsers::{
    character::CharacterDefinition, chart::ParsedSong, png::PngImage, sparrow::SparrowAtlas,
    stage::StageDefinition,
};
use crate::path::AssetPath;
use crate::resolver::AssetResolver;

pub fn load_chart(resolver: &dyn AssetResolver, path: &AssetPath) -> AssetResult<ParsedSong> {
    let src = resolver.resolve(path)?;
    let bytes = src.read_all()?;
    ParsedSong::parse(&bytes)
}

pub fn load_sparrow(resolver: &dyn AssetResolver, path: &AssetPath) -> AssetResult<SparrowAtlas> {
    let src = resolver.resolve(path)?;
    let bytes = src.read_all()?;
    SparrowAtlas::parse(&bytes)
}

pub fn load_png(resolver: &dyn AssetResolver, path: &AssetPath) -> AssetResult<PngImage> {
    let src = resolver.resolve(path)?;
    let bytes = src.read_all()?;
    PngImage::parse(&bytes)
}

pub fn load_character(
    resolver: &dyn AssetResolver,
    path: &AssetPath,
) -> AssetResult<CharacterDefinition> {
    let src = resolver.resolve(path)?;
    let bytes = src.read_all()?;
    CharacterDefinition::parse(&bytes)
}

pub fn load_stage(resolver: &dyn AssetResolver, path: &AssetPath) -> AssetResult<StageDefinition> {
    let src = resolver.resolve(path)?;
    let bytes = src.read_all()?;
    StageDefinition::parse(&bytes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::resolver::{InMemoryLayer, OverlayResolver};

    fn ap(s: &str) -> AssetPath {
        AssetPath::new(s).unwrap()
    }

    const CHART_JSON: &str = r#"{
        "song": {
            "song": "Test",
            "bpm": 100.0,
            "notes": []
        }
    }"#;

    const SPARROW_XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<TextureAtlas imagePath="bf.png">
  <SubTexture name="bf idle0000" x="0" y="0" width="100" height="100"
              frameX="-2" frameY="-3" frameWidth="104" frameHeight="106"/>
</TextureAtlas>"#;

    const CHARACTER_JSON: &str = r#"{
        "id": "bf",
        "atlas": "images/BOYFRIEND.xml",
        "animations": [{ "name": "idle", "prefix": "BF idle dance" }]
    }"#;

    const STAGE_JSON: &str = r#"{
        "id": "stage",
        "objects": [{ "id": "stageback", "image": "images/stageback.png" }]
    }"#;

    fn tiny_png() -> Vec<u8> {
        use image::ImageEncoder;

        let rgba = [
            1, 2, 3, 255, //
            4, 5, 6, 255,
        ];
        let mut out = Vec::new();
        image::codecs::png::PngEncoder::new(&mut out)
            .write_image(&rgba, 2, 1, image::ColorType::Rgba8.into())
            .unwrap();
        out
    }

    #[test]
    fn load_chart_through_resolver() {
        let mut resolver = OverlayResolver::new();
        let mut overlay = InMemoryLayer::new();
        overlay.insert(ap("songs/test/test.json"), CHART_JSON.as_bytes().to_vec());
        resolver.push_overlay(overlay);

        let song = load_chart(&resolver, &ap("songs/test/test.json")).unwrap();
        assert_eq!(song.name, "Test");
        assert_eq!(song.chart.bpm, 100.0);
    }

    #[test]
    fn load_sparrow_through_resolver() {
        let mut resolver = OverlayResolver::new();
        let mut overlay = InMemoryLayer::new();
        overlay.insert(ap("images/bf.xml"), SPARROW_XML.as_bytes().to_vec());
        resolver.push_overlay(overlay);

        let atlas = load_sparrow(&resolver, &ap("images/bf.xml")).unwrap();
        assert_eq!(atlas.image_path, "bf.png");
        assert_eq!(atlas.frames.len(), 1);
    }

    #[test]
    fn load_png_through_resolver() {
        let mut resolver = OverlayResolver::new();
        let mut overlay = InMemoryLayer::new();
        overlay.insert(ap("images/test.png"), tiny_png());
        resolver.push_overlay(overlay);

        let image = load_png(&resolver, &ap("images/test.png")).unwrap();
        assert_eq!((image.width, image.height), (2, 1));
        assert_eq!(image.rgba.len(), 8);
    }

    #[test]
    fn load_character_through_resolver() {
        let mut resolver = OverlayResolver::new();
        let mut overlay = InMemoryLayer::new();
        overlay.insert(
            ap("data/characters/bf.json"),
            CHARACTER_JSON.as_bytes().to_vec(),
        );
        resolver.push_overlay(overlay);

        let character = load_character(&resolver, &ap("data/characters/bf.json")).unwrap();
        assert_eq!(character.id, "bf");
        assert_eq!(character.animations[0].name, "idle");
    }

    #[test]
    fn load_stage_through_resolver() {
        let mut resolver = OverlayResolver::new();
        let mut overlay = InMemoryLayer::new();
        overlay.insert(ap("data/stages/stage.json"), STAGE_JSON.as_bytes().to_vec());
        resolver.push_overlay(overlay);

        let stage = load_stage(&resolver, &ap("data/stages/stage.json")).unwrap();
        assert_eq!(stage.id, "stage");
        assert_eq!(stage.objects[0].image.as_str(), "images/stageback.png");
    }
}

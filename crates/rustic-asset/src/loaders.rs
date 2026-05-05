//! Typed loaders that go through the resolver. Callers in release crates
//! load assets via these helpers; only `rustic-asset`/`xtask`/`rustic-dev`
//! touch the filesystem directly.

use crate::error::AssetResult;
use crate::parsers::{chart::ParsedSong, sparrow::SparrowAtlas};
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
}

//! Typed loaders that go through the resolver. Callers in release crates
//! load assets via these helpers; only `rustic-asset`/`xtask`/`rustic-dev`
//! touch the filesystem directly.
// LINT-ALLOW: long-file typed loader tests cover tracked source assets together.

use crate::error::{AssetError, AssetResult};
use crate::parsers::{
    character::CharacterDefinition, chart::ParsedSong, font::BitmapFont, level::LevelDefinition,
    png::PngImage, sparrow::SparrowAtlas, stage::StageDefinition, text_list::TextList,
};
use crate::path::AssetPath;
use crate::resolver::AssetResolver;
use rustanimate::{Animation as AnimateAnimation, Atlas as AnimateAtlas};

pub fn load_chart(resolver: &dyn AssetResolver, path: &AssetPath) -> AssetResult<ParsedSong> {
    let src = resolver.resolve(path)?;
    let bytes = src.read_all()?;
    ParsedSong::parse(&bytes)
}

pub fn load_vslice_chart(
    resolver: &dyn AssetResolver,
    chart_path: &AssetPath,
    metadata_path: &AssetPath,
    difficulty: &str,
) -> AssetResult<ParsedSong> {
    let chart_src = resolver.resolve(chart_path)?;
    let chart_bytes = chart_src.read_all()?;
    let metadata_src = resolver.resolve(metadata_path)?;
    let metadata_bytes = metadata_src.read_all()?;
    ParsedSong::parse_vslice(&chart_bytes, &metadata_bytes, difficulty)
}

pub fn load_sparrow(resolver: &dyn AssetResolver, path: &AssetPath) -> AssetResult<SparrowAtlas> {
    let src = resolver.resolve(path)?;
    let bytes = src.read_all()?;
    SparrowAtlas::parse(&bytes)
}

pub fn load_animate_animation(
    resolver: &dyn AssetResolver,
    path: &AssetPath,
) -> AssetResult<AnimateAnimation> {
    let src = resolver.resolve(path)?;
    let bytes = src.read_all()?;
    AnimateAnimation::parse(&bytes).map_err(invalid_animate_data)
}

pub fn load_animate_spritemap(
    resolver: &dyn AssetResolver,
    path: &AssetPath,
) -> AssetResult<AnimateAtlas> {
    let src = resolver.resolve(path)?;
    let bytes = src.read_all()?;
    AnimateAtlas::parse_spritemap(&bytes).map_err(invalid_animate_data)
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

pub fn load_level(resolver: &dyn AssetResolver, path: &AssetPath) -> AssetResult<LevelDefinition> {
    let src = resolver.resolve(path)?;
    let bytes = src.read_all()?;
    LevelDefinition::parse(&bytes)
}

pub fn load_text_list(resolver: &dyn AssetResolver, path: &AssetPath) -> AssetResult<TextList> {
    let src = resolver.resolve(path)?;
    let bytes = src.read_all()?;
    TextList::parse(&bytes)
}

pub fn load_bitmap_font(resolver: &dyn AssetResolver, path: &AssetPath) -> AssetResult<BitmapFont> {
    let src = resolver.resolve(path)?;
    let bytes = src.read_all()?;
    BitmapFont::parse(&bytes)
}

pub fn load_bytes(
    resolver: &dyn AssetResolver,
    path: &AssetPath,
) -> AssetResult<std::sync::Arc<[u8]>> {
    let src = resolver.resolve(path)?;
    src.read_all()
}

fn invalid_animate_data(error: rustanimate::AnimateError) -> AssetError {
    AssetError::InvalidData(format!("animate atlas: {error}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::resolver::{InMemoryLayer, OverlayResolver};

    fn ap(s: &str) -> AssetPath {
        AssetPath::new(s).unwrap()
    }

    fn source_resolver() -> OverlayResolver {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace = manifest_dir.parent().unwrap().parent().unwrap();
        OverlayResolver::new().with_baked_root(workspace.join("assets/source"))
    }

    const CHART_JSON: &str = r#"{
        "song": {
            "song": "Test",
            "bpm": 100.0,
            "notes": []
        }
    }"#;

    const VSLICE_CHART_JSON: &str = r#"{
        "scrollSpeed": { "normal": 1.3 },
        "notes": { "normal": [{ "t": 100.0, "d": 0 }] }
    }"#;

    const VSLICE_METADATA_JSON: &str = r#"{
        "songName": "Bopeebo",
        "playData": { "characters": { "player": "bf", "opponent": "dad" } },
        "timeChanges": [{ "bpm": 100 }]
    }"#;

    const SPARROW_XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<TextureAtlas imagePath="bf.png">
  <SubTexture name="bf idle0000" x="0" y="0" width="100" height="100"
              frameX="-2" frameY="-3" frameWidth="104" frameHeight="106"/>
</TextureAtlas>"#;

    const ANIMATE_JSON: &str = r#"{
      "AN": {
        "N": "Dad_assets_TA-Export",
        "SN": "DAD ALL ANIMS",
        "TL": {
          "L": [{
            "FR": [{ "N": "Idle", "I": 0, "DU": 12, "E": [] }]
          }]
        }
      }
    }"#;

    const SPRITEMAP_JSON: &str = r#"{
      "ATLAS": {
        "SPRITES": [
          { "SPRITE": { "name": "0", "x": 10, "y": 20, "w": 30, "h": 40 } }
        ],
        "meta": { "size": { "w": 100, "h": 200 } }
      }
    }"#;

    const CHARACTER_JSON: &str = r#"{
        "id": "bf",
        "atlas": "images/BOYFRIEND.xml",
        "animations": [{ "name": "idle", "prefix": "BF idle dance" }]
    }"#;

    const STAGE_JSON: &str = r#"{
        "id": "stage",
        "objects": [{ "id": "stageback", "image": "images/stageback.png" }]
    }"#;

    const LEVEL_JSON: &str = r#"{
        "name": "DADDY DEAREST",
        "titleAsset": "storymenu/titles/week1",
        "props": [{ "assetPath": "storymenu/props/dad", "offsets": [100, 60] }],
        "songs": ["bopeebo", "fresh", "dadbattle"]
    }"#;

    const FREEPLAY_LIST: &str = "Tutorial\nBopeebo\nFresh\nDadbattle\n";
    const BITMAP_FONT_XML: &str = r#"<font>
      <info face="vcr-bmp" size="16"/>
      <common lineHeight="18" base="14" scaleW="122" scaleH="122"/>
      <pages><page id="0" file="vcr-bmp.png"/></pages>
      <chars count="1">
        <char id="65" x="27" y="32" width="11" height="14"
          xoffset="-1" yoffset="2" xadvance="10" page="0" chnl="15"/>
      </chars>
    </font>"#;

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
    fn load_vslice_chart_through_resolver() {
        let mut resolver = OverlayResolver::new();
        let mut overlay = InMemoryLayer::new();
        overlay.insert(
            ap("data/songs/bopeebo/bopeebo-chart.json"),
            VSLICE_CHART_JSON.as_bytes().to_vec(),
        );
        overlay.insert(
            ap("data/songs/bopeebo/bopeebo-metadata.json"),
            VSLICE_METADATA_JSON.as_bytes().to_vec(),
        );
        resolver.push_overlay(overlay);

        let song = load_vslice_chart(
            &resolver,
            &ap("data/songs/bopeebo/bopeebo-chart.json"),
            &ap("data/songs/bopeebo/bopeebo-metadata.json"),
            "normal",
        )
        .unwrap();
        assert_eq!(song.name, "Bopeebo");
        assert_eq!(song.chart.speed, 1.3);
        assert_eq!(song.chart.notes.len(), 1);
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
    fn load_animate_animation_through_resolver() {
        let mut resolver = OverlayResolver::new();
        let mut overlay = InMemoryLayer::new();
        overlay.insert(
            ap("images/characters/dad/Animation.json"),
            ANIMATE_JSON.as_bytes().to_vec(),
        );
        resolver.push_overlay(overlay);

        let animation =
            load_animate_animation(&resolver, &ap("images/characters/dad/Animation.json")).unwrap();
        assert_eq!(animation.symbol_name, "DAD ALL ANIMS");
        assert_eq!(animation.label("Idle").unwrap().duration, 12);
    }

    #[test]
    fn load_animate_spritemap_through_resolver() {
        let mut resolver = OverlayResolver::new();
        let mut overlay = InMemoryLayer::new();
        overlay.insert(
            ap("images/characters/dad/spritemap1.json"),
            SPRITEMAP_JSON.as_bytes().to_vec(),
        );
        resolver.push_overlay(overlay);

        let atlas = load_animate_spritemap(&resolver, &ap("images/characters/dad/spritemap1.json"))
            .unwrap();
        let frame = atlas.frame("0").unwrap();
        assert_eq!((frame.size.x, frame.size.y), (30.0, 40.0));
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

    #[test]
    fn load_level_through_resolver() {
        let mut resolver = OverlayResolver::new();
        let mut overlay = InMemoryLayer::new();
        overlay.insert(ap("data/levels/week1.json"), LEVEL_JSON.as_bytes().to_vec());
        resolver.push_overlay(overlay);

        let level = load_level(&resolver, &ap("data/levels/week1.json")).unwrap();
        assert_eq!(level.title_asset, "storymenu/titles/week1");
        assert_eq!(level.songs, vec!["bopeebo", "fresh", "dadbattle"]);
    }

    #[test]
    fn load_text_list_through_resolver() {
        let mut resolver = OverlayResolver::new();
        let mut overlay = InMemoryLayer::new();
        overlay.insert(
            ap("data/freeplaySonglist.txt"),
            FREEPLAY_LIST.as_bytes().to_vec(),
        );
        resolver.push_overlay(overlay);

        let list = load_text_list(&resolver, &ap("data/freeplaySonglist.txt")).unwrap();
        assert_eq!(
            list.items,
            vec!["Tutorial", "Bopeebo", "Fresh", "Dadbattle"]
        );
    }

    #[test]
    fn load_bitmap_font_through_resolver() {
        let mut resolver = OverlayResolver::new();
        let mut overlay = InMemoryLayer::new();
        overlay.insert(ap("fonts/vcr-bmp.fnt"), BITMAP_FONT_XML.as_bytes().to_vec());
        resolver.push_overlay(overlay);

        let font = load_bitmap_font(&resolver, &ap("fonts/vcr-bmp.fnt")).unwrap();
        assert_eq!(font.face, "vcr-bmp");
        assert_eq!(font.glyph(65).unwrap().xadvance, 10);
    }

    #[test]
    fn load_bytes_through_resolver() {
        let mut resolver = OverlayResolver::new();
        let mut overlay = InMemoryLayer::new();
        overlay.insert(ap("music/test.ogg"), b"OggSxxxx".to_vec());
        resolver.push_overlay(overlay);

        let bytes = load_bytes(&resolver, &ap("music/test.ogg")).unwrap();
        assert_eq!(&bytes[..4], b"OggS");
    }

    #[test]
    fn tracked_source_seed_definitions_parse() {
        let resolver = source_resolver();

        let bf = load_character(&resolver, &ap("data/characters/bf.json")).unwrap();
        assert_eq!(bf.id, "Boyfriend");
        assert_eq!(
            bf.render_type,
            crate::parsers::character::CharacterRenderType::MultiAnimateAtlas
        );
        assert_eq!(
            bf.asset_path.as_ref().unwrap().as_str(),
            "shared:characters/bf"
        );
        assert_eq!(bf.animations.len(), 16);
        assert_eq!(
            bf.animations[11].asset_path.as_ref().unwrap().as_str(),
            "shared:characters/bf-death"
        );

        let dad = load_character(&resolver, &ap("data/characters/dad.json")).unwrap();
        assert_eq!(dad.id, "Daddy Dearest");
        assert_eq!(
            dad.render_type,
            crate::parsers::character::CharacterRenderType::AnimateAtlas
        );
        assert_eq!(dad.animations.len(), 9);
        assert_eq!(dad.animations[1].indices, vec![11, 12, 0, 1]);

        let gf = load_character(&resolver, &ap("data/characters/gf.json")).unwrap();
        assert_eq!(gf.id, "Girlfriend");
        assert_eq!(gf.initial_animation.as_deref(), Some("danceRight"));
        assert_eq!(gf.animations.len(), 12);
        assert_eq!(gf.animations[1].indices.len(), 15);
        assert_eq!(gf.animations[11].indices.len(), 12);

        let stage = load_stage(&resolver, &ap("data/stages/stage.json")).unwrap();
        assert_eq!(stage.id, "stage");
        assert_eq!(stage.objects.len(), 3);
        let main_stage = load_stage(&resolver, &ap("data/stages/mainStage.json")).unwrap();
        assert_eq!(main_stage.id, "Main Stage");
        assert_eq!(main_stage.objects[0].image.as_str(), "images/stageback.png");

        let tutorial = load_level(&resolver, &ap("data/levels/tutorial.json")).unwrap();
        assert_eq!(tutorial.songs, vec!["tutorial"]);
        let week1 = load_level(&resolver, &ap("data/levels/week1.json")).unwrap();
        assert_eq!(week1.songs, vec!["bopeebo", "fresh", "dadbattle"]);

        let songs = load_text_list(&resolver, &ap("data/freeplaySonglist.txt")).unwrap();
        assert_eq!(
            songs.items,
            vec![
                "Tutorial",
                "Bopeebo",
                "Fresh",
                "DadBattle",
                "Spookeez",
                "South",
                "Monster",
                "Pico",
                "Philly Nice",
                "Blammed",
                "Satin Panties",
                "High",
                "M.I.L.F",
                "Cocoa",
                "Eggnog",
                "Winter Horrorland",
                "Senpai",
                "Roses",
                "Thorns",
                "Ugh",
                "Guns",
                "Stress",
                "Darnell",
                "Lit Up",
                "2hot",
                "Blazin'",
            ]
        );

        let font = load_bitmap_font(&resolver, &ap("fonts/vcr-bmp.fnt")).unwrap();
        assert_eq!(font.face, "vcr-bmp");
        assert_eq!(font.glyphs.len(), 90);
        assert_eq!(font.glyph(u32::from('A')).unwrap().xadvance, 10);
    }

    #[test]
    fn tracked_source_week1_charts_parse() {
        let resolver = source_resolver();

        for (path, name) in [
            ("data/tutorial/tutorial-easy.json", "Tutorial"),
            ("data/tutorial/tutorial.json", "Tutorial"),
            ("data/tutorial/tutorial-hard.json", "Tutorial"),
            ("data/bopeebo/bopeebo.json", "Bopeebo"),
            ("data/fresh/fresh-easy.json", "Fresh"),
            ("data/fresh/fresh.json", "Fresh"),
            ("data/fresh/fresh-hard.json", "Fresh"),
            ("data/dadbattle/dadbattle-easy.json", "Dadbattle"),
            ("data/dadbattle/dadbattle.json", "Dadbattle"),
            ("data/dadbattle/dadbattle-hard.json", "Dadbattle"),
        ] {
            let song = load_chart(&resolver, &ap(path)).unwrap();
            assert_eq!(song.name, name);
            assert!(song.chart.valid_score);
            assert!(!song.chart.notes.is_empty());
        }
    }

    #[test]
    fn tracked_source_week1_vslice_charts_parse() {
        let resolver = source_resolver();

        for (song_id, name, speed, player, opponent) in [
            ("tutorial", "Tutorial", 1.0, "bf", "gf"),
            ("bopeebo", "Bopeebo", 1.3, "bf", "dad"),
            ("fresh", "Fresh", 1.6, "bf", "dad"),
            ("dadbattle", "DadBattle", 1.8, "bf", "dad"),
        ] {
            let song = load_vslice_chart(
                &resolver,
                &ap(&format!("data/songs/{song_id}/{song_id}-chart.json")),
                &ap(&format!("data/songs/{song_id}/{song_id}-metadata.json")),
                "normal",
            )
            .unwrap();
            assert_eq!(song.name, name);
            assert_eq!(song.chart.speed, speed);
            assert_eq!(song.chart.player1, player);
            assert_eq!(song.chart.player2, opponent);
            assert!(!song.chart.notes.is_empty());
        }
    }

    #[test]
    fn tracked_source_week1_visual_assets_parse() {
        let resolver = source_resolver();

        for (path, frame_count) in [
            ("images/BOYFRIEND.xml", 496),
            ("images/DADDY_DEAREST.xml", 45),
            ("images/GF_assets.xml", 252),
            ("images/NOTE_assets.xml", 48),
        ] {
            let atlas = load_sparrow(&resolver, &ap(path)).unwrap();
            assert_eq!(atlas.frames.len(), frame_count);
        }

        let notes = load_png(&resolver, &ap("images/NOTE_assets.png")).unwrap();
        assert_eq!((notes.width, notes.height), (2048, 1024));

        let stage_back = load_png(&resolver, &ap("images/stageback.png")).unwrap();
        assert_eq!((stage_back.width, stage_back.height), (2560, 1400));

        for (path, size) in [
            ("images/healthBar.png", (601, 19)),
            ("images/iconGrid.png", (1500, 900)),
            ("images/sick.png", (403, 152)),
            ("images/good.png", (317, 126)),
            ("images/bad.png", (261, 131)),
            ("images/shit.png", (285, 163)),
            ("images/combo.png", (343, 138)),
            ("images/num0.png", (94, 119)),
            ("images/num1.png", (98, 120)),
            ("images/num2.png", (105, 129)),
            ("images/num3.png", (102, 134)),
            ("images/num4.png", (98, 130)),
            ("images/num5.png", (111, 135)),
            ("images/num6.png", (108, 134)),
            ("images/num7.png", (91, 111)),
            ("images/num8.png", (90, 115)),
            ("images/num9.png", (91, 124)),
            ("images/ready.png", (757, 364)),
            ("images/set.png", (702, 322)),
            ("images/go.png", (558, 430)),
        ] {
            let image = load_png(&resolver, &ap(path)).unwrap();
            assert_eq!((image.width, image.height), size, "{path}");
        }

        for path in [
            "music/Bopeebo_Inst.ogg",
            "music/Bopeebo_Voices.ogg",
            "music/Fresh_Inst.ogg",
            "music/Fresh_Voices.ogg",
            "music/Dadbattle_Inst.ogg",
            "music/Dadbattle_Voices.ogg",
            "music/Tutorial_Inst.ogg",
        ] {
            let bytes = load_bytes(&resolver, &ap(path)).unwrap();
            assert_eq!(&bytes[..4], b"OggS", "{path}");
        }

        let font = load_bitmap_font(&resolver, &ap("fonts/vcr-bmp.fnt")).unwrap();
        assert_eq!(font.pages[0].file, "vcr-bmp.png");
        let font_image = load_png(&resolver, &ap("fonts/vcr-bmp.png")).unwrap();
        assert_eq!((font_image.width, font_image.height), (122, 122));
    }

    #[test]
    fn tracked_source_week1_animate_assets_parse() {
        let resolver = source_resolver();

        for (dir, symbol_name, frames, size) in [
            ("bf", "BF ALL ANIMS", 46, (1998, 1503)),
            ("dad", "DAD ALL ANIMS", 15, (991, 981)),
            ("gf", "GF ALL ANIMS", 45, (2064, 1065)),
            ("bf-death", "BF DEATH ALL ANIMS", 94, (2046, 1098)),
            ("bfFakeOut", "fake out death BF", 69, (2048, 1166)),
        ] {
            let animation = load_animate_animation(
                &resolver,
                &ap(&format!("images/characters/{dir}/Animation.json")),
            )
            .unwrap();
            assert_eq!(animation.symbol_name, symbol_name);
            assert!(
                !animation.labels.is_empty() || !animation.symbols.is_empty(),
                "{dir}"
            );

            let atlas = load_animate_spritemap(
                &resolver,
                &ap(&format!("images/characters/{dir}/spritemap1.json")),
            )
            .unwrap();
            assert_eq!(atlas.sprites[0].frames.len(), frames, "{dir}");

            let image = load_png(
                &resolver,
                &ap(&format!("images/characters/{dir}/spritemap1.png")),
            )
            .unwrap();
            assert_eq!((image.width, image.height), size, "{dir}");
        }
    }
}

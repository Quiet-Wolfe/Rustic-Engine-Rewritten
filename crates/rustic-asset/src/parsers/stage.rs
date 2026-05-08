//! Stage definition parser.
//!
//! Base FNF stages are mostly hard-coded in `PlayState.hx`. RusticV3 uses
//! baked, typed JSON definitions so renderer/app code can load the same shape
//! through the asset resolver that future overlays will use.
// LINT-ALLOW: long-file legacy and v-slice stage shape parsing plus tests

use crate::error::{AssetError, AssetResult};
use crate::parsers::types::AssetVec2;
use crate::path::AssetPath;
use rustic_core::render::RenderLayer;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct StageDefinition {
    pub id: String,
    #[serde(default = "default_camera_zoom")]
    pub camera_zoom: f32,
    #[serde(default)]
    pub boyfriend: StageCharacterSlot,
    #[serde(default)]
    pub girlfriend: StageCharacterSlot,
    #[serde(default)]
    pub opponent: StageCharacterSlot,
    #[serde(default)]
    pub objects: Vec<StageObject>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[non_exhaustive]
pub struct StageCharacterSlot {
    pub position: AssetVec2,
    pub camera_offset: AssetVec2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct StageObject {
    pub id: String,
    pub image: AssetPath,
    #[serde(default)]
    pub solid_color: Option<[u8; 4]>,
    #[serde(default)]
    pub animation: Option<StageObjectAnimation>,
    #[serde(default = "default_layer")]
    pub layer: RenderLayer,
    #[serde(default)]
    pub position: AssetVec2,
    #[serde(default = "default_vec2_one")]
    pub scroll_factor: AssetVec2,
    #[serde(default = "default_vec2_one")]
    pub scale: AssetVec2,
    #[serde(default)]
    pub z: i32,
    #[serde(default = "default_antialiasing")]
    pub antialiasing: bool,
    #[serde(default = "default_active")]
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[non_exhaustive]
pub struct StageObjectAnimation {
    pub name: String,
    pub prefix: String,
    pub frame_rate: u16,
    pub looped: bool,
}

impl Default for StageObjectAnimation {
    fn default() -> Self {
        Self {
            name: String::new(),
            prefix: String::new(),
            frame_rate: 24,
            looped: false,
        }
    }
}

fn default_camera_zoom() -> f32 {
    1.0
}

fn default_layer() -> RenderLayer {
    RenderLayer::Stage
}

fn default_antialiasing() -> bool {
    true
}

fn default_active() -> bool {
    true
}

fn default_vec2_one() -> AssetVec2 {
    AssetVec2::ONE
}

/// Resolve the base FNF stage id from the song name.
///
/// Legacy chart fallback. In v0.8.5 the chart difficulty carries the stage id
/// directly and `mainStage` is the default.
/// ref: bdedc0aa:source/funkin/util/Constants.hx:203-206
/// ref: bdedc0aa:source/funkin/ui/transition/LoadingState.hx:226-233
pub fn stage_id_for_song_name(song_name: &str) -> &'static str {
    match song_name.to_ascii_lowercase().as_str() {
        "spookeez" | "monster" | "south" => "spooky",
        "pico" | "blammed" | "philly" => "philly",
        "milf" | "satin-panties" | "high" => "limo",
        "cocoa" | "eggnog" => "mall",
        "winter-horrorland" => "mallEvil",
        "senpai" | "roses" => "school",
        "thorns" => "schoolEvil",
        _ => "stage",
    }
}

impl StageDefinition {
    pub fn parse(bytes: &[u8]) -> AssetResult<Self> {
        let parsed: RawStageDefinition = serde_json::from_slice(bytes)
            .map_err(|e| AssetError::InvalidData(format!("stage json: {e}")))?;
        let parsed = parsed.into_stage()?;
        parsed.validate()?;
        Ok(parsed)
    }

    fn validate(&self) -> AssetResult<()> {
        if self.id.trim().is_empty() {
            return Err(invalid("stage id is empty"));
        }
        for object in &self.objects {
            if object.id.trim().is_empty() {
                return Err(invalid(&format!(
                    "stage {} has object with empty id",
                    self.id
                )));
            }
            if object.scale.x <= 0.0 || object.scale.y <= 0.0 {
                return Err(invalid(&format!(
                    "stage {} object {} has non-positive scale",
                    self.id, object.id
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStageDefinition {
    id: Option<String>,
    name: Option<String>,
    #[serde(default = "default_camera_zoom")]
    camera_zoom: f32,
    #[serde(default)]
    boyfriend: StageCharacterSlot,
    #[serde(default)]
    girlfriend: StageCharacterSlot,
    #[serde(default)]
    opponent: StageCharacterSlot,
    characters: Option<RawStageCharacters>,
    #[serde(default)]
    objects: Vec<StageObject>,
    #[serde(default)]
    props: Vec<RawStageProp>,
}

#[derive(Debug, Deserialize)]
struct RawStageCharacters {
    bf: RawStageCharacterSlot,
    dad: RawStageCharacterSlot,
    gf: RawStageCharacterSlot,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RawStageCharacterSlot {
    #[serde(deserialize_with = "deserialize_asset_vec2")]
    position: AssetVec2,
    #[serde(rename = "cameraOffsets", deserialize_with = "deserialize_asset_vec2")]
    camera_offset: AssetVec2,
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RawStageProp {
    name: String,
    asset_path: String,
    starting_animation: Option<String>,
    #[serde(default)]
    animations: Vec<StageObjectAnimation>,
    #[serde(deserialize_with = "deserialize_asset_vec2")]
    position: AssetVec2,
    #[serde(deserialize_with = "deserialize_asset_vec2")]
    scale: AssetVec2,
    #[serde(deserialize_with = "deserialize_asset_vec2")]
    scroll: AssetVec2,
    z_index: i32,
    is_pixel: bool,
}

impl Default for RawStageProp {
    fn default() -> Self {
        Self {
            name: String::new(),
            asset_path: String::new(),
            starting_animation: None,
            animations: Vec::new(),
            position: AssetVec2::ZERO,
            scale: AssetVec2::ONE,
            scroll: AssetVec2::ONE,
            z_index: 0,
            is_pixel: false,
        }
    }
}

impl RawStageDefinition {
    fn into_stage(self) -> AssetResult<StageDefinition> {
        let mut boyfriend = self.boyfriend;
        let mut opponent = self.opponent;
        let mut girlfriend = self.girlfriend;
        if let Some(characters) = self.characters {
            boyfriend = characters.bf.into();
            opponent = characters.dad.into();
            girlfriend = characters.gf.into();
        }
        let mut objects = self.objects;
        for prop in self.props {
            if let Some(object) = prop.into_stage_object()? {
                objects.push(object);
            }
        }
        Ok(StageDefinition {
            id: self.id.or(self.name).unwrap_or_default(),
            camera_zoom: self.camera_zoom,
            boyfriend,
            girlfriend,
            opponent,
            objects,
        })
    }
}

impl From<RawStageCharacterSlot> for StageCharacterSlot {
    fn from(value: RawStageCharacterSlot) -> Self {
        Self {
            position: value.position,
            camera_offset: value.camera_offset,
        }
    }
}

impl RawStageProp {
    fn into_stage_object(self) -> AssetResult<Option<StageObject>> {
        if self.asset_path.trim().starts_with('#') {
            return self.into_solid_stage_object().map(Some);
        }
        let animation = prop_starting_animation(&self.starting_animation, &self.animations);
        Ok(Some(StageObject {
            id: if self.name.trim().is_empty() {
                self.asset_path.clone()
            } else {
                self.name
            },
            image: AssetPath::new(format!("images/{}.png", self.asset_path))?,
            solid_color: None,
            animation,
            layer: default_layer(),
            position: self.position,
            scroll_factor: self.scroll,
            scale: self.scale,
            z: self.z_index,
            antialiasing: !self.is_pixel,
            active: false,
        }))
    }

    fn into_solid_stage_object(self) -> AssetResult<StageObject> {
        let color = parse_hex_color(&self.asset_path)?;
        let id = if self.name.trim().is_empty() {
            format!("solid{}", self.asset_path.trim())
        } else {
            self.name
        };
        Ok(StageObject {
            id,
            image: AssetPath::new(format!(
                "generated/stage/solid-{}.png",
                self.asset_path.trim().trim_start_matches('#')
            ))?,
            solid_color: Some(color),
            animation: None,
            layer: default_layer(),
            position: self.position,
            scroll_factor: self.scroll,
            scale: self.scale,
            z: self.z_index,
            antialiasing: !self.is_pixel,
            active: false,
        })
    }
}

fn parse_hex_color(value: &str) -> AssetResult<[u8; 4]> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 && hex.len() != 8 {
        return Err(invalid(&format!("invalid solid color {value}")));
    }
    let parse = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&hex[range], 16)
            .map_err(|_| invalid(&format!("invalid solid color {value}")))
    };
    let r = parse(0..2)?;
    let g = parse(2..4)?;
    let b = parse(4..6)?;
    let a = if hex.len() == 8 { parse(6..8)? } else { 255 };
    Ok([r, g, b, a])
}

fn prop_starting_animation(
    starting_animation: &Option<String>,
    animations: &[StageObjectAnimation],
) -> Option<StageObjectAnimation> {
    if let Some(name) = starting_animation.as_deref() {
        if let Some(animation) = animations.iter().find(|animation| animation.name == name) {
            return Some(animation.clone());
        }
    }
    animations.first().cloned()
}

fn deserialize_asset_vec2<'de, D>(d: D) -> Result<AssetVec2, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Object { x: f32, y: f32 },
        Array([f32; 2]),
    }
    Ok(match Option::<Raw>::deserialize(d)? {
        Some(Raw::Object { x, y }) => AssetVec2::new(x, y),
        Some(Raw::Array([x, y])) => AssetVec2::new(x, y),
        None => AssetVec2::ZERO,
    })
}

fn invalid(msg: &str) -> AssetError {
    AssetError::InvalidData(format!("stage: {msg}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    const STAGE: &str = r#"{
        "id": "stage",
        "cameraZoom": 1.1,
        "boyfriend": {
            "position": { "x": 989.5, "y": 885.0 },
            "cameraOffset": { "x": -100.0, "y": -100.0 }
        },
        "girlfriend": {
            "position": { "x": 751.5, "y": 787.0 },
            "cameraOffset": { "x": 0.0, "y": 0.0 }
        },
        "opponent": {
            "position": { "x": 335.0, "y": 885.0 },
            "cameraOffset": { "x": 150.0, "y": -100.0 }
        },
        "objects": [
            {
                "id": "stageback",
                "image": "images/stageback.png",
                "position": { "x": -600.0, "y": -200.0 },
                "scrollFactor": { "x": 0.9, "y": 0.9 },
                "active": false
            },
            {
                "id": "stagefront",
                "image": "images/stagefront.png",
                "position": { "x": -650.0, "y": 600.0 },
                "scrollFactor": { "x": 0.9, "y": 0.9 },
                "scale": { "x": 1.1, "y": 1.1 },
                "active": false
            },
            {
                "id": "stagecurtains",
                "image": "images/stagecurtains.png",
                "position": { "x": -500.0, "y": -300.0 },
                "scrollFactor": { "x": 1.3, "y": 1.3 },
                "scale": { "x": 0.9, "y": 0.9 },
                "active": false
            }
        ]
    }"#;

    #[test]
    fn parses_stage_definition_with_defaults() {
        // ref: bdedc0aa:assets/preload/data/stages/mainStage.json
        let stage = StageDefinition::parse(STAGE.as_bytes()).unwrap();
        assert_eq!(stage.id, "stage");
        assert_eq!(stage.camera_zoom, 1.1);
        assert_eq!(stage.boyfriend.position, AssetVec2::new(989.5, 885.0));
        assert_eq!(
            stage.boyfriend.camera_offset,
            AssetVec2::new(-100.0, -100.0)
        );
        assert_eq!(stage.girlfriend.position, AssetVec2::new(751.5, 787.0));
        assert_eq!(stage.opponent.position, AssetVec2::new(335.0, 885.0));
        assert_eq!(stage.opponent.camera_offset, AssetVec2::new(150.0, -100.0));
        assert_eq!(stage.objects.len(), 3);

        let back = &stage.objects[0];
        assert_eq!(back.layer, RenderLayer::Stage);
        assert_eq!(back.position, AssetVec2::new(-600.0, -200.0));
        assert_eq!(back.scroll_factor, AssetVec2::new(0.9, 0.9));
        assert_eq!(back.scale, AssetVec2::ONE);
        assert!(back.antialiasing);
        assert!(!back.active);

        let front = &stage.objects[1];
        assert_eq!(front.layer, RenderLayer::Stage);
        assert_eq!(front.position, AssetVec2::new(-650.0, 600.0));
        assert_eq!(front.scale, AssetVec2::new(1.1, 1.1));
        assert_eq!(front.scroll_factor, AssetVec2::new(0.9, 0.9));

        let curtains = &stage.objects[2];
        assert_eq!(curtains.position, AssetVec2::new(-500.0, -300.0));
        assert_eq!(curtains.scale, AssetVec2::new(0.9, 0.9));
        assert_eq!(curtains.scroll_factor, AssetVec2::new(1.3, 1.3));
    }

    #[test]
    fn parses_vslice_stage_props_and_characters() {
        // ref: bdedc0aa:assets/preload/data/stages/mainStage.json
        let stage = StageDefinition::parse(
            br#"{
              "name": "Main Stage",
              "cameraZoom": 1.1,
              "props": [{
                "name": "stageBack",
                "assetPath": "stageback",
                "position": [-600, -200],
                "scroll": [0.9, 0.9],
                "zIndex": 10,
                "startingAnimation": "idle",
                "animations": [{
                  "name": "idle",
                  "prefix": "idle0",
                  "frameRate": 12,
                  "looped": true
                }]
              }],
              "characters": {
                "bf": { "position": [989.5, 885], "cameraOffsets": [-100, -100] },
                "dad": { "position": [335, 885], "cameraOffsets": [150, -100] },
                "gf": { "position": [751.5, 787], "cameraOffsets": [0, 0] }
              }
            }"#,
        )
        .unwrap();

        assert_eq!(stage.id, "Main Stage");
        assert_eq!(stage.boyfriend.position, AssetVec2::new(989.5, 885.0));
        assert_eq!(stage.opponent.camera_offset, AssetVec2::new(150.0, -100.0));
        assert_eq!(stage.objects[0].id, "stageBack");
        assert_eq!(stage.objects[0].image.as_str(), "images/stageback.png");
        assert_eq!(stage.objects[0].z, 10);
        assert_eq!(
            stage.objects[0].animation,
            Some(StageObjectAnimation {
                name: "idle".to_string(),
                prefix: "idle0".to_string(),
                frame_rate: 12,
                looped: true,
            })
        );
    }

    #[test]
    fn parses_vslice_solid_color_props() {
        let stage = StageDefinition::parse(
            br##"{
              "name": "Main Stage [Erect]",
              "props": [{
                "name": "solid",
                "assetPath": "#222026",
                "position": [-500, -1000],
                "scale": [2400, 2000],
                "scroll": [0, 0],
                "zIndex": 0
              }]
            }"##,
        )
        .unwrap();

        assert_eq!(stage.objects[0].id, "solid");
        assert_eq!(stage.objects[0].solid_color, Some([0x22, 0x20, 0x26, 0xff]));
        assert_eq!(
            stage.objects[0].image.as_str(),
            "generated/stage/solid-222026.png"
        );
        assert_eq!(stage.objects[0].scale, AssetVec2::new(2400.0, 2000.0));
    }

    #[test]
    fn resolves_vanilla_stage_from_song_name() {
        // Legacy chart fallback for imported pre-v-slice song ids.
        assert_eq!(stage_id_for_song_name("Spookeez"), "spooky");
        assert_eq!(stage_id_for_song_name("philly"), "philly");
        assert_eq!(stage_id_for_song_name("Satin-Panties"), "limo");
        assert_eq!(stage_id_for_song_name("eggnog"), "mall");
        assert_eq!(stage_id_for_song_name("winter-horrorland"), "mallEvil");
        assert_eq!(stage_id_for_song_name("roses"), "school");
        assert_eq!(stage_id_for_song_name("thorns"), "schoolEvil");
        assert_eq!(stage_id_for_song_name("bopeebo"), "stage");
    }

    #[test]
    fn rejects_empty_stage_id() {
        let json = r#"{"id":"","objects":[]}"#;
        assert!(StageDefinition::parse(json.as_bytes()).is_err());
    }

    #[test]
    fn rejects_invalid_object_path() {
        let json = r#"{
            "id":"stage",
            "objects":[{"id":"bad","image":"../stage.png"}]
        }"#;
        assert!(StageDefinition::parse(json.as_bytes()).is_err());
    }

    #[test]
    fn rejects_non_positive_scale() {
        let json = r#"{
            "id":"stage",
            "objects":[{"id":"bad","image":"images/stage.png","scale":{"x":0.0,"y":1.0}}]
        }"#;
        assert!(StageDefinition::parse(json.as_bytes()).is_err());
    }
}

//! Stage definition parser.
//!
//! Base FNF stages are mostly hard-coded in `PlayState.hx`. RusticV3 uses
//! baked, typed JSON definitions so renderer/app code can load the same shape
//! through the asset resolver that future overlays will use.

use crate::error::{AssetError, AssetResult};
use crate::parsers::types::AssetVec2;
use crate::path::AssetPath;
use rustic_core::render::RenderLayer;
use serde::{Deserialize, Serialize};

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
/// ref: 50fccded:source/PlayState.hx:173-487
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
        let parsed: Self = serde_json::from_slice(bytes)
            .map_err(|e| AssetError::InvalidData(format!("stage json: {e}")))?;
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

fn invalid(msg: &str) -> AssetError {
    AssetError::InvalidData(format!("stage: {msg}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    const STAGE: &str = r#"{
        "id": "stage",
        "cameraZoom": 0.9,
        "boyfriend": {
            "position": { "x": 770.0, "y": 450.0 }
        },
        "girlfriend": {
            "position": { "x": 400.0, "y": 130.0 }
        },
        "opponent": {
            "position": { "x": 100.0, "y": 100.0 },
            "cameraOffset": { "x": 400.0, "y": 0.0 }
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
        // ref: 50fccded:source/PlayState.hx:486-509 — default stage
        // camera zoom, image paths, positions, scroll factors, scale, and
        // inactive static sprites.
        // ref: 50fccded:source/PlayState.hx:529-574 — Week 1 character slots.
        let stage = StageDefinition::parse(STAGE.as_bytes()).unwrap();
        assert_eq!(stage.id, "stage");
        assert_eq!(stage.camera_zoom, 0.9);
        assert_eq!(stage.boyfriend.position, AssetVec2::new(770.0, 450.0));
        assert_eq!(stage.girlfriend.position, AssetVec2::new(400.0, 130.0));
        assert_eq!(stage.opponent.position, AssetVec2::new(100.0, 100.0));
        assert_eq!(stage.opponent.camera_offset, AssetVec2::new(400.0, 0.0));
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
    fn resolves_vanilla_stage_from_song_name() {
        // ref: 50fccded:source/PlayState.hx:173-487
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

//! Character definition parser.
//!
//! Base FNF stores vanilla character layout and animation definitions in
//! `Character.hx`. RusticV3 bakes that source-derived data into typed JSON
//! so gameplay and rendering never depend on hard-coded filesystem paths.

use crate::error::{AssetError, AssetResult};
use crate::parsers::types::AssetVec2;
use crate::path::AssetPath;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CharacterDefinition {
    #[serde(default, alias = "name")]
    pub id: String,
    #[serde(default)]
    pub render_type: CharacterRenderType,
    #[serde(default)]
    pub atlas: Option<AssetPath>,
    #[serde(default)]
    pub asset_path: Option<AssetPath>,
    #[serde(default)]
    pub icon: Option<AssetPath>,
    #[serde(default, alias = "startingAnimation")]
    pub initial_animation: Option<String>,
    #[serde(
        default,
        alias = "offsets",
        deserialize_with = "deserialize_asset_vec2"
    )]
    pub position: AssetVec2,
    #[serde(
        default,
        alias = "cameraOffsets",
        deserialize_with = "deserialize_asset_vec2"
    )]
    pub camera_offset: AssetVec2,
    #[serde(default = "default_sing_time")]
    pub sing_time: f64,
    #[serde(default = "default_dance_every")]
    pub dance_every: f64,
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default)]
    pub flip_x: bool,
    #[serde(default = "default_antialiasing")]
    pub antialiasing: bool,
    #[serde(default)]
    pub death: CharacterDeathDefinition,
    #[serde(default)]
    pub animations: Vec<CharacterAnimation>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[non_exhaustive]
pub struct CharacterDeathDefinition {
    #[serde(alias = "cameraOffsets", deserialize_with = "deserialize_asset_vec2")]
    pub camera_offset: AssetVec2,
    #[serde(default = "default_death_camera_zoom")]
    pub camera_zoom: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum CharacterRenderType {
    #[default]
    Sparrow,
    AnimateAtlas,
    MultiSparrow,
    MultiAnimateAtlas,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CharacterAnimation {
    pub name: String,
    pub prefix: String,
    #[serde(default)]
    pub asset_path: Option<AssetPath>,
    #[serde(default)]
    pub render_type: Option<CharacterRenderType>,
    #[serde(default)]
    pub anim_type: Option<String>,
    #[serde(default = "default_fps", alias = "frameRate")]
    pub fps: u16,
    #[serde(default)]
    pub looped: bool,
    #[serde(
        default,
        alias = "offsets",
        deserialize_with = "deserialize_asset_vec2"
    )]
    pub offset: AssetVec2,
    #[serde(default, alias = "frameIndices")]
    pub indices: Vec<u16>,
}

fn default_scale() -> f32 {
    1.0
}

fn default_antialiasing() -> bool {
    // ref: bdedc0aa:source/funkin/data/character/CharacterData.hx:456
    true
}

fn default_death_camera_zoom() -> f32 {
    // ref: bdedc0aa:source/funkin/play/character/BaseCharacter.hx:202
    1.0
}

fn default_sing_time() -> f64 {
    // ref: bdedc0aa:source/funkin/data/character/CharacterData.hx:452
    8.0
}

fn default_dance_every() -> f64 {
    // ref: bdedc0aa:source/funkin/data/character/CharacterData.hx:454
    1.0
}

fn default_fps() -> u16 {
    24
}

impl CharacterDefinition {
    pub fn parse(bytes: &[u8]) -> AssetResult<Self> {
        let parsed: Self = serde_json::from_slice(bytes)
            .map_err(|e| AssetError::InvalidData(format!("character json: {e}")))?;
        parsed.validate()?;
        Ok(parsed)
    }

    fn validate(&self) -> AssetResult<()> {
        if self.id.trim().is_empty() {
            return Err(invalid("character id is empty"));
        }
        if self.atlas.is_none() && self.asset_path.is_none() {
            return Err(invalid(&format!(
                "character {} has no atlas or assetPath",
                self.id
            )));
        }
        if self.animations.is_empty() {
            return Err(invalid(&format!("character {} has no animations", self.id)));
        }
        if self.sing_time <= 0.0 {
            return Err(invalid(&format!(
                "character {} has non-positive singTime",
                self.id
            )));
        }
        if self.dance_every < 0.0 {
            return Err(invalid(&format!(
                "character {} has negative danceEvery",
                self.id
            )));
        }
        if let Some(initial) = &self.initial_animation {
            if initial.trim().is_empty() {
                return Err(invalid(&format!(
                    "character {} has empty initial animation",
                    self.id
                )));
            }
            if !self.animations.iter().any(|a| a.name == *initial) {
                return Err(invalid(&format!(
                    "character {} initial animation {} is not defined",
                    self.id, initial
                )));
            }
        }
        for animation in &self.animations {
            if animation.name.trim().is_empty() {
                return Err(invalid(&format!(
                    "character {} has empty animation name",
                    self.id
                )));
            }
            if animation.prefix.trim().is_empty() {
                return Err(invalid(&format!(
                    "character {} animation {} has empty prefix",
                    self.id, animation.name
                )));
            }
            if animation.fps == 0 {
                return Err(invalid(&format!(
                    "character {} animation {} has zero fps",
                    self.id, animation.name
                )));
            }
        }
        Ok(())
    }
}

fn invalid(msg: &str) -> AssetError {
    AssetError::InvalidData(format!("character: {msg}"))
}

fn deserialize_asset_vec2<'de, D>(deserializer: D) -> Result<AssetVec2, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<RawVec2>::deserialize(deserializer).map(|raw| raw.map(Into::into).unwrap_or_default())
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawVec2 {
    Object(AssetVec2),
    Array([f32; 2]),
}

impl From<RawVec2> for AssetVec2 {
    fn from(raw: RawVec2) -> Self {
        match raw {
            RawVec2::Object(value) => value,
            RawVec2::Array([x, y]) => Self::new(x, y),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    const BF: &str = r#"{
        "id": "bf",
        "atlas": "images/BOYFRIEND.xml",
        "initialAnimation": "idle",
        "flipX": true,
        "animations": [
            {
                "name": "idle",
                "prefix": "BF idle dance",
                "fps": 24,
                "looped": false,
                "offset": { "x": -5.0, "y": 0.0 }
            },
            {
                "name": "singUP",
                "prefix": "BF NOTE UP0",
                "fps": 24,
                "looped": false,
                "offset": { "x": -29.0, "y": 27.0 }
            }
        ]
    }"#;

    #[test]
    fn parses_character_definition_with_defaults() {
        // Legacy Sparrow compatibility fixture. Current v0.8.5 character
        // data is covered by `parses_vslice_animate_character_shape`.
        let character = CharacterDefinition::parse(BF.as_bytes()).unwrap();
        assert_eq!(character.id, "bf");
        assert_eq!(
            character.atlas.as_ref().unwrap().as_str(),
            "images/BOYFRIEND.xml"
        );
        assert_eq!(character.render_type, CharacterRenderType::Sparrow);
        assert_eq!(character.asset_path, None);
        assert_eq!(character.initial_animation.as_deref(), Some("idle"));
        assert_eq!(character.position, AssetVec2::ZERO);
        assert_eq!(character.sing_time, 8.0);
        assert_eq!(character.dance_every, 1.0);
        assert_eq!(character.scale, 1.0);
        assert!(character.flip_x);
        assert!(character.antialiasing);
        assert_eq!(character.animations.len(), 2);
        assert_eq!(character.animations[0].fps, 24);
        assert_eq!(character.animations[0].offset, AssetVec2::new(-5.0, 0.0));
        assert_eq!(character.animations[1].prefix, "BF NOTE UP0");
        assert_eq!(character.animations[1].offset, AssetVec2::new(-29.0, 27.0));
    }

    #[test]
    fn parses_vslice_animate_character_shape() {
        let json = r#"{
            "version": "1.0.0",
            "name": "Daddy Dearest",
            "renderType": "animateatlas",
            "assetPath": "shared:characters/dad",
            "offsets": [13, 3],
            "cameraOffsets": [11, 6],
            "animations": [
                {
                    "name": "idle",
                    "prefix": "Idle"
                },
                {
                    "name": "idle-hold",
                    "prefix": "Idle",
                    "frameRate": 12,
                    "frameIndices": [11, 12, 0, 1],
                    "looped": true,
                    "offsets": [1, -2]
                }
            ]
        }"#;
        let character = CharacterDefinition::parse(json.as_bytes()).unwrap();

        assert_eq!(character.id, "Daddy Dearest");
        assert_eq!(character.render_type, CharacterRenderType::AnimateAtlas);
        assert_eq!(character.atlas, None);
        assert_eq!(
            character.asset_path.as_ref().unwrap().as_str(),
            "shared:characters/dad"
        );
        assert_eq!(character.position, AssetVec2::new(13.0, 3.0));
        assert_eq!(character.camera_offset, AssetVec2::new(11.0, 6.0));
        assert_eq!(character.animations[1].fps, 12);
        assert_eq!(character.animations[1].indices, vec![11, 12, 0, 1]);
        assert_eq!(character.animations[1].offset, AssetVec2::new(1.0, -2.0));
    }

    #[test]
    fn rejects_empty_animation_list() {
        let json = r#"{"id":"dad","atlas":"images/dad.xml","animations":[]}"#;
        assert!(CharacterDefinition::parse(json.as_bytes()).is_err());
    }

    #[test]
    fn rejects_invalid_asset_path() {
        let json = r#"{
            "id":"dad",
            "atlas":"../dad.xml",
            "animations":[{"name":"idle","prefix":"Dad idle dance"}]
        }"#;
        assert!(CharacterDefinition::parse(json.as_bytes()).is_err());
    }

    #[test]
    fn rejects_unknown_initial_animation() {
        let json = r#"{
            "id":"dad",
            "atlas":"images/dad.xml",
            "initialAnimation":"danceRight",
            "animations":[{"name":"idle","prefix":"Dad idle dance"}]
        }"#;
        assert!(CharacterDefinition::parse(json.as_bytes()).is_err());
    }

    #[test]
    fn accepts_vslice_starting_animation_alias_and_sing_time() {
        let json = r#"{
            "id":"dad",
            "atlas":"images/dad.xml",
            "startingAnimation":"idle",
            "singTime":6.5,
            "danceEvery":2,
            "animations":[{"name":"idle","prefix":"Dad idle dance"}]
        }"#;
        let character = CharacterDefinition::parse(json.as_bytes()).unwrap();
        assert_eq!(character.initial_animation.as_deref(), Some("idle"));
        assert_eq!(character.sing_time, 6.5);
        assert_eq!(character.dance_every, 2.0);
    }

    #[test]
    fn parses_death_camera_data() {
        let json = br#"{
            "id":"bf",
            "atlas":"images/bf.xml",
            "death": { "cameraOffsets": [-73, 42], "cameraZoom": 1.2 },
            "animations":[{"name":"idle","prefix":"BF idle dance"}]
        }"#;
        let character = CharacterDefinition::parse(json).unwrap();
        assert_eq!(character.death.camera_offset, AssetVec2::new(-73.0, 42.0));
        assert_eq!(character.death.camera_zoom, 1.2);
    }

    #[test]
    fn rejects_non_positive_sing_time() {
        let json = r#"{
            "id":"dad",
            "atlas":"images/dad.xml",
            "singTime":0,
            "animations":[{"name":"idle","prefix":"Dad idle dance"}]
        }"#;
        assert!(CharacterDefinition::parse(json.as_bytes()).is_err());
    }
}

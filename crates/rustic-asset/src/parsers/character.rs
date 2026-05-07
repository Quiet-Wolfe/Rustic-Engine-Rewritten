//! Character definition parser.
//!
//! Base FNF stores vanilla character layout and animation definitions in
//! `Character.hx`. RusticV3 bakes that source-derived data into typed JSON
//! so gameplay and rendering never depend on hard-coded filesystem paths.

use crate::error::{AssetError, AssetResult};
use crate::parsers::types::AssetVec2;
use crate::path::AssetPath;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CharacterDefinition {
    pub id: String,
    pub atlas: AssetPath,
    #[serde(default)]
    pub icon: Option<AssetPath>,
    #[serde(default, alias = "startingAnimation")]
    pub initial_animation: Option<String>,
    #[serde(default)]
    pub position: AssetVec2,
    #[serde(default)]
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
    pub animations: Vec<CharacterAnimation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CharacterAnimation {
    pub name: String,
    pub prefix: String,
    #[serde(default = "default_fps")]
    pub fps: u16,
    #[serde(default)]
    pub looped: bool,
    #[serde(default)]
    pub offset: AssetVec2,
    #[serde(default)]
    pub indices: Vec<u16>,
}

fn default_scale() -> f32 {
    1.0
}

fn default_antialiasing() -> bool {
    // ref: bdedc0aa:source/funkin/data/character/CharacterData.hx:456
    true
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
        // ref: 50fccded:source/Character.hx:268-305 — BF atlas,
        // animation prefixes, offsets, and flipX.
        let character = CharacterDefinition::parse(BF.as_bytes()).unwrap();
        assert_eq!(character.id, "bf");
        assert_eq!(character.atlas.as_str(), "images/BOYFRIEND.xml");
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

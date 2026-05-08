//! Story mode level data parser.
//!
//! ref: bdedc0aa:source/funkin/data/story/level/LevelData.hx

use crate::error::{AssetError, AssetResult};
use crate::parsers::character::CharacterAnimation;
use crate::parsers::types::AssetVec2;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct LevelDefinition {
    #[serde(default = "default_version")]
    pub version: String,
    pub name: String,
    pub title_asset: String,
    #[serde(default)]
    pub props: Vec<LevelPropDefinition>,
    #[serde(default = "default_visible")]
    pub visible: bool,
    #[serde(default = "default_songs")]
    pub songs: Vec<String>,
    #[serde(default = "default_background")]
    pub background: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[non_exhaustive]
pub struct LevelPropDefinition {
    pub asset_path: String,
    pub scale: f32,
    pub alpha: f32,
    pub is_pixel: bool,
    pub dance_every: f64,
    #[serde(alias = "offsets", deserialize_with = "deserialize_asset_vec2")]
    pub offset: AssetVec2,
    pub animations: Vec<CharacterAnimation>,
    pub starting_animation: String,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl Default for LevelPropDefinition {
    fn default() -> Self {
        Self {
            asset_path: String::new(),
            scale: 1.0,
            alpha: 1.0,
            is_pixel: false,
            dance_every: 1.0,
            offset: AssetVec2::ZERO,
            animations: Vec::new(),
            starting_animation: String::new(),
            flip_x: false,
            flip_y: false,
        }
    }
}

impl LevelDefinition {
    pub fn parse(bytes: &[u8]) -> AssetResult<Self> {
        let parsed: Self = serde_json::from_slice(bytes)
            .map_err(|e| AssetError::InvalidData(format!("level json: {e}")))?;
        parsed.validate()?;
        Ok(parsed)
    }

    fn validate(&self) -> AssetResult<()> {
        if self.name.trim().is_empty() {
            return Err(invalid("level name is empty"));
        }
        if self.title_asset.trim().is_empty() {
            return Err(invalid("titleAsset is empty"));
        }
        if self.songs.iter().any(|song| song.trim().is_empty()) {
            return Err(invalid("song id is empty"));
        }
        for prop in &self.props {
            if prop.asset_path.trim().is_empty() {
                return Err(invalid("prop assetPath is empty"));
            }
            if prop.scale <= 0.0 {
                return Err(invalid("prop scale must be positive"));
            }
        }
        Ok(())
    }
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
    AssetError::InvalidData(format!("level: {msg}"))
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_visible() -> bool {
    true
}

fn default_songs() -> Vec<String> {
    vec!["bopeebo".to_string()]
}

fn default_background() -> String {
    "#F9CF51".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_level_data_with_props_and_songs() {
        let level = LevelDefinition::parse(
            br##"{
              "version": "1.0.0",
              "name": "DADDY DEAREST",
              "titleAsset": "storymenu/titles/week1",
              "props": [{
                "assetPath": "storymenu/props/gf",
                "offsets": [200, 80],
                "animations": [{ "name": "danceLeft", "prefix": "idle0" }]
              }],
              "background": "#F9CF51",
              "songs": ["bopeebo", "fresh", "dadbattle"]
            }"##,
        )
        .unwrap();

        assert_eq!(level.name, "DADDY DEAREST");
        assert_eq!(level.title_asset, "storymenu/titles/week1");
        assert_eq!(level.props[0].offset, AssetVec2::new(200.0, 80.0));
        assert_eq!(level.props[0].animations[0].name, "danceLeft");
        assert_eq!(level.songs, vec!["bopeebo", "fresh", "dadbattle"]);
    }
}

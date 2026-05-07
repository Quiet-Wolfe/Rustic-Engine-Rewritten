//! Adobe Animate `Animation.json` timeline labels and symbol inventory.

use crate::error::{AnimateError, AnimateResult};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Animation {
    pub name: String,
    pub symbol_name: String,
    pub labels: Vec<AnimationLabel>,
    pub symbols: Vec<Symbol>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AnimationLabel {
    pub name: String,
    pub index: u32,
    pub duration: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Symbol {
    pub name: String,
}

impl Animation {
    pub fn parse(bytes: &[u8]) -> AnimateResult<Self> {
        let raw: RawAnimationFile = serde_json::from_slice(bytes)?;
        let name = raw.animation.name.unwrap_or_default();
        let symbol_name = raw.animation.symbol_name.unwrap_or_default();
        let labels = timeline_labels(raw.animation.timeline)?;
        let symbols = raw
            .animation
            .symbol_dictionary
            .map(|dictionary| dictionary.symbols)
            .unwrap_or_default()
            .into_iter()
            .map(|symbol| {
                if symbol.name.trim().is_empty() {
                    return Err(AnimateError::Atlas("symbol name is empty".into()));
                }
                Ok(Symbol { name: symbol.name })
            })
            .collect::<AnimateResult<Vec<_>>>()?;

        Ok(Self {
            name,
            symbol_name,
            labels,
            symbols,
        })
    }

    pub fn label(&self, name: &str) -> Option<&AnimationLabel> {
        self.labels.iter().find(|label| label.name == name)
    }

    pub fn has_symbol(&self, name: &str) -> bool {
        self.symbols.iter().any(|symbol| symbol.name == name)
    }
}

fn timeline_labels(timeline: RawTimeline) -> AnimateResult<Vec<AnimationLabel>> {
    let mut labels = Vec::new();
    for layer in timeline.layers {
        for frame in layer.frames {
            let Some(name) = frame.name else {
                continue;
            };
            if name.trim().is_empty() {
                return Err(AnimateError::Atlas("timeline label name is empty".into()));
            }
            if frame.duration == 0 {
                return Err(AnimateError::Atlas(format!(
                    "timeline label {name} has zero duration"
                )));
            }
            labels.push(AnimationLabel {
                name,
                index: frame.index,
                duration: frame.duration,
            });
        }
    }
    labels.sort_by_key(|label| label.index);
    Ok(labels)
}

#[derive(Debug, Deserialize)]
struct RawAnimationFile {
    #[serde(rename = "AN")]
    animation: RawAnimation,
}

#[derive(Debug, Deserialize)]
struct RawAnimation {
    #[serde(rename = "N")]
    name: Option<String>,
    #[serde(rename = "SN")]
    symbol_name: Option<String>,
    #[serde(rename = "TL")]
    timeline: RawTimeline,
    #[serde(rename = "SD")]
    symbol_dictionary: Option<RawSymbolDictionary>,
}

#[derive(Debug, Deserialize)]
struct RawTimeline {
    #[serde(rename = "L", default)]
    layers: Vec<RawLayer>,
}

#[derive(Debug, Deserialize)]
struct RawLayer {
    #[serde(rename = "FR", default)]
    frames: Vec<RawFrame>,
}

#[derive(Debug, Deserialize)]
struct RawFrame {
    #[serde(rename = "N")]
    name: Option<String>,
    #[serde(rename = "I")]
    index: u32,
    #[serde(rename = "DU")]
    duration: u32,
}

#[derive(Debug, Deserialize)]
struct RawSymbolDictionary {
    #[serde(rename = "S", default)]
    symbols: Vec<RawSymbol>,
}

#[derive(Debug, Deserialize)]
struct RawSymbol {
    #[serde(rename = "SN")]
    name: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const ANIMATION: &[u8] = br#"{
      "AN": {
        "N": "BoyFriend Assets_TA-Export",
        "SN": "BF ALL ANIMS",
        "TL": {
          "L": [
            {
              "LN": "Labels",
              "FR": [
                { "N": "Idle", "I": 0, "DU": 14, "E": [] },
                { "N": "Left", "I": 14, "DU": 8, "E": [] },
                { "I": 22, "DU": 2, "E": [] }
              ]
            }
          ]
        },
        "SD": {
          "S": [
            { "SN": "BF idle dance", "TL": { "L": [] } },
            { "SN": "BF NOTE LEFT", "TL": { "L": [] } }
          ]
        }
      }
    }"#;

    #[test]
    fn parses_animation_labels_and_symbols() {
        let animation = Animation::parse(ANIMATION).unwrap();

        assert_eq!(animation.name, "BoyFriend Assets_TA-Export");
        assert_eq!(animation.symbol_name, "BF ALL ANIMS");
        assert_eq!(
            animation.labels,
            vec![
                AnimationLabel {
                    name: "Idle".to_owned(),
                    index: 0,
                    duration: 14,
                },
                AnimationLabel {
                    name: "Left".to_owned(),
                    index: 14,
                    duration: 8,
                },
            ]
        );
        assert_eq!(animation.label("Left").unwrap().duration, 8);
        assert!(animation.has_symbol("BF NOTE LEFT"));
        assert!(!animation.has_symbol("Missing"));
    }

    #[test]
    fn rejects_zero_duration_labels() {
        let bad = br#"{
          "AN": {
            "TL": { "L": [{ "FR": [{ "N": "Idle", "I": 0, "DU": 0 }] }] }
          }
        }"#;
        assert!(matches!(Animation::parse(bad), Err(AnimateError::Atlas(_))));
    }

    #[test]
    fn rejects_empty_symbol_names() {
        let bad = br#"{
          "AN": {
            "TL": { "L": [] },
            "SD": { "S": [{ "SN": "" }] }
          }
        }"#;
        assert!(matches!(Animation::parse(bad), Err(AnimateError::Atlas(_))));
    }
}

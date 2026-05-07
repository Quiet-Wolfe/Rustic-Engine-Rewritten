//! Adobe Animate `Animation.json` timeline labels and symbol inventory.
// LINT-ALLOW: long-file animation timeline parser plus nested fixture coverage

use crate::error::{AnimateError, AnimateResult};
use glam::Vec2;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Symbol {
    pub name: String,
    pub layers: Vec<TimelineLayer>,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TimelineLayer {
    pub name: String,
    pub frames: Vec<TimelineFrame>,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TimelineFrame {
    pub index: u32,
    pub duration: u32,
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Element {
    pub matrix: [f32; 6],
    pub kind: ElementKind,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ElementKind {
    Symbol(SymbolInstance),
    Atlas(AtlasInstance),
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SymbolInstance {
    pub symbol_name: String,
    pub first_frame: u32,
    pub transform_point: Vec2,
    pub loop_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AtlasInstance {
    pub frame_name: String,
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
                Ok(Symbol {
                    name: symbol.name,
                    layers: timeline_layers(symbol.timeline.unwrap_or_default())?,
                })
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

    pub fn symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|symbol| symbol.name == name)
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

fn timeline_layers(timeline: RawTimeline) -> AnimateResult<Vec<TimelineLayer>> {
    timeline
        .layers
        .into_iter()
        .map(|layer| {
            let mut frames = layer
                .frames
                .into_iter()
                .map(|frame| {
                    if frame.duration == 0 {
                        return Err(AnimateError::Atlas(format!(
                            "timeline frame {} has zero duration",
                            frame.index
                        )));
                    }
                    Ok(TimelineFrame {
                        index: frame.index,
                        duration: frame.duration,
                        elements: frame_elements(frame.elements)?,
                    })
                })
                .collect::<AnimateResult<Vec<_>>>()?;
            frames.sort_by_key(|frame| frame.index);
            Ok(TimelineLayer {
                name: layer.name.unwrap_or_default(),
                frames,
            })
        })
        .collect()
}

fn frame_elements(elements: Vec<RawElement>) -> AnimateResult<Vec<Element>> {
    let mut parsed = Vec::new();
    for element in elements {
        match (element.symbol_instance, element.atlas_instance) {
            (Some(_), Some(_)) => {
                return Err(AnimateError::Atlas(
                    "timeline element has both symbol and atlas instances".into(),
                ));
            }
            (Some(instance), None) => parsed.push(symbol_element(instance)?),
            (None, Some(instance)) => parsed.push(atlas_element(instance)?),
            (None, None) => {}
        }
    }
    Ok(parsed)
}

fn symbol_element(instance: RawSymbolInstance) -> AnimateResult<Element> {
    if instance.symbol_name.trim().is_empty() {
        return Err(AnimateError::Atlas("element symbol name is empty".into()));
    }
    Ok(Element {
        matrix: instance_matrix(instance.matrix, instance.matrix3d),
        kind: ElementKind::Symbol(SymbolInstance {
            symbol_name: instance.symbol_name,
            first_frame: instance.first_frame,
            transform_point: Vec2::new(instance.transform_point.x, instance.transform_point.y),
            loop_mode: instance.loop_mode,
        }),
    })
}

fn atlas_element(instance: RawAtlasInstance) -> AnimateResult<Element> {
    if instance.frame_name.trim().is_empty() {
        return Err(AnimateError::Atlas(
            "element atlas frame name is empty".into(),
        ));
    }
    Ok(Element {
        matrix: instance_matrix(instance.matrix, instance.matrix3d),
        kind: ElementKind::Atlas(AtlasInstance {
            frame_name: instance.frame_name,
        }),
    })
}

fn instance_matrix(matrix: Option<[f32; 6]>, matrix3d: Option<[f32; 16]>) -> [f32; 6] {
    matrix
        .or_else(|| matrix3d.map(matrix3d_to_affine))
        .unwrap_or_else(identity_matrix)
}

fn matrix3d_to_affine(matrix: [f32; 16]) -> [f32; 6] {
    [
        matrix[0], matrix[1], matrix[4], matrix[5], matrix[12], matrix[13],
    ]
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

#[derive(Debug, Default, Deserialize)]
struct RawTimeline {
    #[serde(rename = "L", default)]
    layers: Vec<RawLayer>,
}

#[derive(Debug, Deserialize)]
struct RawLayer {
    #[serde(rename = "LN")]
    name: Option<String>,
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
    #[serde(rename = "E", default)]
    elements: Vec<RawElement>,
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
    #[serde(rename = "TL")]
    timeline: Option<RawTimeline>,
}

#[derive(Debug, Deserialize)]
struct RawElement {
    #[serde(rename = "SI")]
    symbol_instance: Option<RawSymbolInstance>,
    #[serde(rename = "ASI")]
    atlas_instance: Option<RawAtlasInstance>,
}

#[derive(Debug, Deserialize)]
struct RawSymbolInstance {
    #[serde(rename = "SN")]
    symbol_name: String,
    #[serde(rename = "FF", default)]
    first_frame: u32,
    #[serde(rename = "MX")]
    matrix: Option<[f32; 6]>,
    #[serde(rename = "M3D")]
    matrix3d: Option<[f32; 16]>,
    #[serde(rename = "TRP", default)]
    transform_point: RawPoint,
    #[serde(rename = "LP")]
    loop_mode: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawAtlasInstance {
    #[serde(rename = "N")]
    frame_name: String,
    #[serde(rename = "MX")]
    matrix: Option<[f32; 6]>,
    #[serde(rename = "M3D")]
    matrix3d: Option<[f32; 16]>,
}

#[derive(Debug, Default, Deserialize)]
struct RawPoint {
    #[serde(default)]
    x: f32,
    #[serde(default)]
    y: f32,
}

fn identity_matrix() -> [f32; 6] {
    [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
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
            {
              "SN": "BF idle dance",
              "TL": {
                "L": [
                  {
                    "LN": "Layer 1",
                    "FR": [
                      {
                        "I": 0,
                        "DU": 2,
                        "E": [
                          {
                            "SI": {
                              "SN": "BF Head default",
                              "FF": 4,
                              "TRP": { "x": 532.6, "y": -82 },
                              "LP": "LP",
                              "MX": [0.994, -0.105, 0.105, 0.994, 354.55, -165.35]
                            }
                          },
                          {
                            "ASI": {
                              "N": "0",
                              "MX": [1, 0, 0, 1, 401.15, -123]
                            }
                          }
                        ]
                      }
                    ]
                  }
                ]
              }
            },
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

        let symbol = animation.symbol("BF idle dance").unwrap();
        assert_eq!(symbol.layers.len(), 1);
        assert_eq!(symbol.layers[0].name, "Layer 1");
        assert_eq!(symbol.layers[0].frames.len(), 1);
        let element = &symbol.layers[0].frames[0].elements[0];
        assert_eq!(
            element.matrix,
            [0.994, -0.105, 0.105, 0.994, 354.55, -165.35]
        );
        let ElementKind::Symbol(instance) = &element.kind else {
            panic!("expected symbol instance");
        };
        assert_eq!(instance.symbol_name, "BF Head default");
        assert_eq!(instance.first_frame, 4);
        assert_eq!(instance.transform_point, Vec2::new(532.6, -82.0));
        assert_eq!(instance.loop_mode.as_deref(), Some("LP"));

        let element = &symbol.layers[0].frames[0].elements[1];
        assert_eq!(element.matrix, [1.0, 0.0, 0.0, 1.0, 401.15, -123.0]);
        let ElementKind::Atlas(instance) = &element.kind else {
            panic!("expected atlas instance");
        };
        assert_eq!(instance.frame_name, "0");
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

    #[test]
    fn rejects_empty_element_symbol_names() {
        let bad = br#"{
          "AN": {
            "TL": { "L": [] },
            "SD": {
              "S": [{
                "SN": "container",
                "TL": {
                  "L": [{
                    "FR": [{
                      "I": 0,
                      "DU": 1,
                      "E": [{ "SI": { "SN": "" } }]
                    }]
                  }]
                }
              }]
            }
          }
        }"#;
        assert!(matches!(Animation::parse(bad), Err(AnimateError::Atlas(_))));
    }

    #[test]
    fn parses_matrix3d_as_affine() {
        let json = br#"{
          "AN": {
            "TL": { "L": [] },
            "SD": {
              "S": [{
                "SN": "container",
                "TL": {
                  "L": [{
                    "FR": [{
                      "I": 0,
                      "DU": 1,
                      "E": [{
                        "ASI": {
                          "N": "0000",
                          "M3D": [2, 0.5, 0, 0, -0.25, 3, 0, 0, 0, 0, 1, 0, 7, 8, 0, 1]
                        }
                      }]
                    }]
                  }]
                }
              }]
            }
          }
        }"#;
        let animation = Animation::parse(json).unwrap();
        let element = &animation.symbol("container").unwrap().layers[0].frames[0].elements[0];
        assert_eq!(element.matrix, [2.0, 0.5, -0.25, 3.0, 7.0, 8.0]);
    }

    #[test]
    fn rejects_empty_element_atlas_frame_names() {
        let bad = br#"{
          "AN": {
            "TL": { "L": [] },
            "SD": {
              "S": [{
                "SN": "container",
                "TL": {
                  "L": [{
                    "FR": [{
                      "I": 0,
                      "DU": 1,
                      "E": [{ "ASI": { "N": "" } }]
                    }]
                  }]
                }
              }]
            }
          }
        }"#;
        assert!(matches!(Animation::parse(bad), Err(AnimateError::Atlas(_))));
    }
}

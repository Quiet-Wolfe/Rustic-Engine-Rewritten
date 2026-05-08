//! Parser for verbose Adobe Animate JSFL `Animation.json` exports.

use crate::animation::{
    normalize_symbol_first_frame, normalize_symbol_loop_mode, Animation, AnimationLabel,
    AtlasInstance, DrawPart, Element, ElementKind, Symbol, SymbolInstance, TimelineFrame,
    TimelineLayer,
};
use crate::error::{AnimateError, AnimateResult};
use glam::Vec2;
use serde::Deserialize;

pub(crate) fn parse(bytes: &[u8]) -> AnimateResult<Animation> {
    let raw: RawFile = serde_json::from_slice(bytes)?;
    let symbol_name = raw.animation.symbol_name.unwrap_or_default();
    let labels = timeline_labels(&raw.animation.timeline)?;
    let layers = timeline_layers(raw.animation.timeline)?;
    let stage = raw
        .animation
        .stage_instance
        .and_then(|stage| stage.symbol_instance)
        .map(symbol_element)
        .transpose()?;
    let mut symbols = raw
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
                layers: timeline_layers(symbol.timeline)?,
            })
        })
        .collect::<AnimateResult<Vec<_>>>()?;
    if !symbol_name.trim().is_empty() && !symbols.iter().any(|symbol| symbol.name == symbol_name) {
        symbols.push(Symbol {
            name: symbol_name.clone(),
            layers: layers.clone(),
        });
    }

    Ok(Animation {
        name: raw.animation.name.unwrap_or_default(),
        symbol_name,
        labels,
        layers,
        symbols,
        stage_matrix: stage.as_ref().map_or(ID_MATRIX, |stage| stage.matrix),
        stage_color: stage.as_ref().map_or(ID_COLOR, |stage| stage.color),
        stage_color_offset: stage
            .as_ref()
            .map_or(ZERO_COLOR_OFFSET, |stage| stage.color_offset),
    })
}

impl DrawPart {
    pub fn atlas_frame(frame_name: impl Into<String>, matrix: [f32; 6]) -> Self {
        Self {
            frame_name: frame_name.into(),
            matrix,
            color: ID_COLOR,
            color_offset: ZERO_COLOR_OFFSET,
        }
    }
}

fn timeline_labels(timeline: &RawTimeline) -> AnimateResult<Vec<AnimationLabel>> {
    let mut labels = Vec::new();
    for layer in &timeline.layers {
        for frame in &layer.frames {
            let Some(name) = &frame.name else {
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
                name: name.clone(),
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
    let position = instance.bitmap.and_then(|bitmap| bitmap.position);
    let (color, color_offset) = instance
        .color
        .map(color_transform)
        .unwrap_or((ID_COLOR, ZERO_COLOR_OFFSET));
    Ok(Element {
        matrix: matrix_with_position(instance.matrix, position),
        color,
        color_offset,
        kind: ElementKind::Symbol(SymbolInstance {
            symbol_name: instance.symbol_name,
            first_frame: normalize_symbol_first_frame(
                instance.symbol_type.as_deref(),
                instance.first_frame,
            ),
            transform_point: Vec2::new(instance.transform_point.x, instance.transform_point.y),
            loop_mode: normalize_symbol_loop_mode(
                instance.symbol_type.as_deref(),
                instance.loop_mode.as_deref(),
            ),
        }),
    })
}

fn atlas_element(instance: RawAtlasInstance) -> AnimateResult<Element> {
    if instance.name.trim().is_empty() {
        return Err(AnimateError::Atlas(
            "element atlas frame name is empty".into(),
        ));
    }
    let (color, color_offset) = instance
        .color
        .map(color_transform)
        .unwrap_or((ID_COLOR, ZERO_COLOR_OFFSET));
    Ok(Element {
        matrix: matrix_with_position(instance.matrix, instance.position),
        color,
        color_offset,
        kind: ElementKind::Atlas(AtlasInstance {
            frame_name: instance.name,
        }),
    })
}

fn color_transform(color: RawColorTransform) -> ([f32; 4], [f32; 4]) {
    (
        [
            color.red_multiplier,
            color.green_multiplier,
            color.blue_multiplier,
            color.alpha_multiplier,
        ],
        [
            color.red_offset / 255.0,
            color.green_offset / 255.0,
            color.blue_offset / 255.0,
            color.alpha_offset / 255.0,
        ],
    )
}

fn matrix_with_position(matrix: Option<RawMatrix3d>, position: Option<RawPoint>) -> [f32; 6] {
    let mut matrix = matrix.map(matrix3d_to_affine).unwrap_or(ID_MATRIX);
    // ref: references/flxanimate/flxanimate/animate/FlxElement.hx:160-174
    if let Some(position) = position {
        matrix[4] += position.x;
        matrix[5] += position.y;
    }
    matrix
}

fn matrix3d_to_affine(matrix: RawMatrix3d) -> [f32; 6] {
    [
        matrix.m00, matrix.m01, matrix.m10, matrix.m11, matrix.m30, matrix.m31,
    ]
}

#[derive(Debug, Deserialize)]
struct RawFile {
    #[serde(rename = "ANIMATION")]
    animation: RawAnimation,
    #[serde(rename = "SYMBOL_DICTIONARY")]
    symbol_dictionary: Option<RawSymbolDictionary>,
}

#[derive(Debug, Deserialize)]
struct RawAnimation {
    name: Option<String>,
    #[serde(rename = "SYMBOL_name")]
    symbol_name: Option<String>,
    #[serde(rename = "TIMELINE")]
    timeline: RawTimeline,
    #[serde(rename = "StageInstance")]
    stage_instance: Option<RawStageInstance>,
}

#[derive(Debug, Deserialize)]
struct RawSymbolDictionary {
    #[serde(rename = "Symbols", default)]
    symbols: Vec<RawSymbol>,
}

#[derive(Debug, Deserialize)]
struct RawSymbol {
    #[serde(rename = "SYMBOL_name")]
    name: String,
    #[serde(rename = "TIMELINE")]
    timeline: RawTimeline,
}

#[derive(Debug, Default, Deserialize)]
struct RawTimeline {
    #[serde(rename = "LAYERS", default)]
    layers: Vec<RawLayer>,
}

#[derive(Debug, Deserialize)]
struct RawLayer {
    #[serde(rename = "Layer_name")]
    name: Option<String>,
    #[serde(rename = "Frames", default)]
    frames: Vec<RawFrame>,
}

#[derive(Debug, Deserialize)]
struct RawFrame {
    name: Option<String>,
    index: u32,
    duration: u32,
    #[serde(default)]
    elements: Vec<RawElement>,
}

#[derive(Debug, Deserialize)]
struct RawElement {
    #[serde(rename = "SYMBOL_Instance")]
    symbol_instance: Option<RawSymbolInstance>,
    #[serde(rename = "ATLAS_SPRITE_instance")]
    atlas_instance: Option<RawAtlasInstance>,
}

#[derive(Debug, Deserialize)]
struct RawStageInstance {
    #[serde(rename = "SYMBOL_Instance")]
    symbol_instance: Option<RawSymbolInstance>,
}

#[derive(Debug, Deserialize)]
struct RawSymbolInstance {
    #[serde(rename = "SYMBOL_name")]
    symbol_name: String,
    #[serde(rename = "firstFrame", default)]
    first_frame: u32,
    #[serde(rename = "symbolType")]
    symbol_type: Option<String>,
    #[serde(rename = "loop")]
    loop_mode: Option<String>,
    #[serde(rename = "transformationPoint", default)]
    transform_point: RawPoint,
    #[serde(rename = "Matrix3D")]
    matrix: Option<RawMatrix3d>,
    #[serde(rename = "color", alias = "Color", alias = "C")]
    color: Option<RawColorTransform>,
    #[serde(alias = "BM")]
    bitmap: Option<RawBitmap>,
}

#[derive(Debug, Deserialize)]
struct RawAtlasInstance {
    name: String,
    #[serde(rename = "Position", alias = "POS")]
    position: Option<RawPoint>,
    #[serde(rename = "Matrix3D")]
    matrix: Option<RawMatrix3d>,
    #[serde(rename = "color", alias = "Color", alias = "C")]
    color: Option<RawColorTransform>,
}

#[derive(Debug, Deserialize)]
struct RawColorTransform {
    #[serde(
        rename = "RM",
        alias = "redMultiplier",
        alias = "RedMultiplier",
        default = "one"
    )]
    red_multiplier: f32,
    #[serde(
        rename = "GM",
        alias = "greenMultiplier",
        alias = "GreenMultiplier",
        default = "one"
    )]
    green_multiplier: f32,
    #[serde(
        rename = "BM",
        alias = "blueMultiplier",
        alias = "BlueMultiplier",
        default = "one"
    )]
    blue_multiplier: f32,
    #[serde(
        rename = "AM",
        alias = "alphaMultiplier",
        alias = "AlphaMultiplier",
        default = "one"
    )]
    alpha_multiplier: f32,
    #[serde(rename = "RO", alias = "redOffset", alias = "RedOffset", default)]
    red_offset: f32,
    #[serde(rename = "GO", alias = "greenOffset", alias = "GreenOffset", default)]
    green_offset: f32,
    #[serde(rename = "BO", alias = "blueOffset", alias = "BlueOffset", default)]
    blue_offset: f32,
    #[serde(rename = "AO", alias = "alphaOffset", alias = "AlphaOffset", default)]
    alpha_offset: f32,
}

#[derive(Debug, Deserialize)]
struct RawBitmap {
    #[serde(rename = "Position", alias = "POS")]
    position: Option<RawPoint>,
}

#[derive(Debug, Default, Deserialize)]
struct RawPoint {
    #[serde(default)]
    x: f32,
    #[serde(default)]
    y: f32,
}

#[derive(Debug, Deserialize)]
struct RawMatrix3d {
    m00: f32,
    m01: f32,
    m10: f32,
    m11: f32,
    m30: f32,
    m31: f32,
}

const ID_MATRIX: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
const ID_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const ZERO_COLOR_OFFSET: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

fn one() -> f32 {
    1.0
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::animation::{Animation, ElementKind};

    const VERBOSE_ANIMATION: &[u8] = br#"{
      "ANIMATION": {
        "name": "verbose",
        "SYMBOL_name": "root",
        "TIMELINE": {
          "LAYERS": [{
            "Layer_name": "root-art",
            "Frames": [{
              "index": 0,
              "duration": 1,
              "elements": [{
                    "ATLAS_SPRITE_instance": {
                      "name": "root0",
                      "color": { "RM": 0.5, "RO": 128 },
                      "Matrix3D": {
                    "m00": 1, "m01": 0, "m02": 0, "m03": 0,
                    "m10": 0, "m11": 1, "m12": 0, "m13": 0,
                    "m20": 0, "m21": 0, "m22": 1, "m23": 0,
                    "m30": 3, "m31": 4, "m32": 0, "m33": 1
                  }
                }
              }]
            }]
          }]
        }
      },
      "SYMBOL_DICTIONARY": {
        "Symbols": [{
          "SYMBOL_name": "container",
          "TIMELINE": {
            "LAYERS": [{
              "Layer_name": "front",
              "Frames": [{
                "index": 0,
                "duration": 2,
                "elements": [{
                  "SYMBOL_Instance": {
                    "SYMBOL_name": "hand",
                    "firstFrame": 1,
                    "loop": "singleframe",
                    "transformationPoint": { "x": 8, "y": 9 },
                    "Matrix3D": {
                      "m00": 2, "m01": 0.5, "m02": 0, "m03": 0,
                      "m10": -0.25, "m11": 3, "m12": 0, "m13": 0,
                      "m20": 0, "m21": 0, "m22": 1, "m23": 0,
                      "m30": 7, "m31": 8, "m32": 0, "m33": 1
                    }
                  }
                }]
              }]
            }]
          }
        }, {
          "SYMBOL_name": "hand",
          "TIMELINE": {
            "LAYERS": [{
              "Layer_name": "art",
              "Frames": [
                {
                  "index": 0, "duration": 1,
                  "elements": [{
                    "ATLAS_SPRITE_instance": {
                      "name": "hand0",
                      "Matrix3D": {
                        "m00": 1, "m01": 0, "m02": 0, "m03": 0,
                        "m10": 0, "m11": 1, "m12": 0, "m13": 0,
                        "m20": 0, "m21": 0, "m22": 1, "m23": 0,
                        "m30": 1, "m31": 2, "m32": 0, "m33": 1
                      }
                    }
                  }]
                },
                {
                  "index": 1, "duration": 1,
                  "elements": [{
                    "ATLAS_SPRITE_instance": {
                      "name": "hand1",
                      "Matrix3D": {
                        "m00": 1, "m01": 0, "m02": 0, "m03": 0,
                        "m10": 0, "m11": 1, "m12": 0, "m13": 0,
                        "m20": 0, "m21": 0, "m22": 1, "m23": 0,
                        "m30": 3, "m31": 4, "m32": 0, "m33": 1
                      }
                    }
                  }]
                }
              ]
            }]
          }
        }]
      },
      "metadata": { "framerate": 24 }
    }"#;

    #[test]
    fn parses_verbose_export_symbols() {
        let animation = Animation::parse(VERBOSE_ANIMATION).unwrap();
        assert_eq!(animation.name, "verbose");
        assert_eq!(animation.symbol_name, "root");
        assert!(animation.has_symbol("root"));
        assert!(animation.has_symbol("container"));

        let element = &animation.symbol("container").unwrap().layers[0].frames[0].elements[0];
        assert_eq!(element.matrix, [2.0, 0.5, -0.25, 3.0, 7.0, 8.0]);
        let ElementKind::Symbol(instance) = &element.kind else {
            panic!("expected symbol");
        };
        assert_eq!(instance.loop_mode.as_deref(), Some("SF"));
        assert_eq!(instance.transform_point, glam::Vec2::new(8.0, 9.0));

        let parts = animation.flatten_symbol_frame("container", 1).unwrap();
        assert_eq!(parts[0].frame_name, "hand1");
        assert_eq!(parts[0].matrix, [2.0, 0.5, -0.25, 3.0, 12.0, 21.5]);

        let parts = animation.flatten_symbol_frame("root", 0).unwrap();
        assert_eq!(parts[0].frame_name, "root0");
        assert_eq!(parts[0].matrix, [1.0, 0.0, 0.0, 1.0, 3.0, 4.0]);
        assert_eq!(parts[0].color, [0.5, 1.0, 1.0, 1.0]);
        assert!((parts[0].color_offset[0] - 128.0 / 255.0).abs() < 1e-6);
    }
}

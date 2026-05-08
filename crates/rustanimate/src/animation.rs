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
    pub layers: Vec<TimelineLayer>,
    pub symbols: Vec<Symbol>,
    pub stage_matrix: [f32; 6],
    pub stage_color: [f32; 4],
    pub stage_color_offset: [f32; 4],
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
    pub color: [f32; 4],
    pub color_offset: [f32; 4],
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

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct DrawPart {
    pub frame_name: String,
    pub matrix: [f32; 6],
    pub color: [f32; 4],
    pub color_offset: [f32; 4],
}

impl Animation {
    pub fn parse(bytes: &[u8]) -> AnimateResult<Self> {
        let raw: RawAnimationFile = match serde_json::from_slice(bytes) {
            Ok(raw) => raw,
            Err(err) => return crate::animation_alt::parse(bytes).map_err(|_| err.into()),
        };
        let RawAnimationFile {
            animation,
            symbol_dictionary: top_level_symbol_dictionary,
        } = raw;
        let RawAnimation {
            name,
            symbol_name,
            timeline,
            symbol_dictionary,
            stage_instance,
        } = animation;
        let name = name.unwrap_or_default();
        let symbol_name = symbol_name.unwrap_or_default();
        let labels = timeline_labels(&timeline)?;
        let layers = timeline_layers(timeline)?;
        let stage = stage_instance
            .and_then(|stage| stage.symbol_instance)
            .map(symbol_element)
            .transpose()?;
        let symbols = symbol_dictionary
            .or(top_level_symbol_dictionary)
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
            layers,
            symbols,
            stage_matrix: stage
                .as_ref()
                .map_or_else(identity_matrix, |stage| stage.matrix),
            stage_color: stage
                .as_ref()
                .map_or_else(identity_color, |stage| stage.color),
            stage_color_offset: stage
                .as_ref()
                .map_or_else(zero_color_offset, |stage| stage.color_offset),
        })
    }

    pub fn label(&self, name: &str) -> Option<&AnimationLabel> {
        self.labels.iter().find(|label| label.name == name)
    }

    pub fn has_symbol(&self, name: &str) -> bool {
        self.symbols.iter().any(|symbol| symbol.name == name)
    }

    pub fn symbol(&self, name: &str) -> Option<&Symbol> {
        // flxanimate loads the Animate symbol dictionary into a Map with
        // repeated set() calls, so later duplicate names replace earlier ones.
        self.symbols.iter().rev().find(|symbol| symbol.name == name)
    }

    pub fn flatten_label_frame(
        &self,
        label_name: &str,
        frame_offset: u32,
    ) -> AnimateResult<Vec<DrawPart>> {
        let label = self.label(label_name).ok_or_else(|| {
            AnimateError::Atlas(format!("animation label {label_name} was not found"))
        })?;
        let frame_offset = frame_offset.min(label.duration.saturating_sub(1));
        let mut parts = Vec::new();
        self.flatten_layers(
            &self.layers,
            label.index.saturating_add(frame_offset),
            self.stage_matrix,
            self.stage_color,
            self.stage_color_offset,
            &mut Vec::new(),
            &mut parts,
        )?;
        Ok(parts)
    }

    pub fn flatten_symbol_frame(
        &self,
        symbol_name: &str,
        frame_index: u32,
    ) -> AnimateResult<Vec<DrawPart>> {
        let symbol = self.symbol(symbol_name).ok_or_else(|| {
            AnimateError::Atlas(format!("animation symbol {symbol_name} was not found"))
        })?;
        let mut parts = Vec::new();
        self.flatten_symbol(
            symbol,
            frame_index,
            identity_matrix(),
            identity_color(),
            zero_color_offset(),
            &mut Vec::new(),
            &mut parts,
        )?;
        Ok(parts)
    }

    fn flatten_symbol(
        &self,
        symbol: &Symbol,
        frame_index: u32,
        parent_matrix: [f32; 6],
        parent_color: [f32; 4],
        parent_color_offset: [f32; 4],
        stack: &mut Vec<String>,
        parts: &mut Vec<DrawPart>,
    ) -> AnimateResult<()> {
        if stack.iter().any(|name| name == &symbol.name) {
            return Err(AnimateError::Atlas(format!(
                "animation symbol recursion includes {}",
                symbol.name
            )));
        }
        stack.push(symbol.name.clone());
        let frame_index = frame_index.min(symbol.duration().saturating_sub(1));
        let result = self.flatten_layers(
            &symbol.layers,
            frame_index,
            parent_matrix,
            parent_color,
            parent_color_offset,
            stack,
            parts,
        );
        stack.pop();
        result
    }

    fn flatten_layers(
        &self,
        layers: &[TimelineLayer],
        frame_index: u32,
        parent_matrix: [f32; 6],
        parent_color: [f32; 4],
        parent_color_offset: [f32; 4],
        stack: &mut Vec<String>,
        parts: &mut Vec<DrawPart>,
    ) -> AnimateResult<()> {
        for layer in layers.iter().rev() {
            let Some(frame) = active_frame(&layer.frames, frame_index) else {
                continue;
            };
            let frame_offset = frame_index.saturating_sub(frame.index);
            for element in &frame.elements {
                self.flatten_element(
                    element,
                    frame_offset,
                    parent_matrix,
                    parent_color,
                    parent_color_offset,
                    stack,
                    parts,
                )?;
            }
        }
        Ok(())
    }

    fn flatten_element(
        &self,
        element: &Element,
        frame_offset: u32,
        parent_matrix: [f32; 6],
        parent_color: [f32; 4],
        parent_color_offset: [f32; 4],
        stack: &mut Vec<String>,
        parts: &mut Vec<DrawPart>,
    ) -> AnimateResult<()> {
        let matrix = compose_affine(parent_matrix, element.matrix);
        let (color, color_offset) = concat_color(
            parent_color,
            parent_color_offset,
            element.color,
            element.color_offset,
        );
        match &element.kind {
            ElementKind::Atlas(instance) => {
                parts.push(DrawPart {
                    frame_name: instance.frame_name.clone(),
                    matrix,
                    color,
                    color_offset,
                });
                Ok(())
            }
            ElementKind::Symbol(instance) => {
                let symbol = self.symbol(&instance.symbol_name).ok_or_else(|| {
                    AnimateError::Atlas(format!(
                        "animation symbol {} was not found",
                        instance.symbol_name
                    ))
                })?;
                let frame_index = symbol_frame_index(instance, symbol.duration(), frame_offset);
                self.flatten_symbol(
                    symbol,
                    frame_index,
                    matrix,
                    color,
                    color_offset,
                    stack,
                    parts,
                )
            }
        }
    }
}

impl Symbol {
    pub fn duration(&self) -> u32 {
        timeline_duration(&self.layers)
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

fn timeline_duration(layers: &[TimelineLayer]) -> u32 {
    layers
        .iter()
        .flat_map(|layer| layer.frames.iter())
        .map(|frame| frame.index.saturating_add(frame.duration))
        .max()
        .unwrap_or(0)
}

fn active_frame(frames: &[TimelineFrame], frame_index: u32) -> Option<&TimelineFrame> {
    frames.iter().find(|frame| {
        frame_index >= frame.index && frame_index < frame.index.saturating_add(frame.duration)
    })
}

fn symbol_frame_index(instance: &SymbolInstance, symbol_duration: u32, frame_offset: u32) -> u32 {
    if symbol_duration == 0 {
        return 0;
    }
    let frame_index = instance.first_frame.saturating_add(frame_offset);
    match instance.loop_mode.as_deref() {
        Some("LP") => frame_index % symbol_duration,
        Some("SF") => instance.first_frame.min(symbol_duration - 1),
        _ => frame_index.min(symbol_duration - 1),
    }
}

pub(crate) fn normalize_symbol_first_frame(symbol_type: Option<&str>, first_frame: u32) -> u32 {
    if symbol_type.is_some_and(is_non_graphic_symbol_type) {
        0
    } else {
        first_frame
    }
}

pub(crate) fn normalize_symbol_loop_mode(
    symbol_type: Option<&str>,
    loop_mode: Option<&str>,
) -> Option<String> {
    if symbol_type.is_some_and(is_movie_clip_symbol_type) {
        return Some("LP".to_owned());
    }
    if symbol_type.is_some_and(is_button_symbol_type) {
        return Some("SF".to_owned());
    }
    match loop_mode.unwrap_or("LP").split('R').next().unwrap_or("LP") {
        "PO" | "playonce" => None,
        "SF" | "singleframe" => Some("SF".to_owned()),
        _ => Some("LP".to_owned()),
    }
}

fn is_non_graphic_symbol_type(symbol_type: &str) -> bool {
    is_movie_clip_symbol_type(symbol_type) || is_button_symbol_type(symbol_type)
}

fn is_movie_clip_symbol_type(symbol_type: &str) -> bool {
    symbol_type == "MC" || symbol_type.eq_ignore_ascii_case("movieclip")
}

fn is_button_symbol_type(symbol_type: &str) -> bool {
    symbol_type == "B" || symbol_type.eq_ignore_ascii_case("button")
}

fn compose_affine(parent: [f32; 6], child: [f32; 6]) -> [f32; 6] {
    [
        parent[0] * child[0] + parent[2] * child[1],
        parent[1] * child[0] + parent[3] * child[1],
        parent[0] * child[2] + parent[2] * child[3],
        parent[1] * child[2] + parent[3] * child[3],
        parent[0] * child[4] + parent[2] * child[5] + parent[4],
        parent[1] * child[4] + parent[3] * child[5] + parent[5],
    ]
}

fn concat_color(
    parent: [f32; 4],
    parent_offset: [f32; 4],
    child: [f32; 4],
    child_offset: [f32; 4],
) -> ([f32; 4], [f32; 4]) {
    let color = [
        parent[0] * child[0],
        parent[1] * child[1],
        parent[2] * child[2],
        parent[3] * child[3],
    ];
    let offset = [
        parent_offset[0] * child[0] + child_offset[0],
        parent_offset[1] * child[1] + child_offset[1],
        parent_offset[2] * child[2] + child_offset[2],
        parent_offset[3] * child[3] + child_offset[3],
    ];
    (color, offset)
}

fn color_transform(color: RawColorTransform) -> ([f32; 4], [f32; 4]) {
    let multiplier = [
        color.red_multiplier,
        color.green_multiplier,
        color.blue_multiplier,
        color.alpha_multiplier,
    ];
    let offset = [
        color.red_offset / 255.0,
        color.green_offset / 255.0,
        color.blue_offset / 255.0,
        color.alpha_offset / 255.0,
    ];
    (multiplier, offset)
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
        .unwrap_or_else(identity_color_transform);
    Ok(Element {
        matrix: instance_matrix_with_position(instance.matrix, instance.matrix3d, position),
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
    if instance.frame_name.trim().is_empty() {
        return Err(AnimateError::Atlas(
            "element atlas frame name is empty".into(),
        ));
    }
    let (color, color_offset) = instance
        .color
        .map(color_transform)
        .unwrap_or_else(identity_color_transform);
    Ok(Element {
        matrix: instance_matrix_with_position(
            instance.matrix,
            instance.matrix3d,
            instance.position,
        ),
        color,
        color_offset,
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

fn instance_matrix_with_position(
    matrix: Option<[f32; 6]>,
    matrix3d: Option<[f32; 16]>,
    position: Option<RawPoint>,
) -> [f32; 6] {
    let mut matrix = instance_matrix(matrix, matrix3d);
    if let Some(position) = position {
        matrix[4] += position.x;
        matrix[5] += position.y;
    }
    matrix
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
    #[serde(rename = "SD")]
    symbol_dictionary: Option<RawSymbolDictionary>,
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
    #[serde(rename = "STI")]
    stage_instance: Option<RawStageInstance>,
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
struct RawStageInstance {
    #[serde(rename = "SI")]
    symbol_instance: Option<RawSymbolInstance>,
}

#[derive(Debug, Deserialize)]
struct RawSymbolInstance {
    #[serde(rename = "SN")]
    symbol_name: String,
    #[serde(rename = "FF", default)]
    first_frame: u32,
    #[serde(rename = "ST")]
    symbol_type: Option<String>,
    #[serde(rename = "MX")]
    matrix: Option<[f32; 6]>,
    #[serde(rename = "M3D")]
    matrix3d: Option<[f32; 16]>,
    #[serde(rename = "C")]
    color: Option<RawColorTransform>,
    #[serde(rename = "TRP", default)]
    transform_point: RawPoint,
    #[serde(rename = "LP")]
    loop_mode: Option<String>,
    #[serde(rename = "BM", alias = "bitmap")]
    bitmap: Option<RawBitmap>,
}

#[derive(Debug, Deserialize)]
struct RawAtlasInstance {
    #[serde(rename = "N")]
    frame_name: String,
    #[serde(rename = "MX")]
    matrix: Option<[f32; 6]>,
    #[serde(rename = "M3D")]
    matrix3d: Option<[f32; 16]>,
    #[serde(rename = "C")]
    color: Option<RawColorTransform>,
    #[serde(rename = "POS", alias = "Position")]
    position: Option<RawPoint>,
}

#[derive(Debug, Deserialize)]
struct RawColorTransform {
    #[serde(rename = "RM", default = "one")]
    red_multiplier: f32,
    #[serde(rename = "GM", default = "one")]
    green_multiplier: f32,
    #[serde(rename = "BM", default = "one")]
    blue_multiplier: f32,
    #[serde(rename = "AM", default = "one")]
    alpha_multiplier: f32,
    #[serde(rename = "RO", default)]
    red_offset: f32,
    #[serde(rename = "GO", default)]
    green_offset: f32,
    #[serde(rename = "BO", default)]
    blue_offset: f32,
    #[serde(rename = "AO", default)]
    alpha_offset: f32,
}

#[derive(Debug, Deserialize)]
struct RawBitmap {
    #[serde(rename = "POS", alias = "Position")]
    position: Option<RawPoint>,
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

fn identity_color() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

fn zero_color_offset() -> [f32; 4] {
    [0.0, 0.0, 0.0, 0.0]
}

fn identity_color_transform() -> ([f32; 4], [f32; 4]) {
    (identity_color(), zero_color_offset())
}

fn one() -> f32 {
    1.0
}

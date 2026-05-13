//! glyphon-backed text rendering. See `PLAN.md` Section 7 (Text).
//!
//! Text positions are reference 1280x720 screen-space coordinates and use
//! the same camera/layer ordering as sprites for batching.

use crate::error::{RenderError, RenderResult};
use crate::state::{RenderState, REFERENCE_HEIGHT, REFERENCE_WIDTH};
use glam::{Vec2, Vec4};
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use rustic_core::ids::CameraId;
use rustic_core::render::RenderLayer;

/// Identifier for a registered font family. The renderer ships with one
/// default family resolved through `FontFamily::default()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FontFamily(pub u16);

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TextCommand {
    pub camera: CameraId,
    pub layer: RenderLayer,
    pub z: i32,
    pub text: String,
    pub position: Vec2,
    pub size_px: f32,
    pub line_height_px: f32,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
    pub color: Vec4,
    pub family: FontFamily,
}

impl TextCommand {
    pub fn new(text: impl Into<String>, position: Vec2, size_px: f32) -> Self {
        Self {
            camera: CameraId(1),
            layer: RenderLayer::Hud,
            z: 0,
            text: text.into(),
            position,
            size_px,
            line_height_px: size_px * 1.2,
            max_width: None,
            max_height: None,
            color: Vec4::ONE,
            family: FontFamily::default(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TextCommandList {
    commands: Vec<TextCommand>,
}

impl TextCommandList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, cmd: TextCommand) {
        self.commands.push(cmd);
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }

    pub fn as_slice(&self) -> &[TextCommand] {
        &self.commands
    }

    pub fn iter(&self) -> impl Iterator<Item = &TextCommand> {
        self.commands.iter()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// Holds glyphon GPU state. One instance per `RenderState`; cached for
/// the lifetime of the renderer.
pub struct TextSystem {
    font_system: FontSystem,
    swash_cache: SwashCache,
    // `Cache` owns the GPU pipelines/bind-group layouts shared by atlas
    // and viewport; the field exists to keep that GPU state alive.
    #[allow(dead_code)]
    cache: Cache,
    atlas: TextAtlas,
    viewport: Viewport,
    renderer: TextRenderer,
    buffers: Vec<Buffer>,
    default_family: DefaultTextFamily,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DefaultTextFamily {
    Named(String),
    SansSerif,
}

impl std::fmt::Debug for TextSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextSystem")
            .field("default_family", &self.default_family())
            .field("buffers", &self.buffers.len())
            .finish_non_exhaustive()
    }
}

impl TextSystem {
    pub fn new(rs: &RenderState, target_format: wgpu::TextureFormat) -> Self {
        let cache = Cache::new(&rs.device);
        let mut atlas = TextAtlas::new(&rs.device, &rs.queue, &cache, target_format);
        let mut viewport = Viewport::new(&rs.device, &cache);
        viewport.update(
            &rs.queue,
            Resolution {
                width: REFERENCE_WIDTH,
                height: REFERENCE_HEIGHT,
            },
        );
        let renderer = TextRenderer::new(
            &mut atlas,
            &rs.device,
            wgpu::MultisampleState::default(),
            None,
        );

        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            cache,
            atlas,
            viewport,
            renderer,
            buffers: Vec::new(),
            default_family: DefaultTextFamily::SansSerif,
        }
    }

    /// Register a font from raw TTF/OTF bytes. The bytes are copied into
    /// glyphon's font database; the family name is read from the file.
    pub fn add_font_bytes(&mut self, bytes: Vec<u8>) {
        self.font_system.db_mut().load_font_data(bytes);
    }

    /// Set the family name used when a command does not pick one explicitly.
    /// Useful after registering the VCR font so menus pick it up by default.
    pub fn set_default_family(&mut self, family: impl Into<String>) {
        self.default_family = DefaultTextFamily::Named(family.into());
    }

    pub fn default_family(&self) -> &str {
        match &self.default_family {
            DefaultTextFamily::Named(family) => family,
            DefaultTextFamily::SansSerif => "sans-serif",
        }
    }

    /// Resize the viewport (e.g. on surface resize). Text is rendered at
    /// the reference resolution by default; callers only need to update if
    /// they render text outside the reference target.
    pub fn set_viewport(&mut self, rs: &RenderState, width: u32, height: u32) {
        self.viewport
            .update(&rs.queue, Resolution { width, height });
    }

    /// Build cosmic-text buffers for each command and run glyphon's
    /// `prepare` pass. Must be called before `render`.
    pub fn prepare(&mut self, rs: &RenderState, commands: &[TextCommand]) -> RenderResult<()> {
        self.buffers.clear();
        self.buffers.reserve(commands.len());
        for cmd in commands {
            let metrics = Metrics::new(cmd.size_px, cmd.line_height_px.max(cmd.size_px));
            let mut buffer = Buffer::new(&mut self.font_system, metrics);
            let attrs = match &self.default_family {
                DefaultTextFamily::Named(family) => {
                    Attrs::new().family(Family::Name(family.as_str()))
                }
                DefaultTextFamily::SansSerif => Attrs::new().family(Family::SansSerif),
            };
            buffer.set_size(&mut self.font_system, cmd.max_width, cmd.max_height);
            buffer.set_text(&mut self.font_system, &cmd.text, attrs, Shaping::Advanced);
            buffer.shape_until_scroll(&mut self.font_system, false);
            self.buffers.push(buffer);
        }

        let areas: Vec<TextArea> = commands
            .iter()
            .zip(self.buffers.iter())
            .map(|(cmd, buffer)| TextArea {
                buffer,
                left: cmd.position.x,
                top: cmd.position.y,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: REFERENCE_WIDTH as i32,
                    bottom: REFERENCE_HEIGHT as i32,
                },
                default_color: color_from_vec4(cmd.color),
                custom_glyphs: &[],
            })
            .collect();

        self.renderer
            .prepare(
                &rs.device,
                &rs.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                areas,
                &mut self.swash_cache,
            )
            .map_err(|e| RenderError::Text(format!("glyphon prepare failed: {e:?}")))
    }

    /// Record glyphon draws into an existing render pass. Call inside the
    /// same pass that drew the sprite batch so the text overlays sprites
    /// on the active target.
    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) -> RenderResult<()> {
        self.renderer
            .render(&self.atlas, &self.viewport, pass)
            .map_err(|e| RenderError::Text(format!("glyphon render failed: {e:?}")))
    }

    /// End-of-frame cleanup. glyphon manages atlas growth; this releases
    /// per-frame staging.
    pub fn trim_atlas(&mut self) {
        self.atlas.trim();
    }
}

fn color_from_vec4(c: Vec4) -> Color {
    let r = (c.x.clamp(0.0, 1.0) * 255.0) as u8;
    let g = (c.y.clamp(0.0, 1.0) * 255.0) as u8;
    let b = (c.z.clamp(0.0, 1.0) * 255.0) as u8;
    let a = (c.w.clamp(0.0, 1.0) * 255.0) as u8;
    Color::rgba(r, g, b, a)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn text_command_defaults_into_camhud() {
        let cmd = TextCommand::new("hello", Vec2::new(10.0, 20.0), 24.0);
        assert_eq!(cmd.camera, CameraId(1));
        assert_eq!(cmd.layer, RenderLayer::Hud);
        assert_eq!(cmd.line_height_px, 24.0 * 1.2);
    }

    #[test]
    fn text_system_uses_generic_sans_serif_until_vcr_is_registered() {
        let family = DefaultTextFamily::SansSerif;

        assert_eq!(
            match family {
                DefaultTextFamily::Named(ref name) => name.as_str(),
                DefaultTextFamily::SansSerif => "sans-serif",
            },
            "sans-serif"
        );
    }

    #[test]
    fn color_quantizes_within_u8_range() {
        let c = color_from_vec4(Vec4::new(0.0, 0.5, 1.0, 1.0));
        assert_eq!(c.r(), 0);
        assert!(c.g() >= 127 && c.g() <= 128);
        assert_eq!(c.b(), 255);
        assert_eq!(c.a(), 255);
    }

    #[test]
    fn list_collects_commands_in_order() {
        let mut list = TextCommandList::new();
        list.push(TextCommand::new("a", Vec2::ZERO, 12.0));
        list.push(TextCommand::new("b", Vec2::ZERO, 12.0));
        assert_eq!(list.len(), 2);
        assert_eq!(list.as_slice()[0].text, "a");
        assert_eq!(list.as_slice()[1].text, "b");
    }
}

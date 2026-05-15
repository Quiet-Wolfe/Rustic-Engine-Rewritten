//! Dynamic wrapper for stage props that are graphically static but script-driven.

use crate::stage_scripted_motion::{
    limo_fast_car_position, philly_blazin_lightning_state, philly_train_x,
};
use rustic_asset::StageObject;
use rustic_core::ids::{AssetId, CameraId};
use rustic_render::{DrawCommand, FilterMode};

#[derive(Debug, Clone)]
pub(crate) struct StaticStagePropSprite {
    texture_id: AssetId,
    object: StageObject,
    size: glam::Vec2,
    filter: FilterMode,
}

impl StaticStagePropSprite {
    pub(crate) fn new(
        texture_id: AssetId,
        object: StageObject,
        size: glam::Vec2,
        filter: FilterMode,
    ) -> Self {
        Self {
            texture_id,
            object,
            size,
            filter,
        }
    }

    pub(crate) fn commands(
        &self,
        cursor: rustic_core::time::Samples,
        sample_rate: u32,
        bpm: f64,
    ) -> Vec<DrawCommand> {
        if self.hidden(cursor, sample_rate) {
            return Vec::new();
        }
        if let Some(commands) = self.scrolling_backdrop_commands(cursor, sample_rate) {
            return commands;
        }
        let mut cmd = self.command();
        self.apply_scripted_motion(&mut cmd, cursor, sample_rate, bpm);
        vec![cmd]
    }

    pub(crate) fn command(&self) -> DrawCommand {
        let mut cmd = DrawCommand::sprite(
            self.texture_id,
            glam::vec2(self.object.position.x, self.object.position.y),
            self.size,
        );
        cmd.camera = CameraId(0);
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = self.object.layer;
        cmd.z = self.object.z;
        cmd.filter = self.filter;
        cmd.scroll_factor = glam::vec2(self.object.scroll_factor.x, self.object.scroll_factor.y);
        cmd.color.w = self.object.alpha;
        cmd
    }

    fn hidden(&self, cursor: rustic_core::time::Samples, sample_rate: u32) -> bool {
        matches!(
            self.object.id.as_str(),
            "skyAdditive" | "foregroundMultiply" | "additionalLighten"
        ) && philly_blazin_lightning_state(cursor, sample_rate).is_none()
    }

    fn apply_scripted_motion(
        &self,
        cmd: &mut DrawCommand,
        cursor: rustic_core::time::Samples,
        sample_rate: u32,
        bpm: f64,
    ) {
        match self.object.id.as_str() {
            "train" if self.object.image.as_str() == "images/philly/train.png" => {
                if let Some(x) = philly_train_x(cursor, sample_rate, bpm) {
                    cmd.world_pos.x = x;
                }
            }
            "fastCar" => {
                if let Some(position) = limo_fast_car_position(cursor, sample_rate, bpm) {
                    cmd.world_pos = position;
                }
            }
            "skyAdditive" => {
                if let Some(state) = philly_blazin_lightning_state(cursor, sample_rate) {
                    cmd.color.w *= 0.7 * state.alpha;
                }
            }
            "foregroundMultiply" => {
                if let Some(state) = philly_blazin_lightning_state(cursor, sample_rate) {
                    cmd.color.w *= 0.64 * state.alpha;
                }
            }
            "additionalLighten" => {
                if let Some(state) = philly_blazin_lightning_state(cursor, sample_rate) {
                    cmd.color.w *= 0.3 * state.alpha.min(0.3);
                }
            }
            _ => {}
        }
    }

    fn scrolling_backdrop_commands(
        &self,
        cursor: rustic_core::time::Samples,
        sample_rate: u32,
    ) -> Option<Vec<DrawCommand>> {
        let spec = scrolling_backdrop_spec(self.object.id.as_str())?;
        let seconds = cursor.0.max(0) as f32 / sample_rate.max(1) as f32;
        let tile_width = self.size.x.max(1.0);
        let offset = (spec.velocity_x * self.object.scale.x * seconds).rem_euclid(tile_width);
        let mut commands = Vec::with_capacity(4);
        for tile in -1..=2 {
            let mut cmd = self.command();
            cmd.world_pos.x += offset + tile as f32 * tile_width;
            if let Some(wave) = spec.y_wave {
                cmd.world_pos.y = wave.base_y + (seconds * wave.frequency).sin() * wave.amplitude;
            }
            cmd.color *= spec.color;
            commands.push(cmd);
        }
        Some(commands)
    }
}

pub(crate) fn scripted_static_stage_object(object: &StageObject) -> bool {
    matches!(
        object.id.as_str(),
        "train"
            | "fastCar"
            | "skyAdditive"
            | "foregroundMultiply"
            | "additionalLighten"
            | "phillyScrollingSky"
            | "phillyErectScrollingSky"
            | "blazinScrollingSky"
            | "tankCloudsScrolling"
            | "phillyMist0"
            | "phillyMist1"
            | "phillyMist2"
            | "phillyMist3"
            | "phillyMist4"
            | "phillyMist5"
            | "limoMist1"
            | "limoMist2"
            | "limoMist3"
            | "limoMist4"
            | "limoMist5"
            | "sserafimDust1"
            | "sserafimDust2"
            | "sserafimDust3"
            | "sserafimDust4"
    )
}

#[derive(Debug, Clone, Copy)]
struct ScrollingBackdropSpec {
    velocity_x: f32,
    color: glam::Vec4,
    y_wave: Option<YWave>,
}

#[derive(Debug, Clone, Copy)]
struct YWave {
    base_y: f32,
    frequency: f32,
    amplitude: f32,
}

fn scrolling_backdrop_spec(id: &str) -> Option<ScrollingBackdropSpec> {
    let spec = match id {
        "phillyScrollingSky" | "phillyErectScrollingSky" => (-22.0, glam::Vec4::ONE, None),
        "blazinScrollingSky" => (-35.0, glam::Vec4::ONE, None),
        "tankCloudsScrolling" => (8.0, glam::Vec4::ONE, None),
        "phillyMist0" => (172.0, rgb(0x5c, 0x5c, 0x5c), Some(wave(660.0, 0.35, 70.0))),
        "phillyMist1" => (150.0, rgb(0x5c, 0x5c, 0x5c), Some(wave(500.0, 0.3, 80.0))),
        "phillyMist2" => (-80.0, rgb(0x5c, 0x5c, 0x5c), Some(wave(540.0, 0.4, 60.0))),
        "phillyMist3" => (-50.0, rgb(0x5c, 0x5c, 0x5c), Some(wave(230.0, 0.3, 70.0))),
        "phillyMist4" => (40.0, rgb(0x5c, 0x5c, 0x5c), Some(wave(170.0, 0.35, 50.0))),
        "phillyMist5" => (20.0, rgb(0x5c, 0x5c, 0x5c), Some(wave(-80.0, 0.08, 100.0))),
        "limoMist1" => (1700.0, rgb(0xc6, 0xbf, 0xde), Some(wave(100.0, 1.0, 200.0))),
        "limoMist2" => (2100.0, rgb(0x6a, 0x4d, 0xa1), Some(wave(0.0, 0.8, 100.0))),
        "limoMist3" => (900.0, rgb(0xa7, 0xd9, 0xbe), Some(wave(-20.0, 0.5, 200.0))),
        "limoMist4" => (700.0, rgb(0x9c, 0x77, 0xc7), Some(wave(-180.0, 0.4, 300.0))),
        "limoMist5" => (100.0, rgb(0xe7, 0xa4, 0x80), Some(wave(-450.0, 0.2, 150.0))),
        "sserafimDust1" => (350.0, rgb(0x98, 0x84, 0x7d), None),
        "sserafimDust2" => (-300.0, rgb(0x8b, 0x6c, 0x63), None),
        "sserafimDust3" => (-200.0, rgb(0x6e, 0x64, 0x5c), None),
        "sserafimDust4" => (-150.0, rgb(0x88, 0x6a, 0x60), None),
        _ => return None,
    };
    Some(ScrollingBackdropSpec {
        velocity_x: spec.0,
        color: spec.1,
        y_wave: spec.2,
    })
}

fn wave(base_y: f32, frequency: f32, amplitude: f32) -> YWave {
    YWave {
        base_y,
        frequency,
        amplitude,
    }
}

fn rgb(red: u8, green: u8, blue: u8) -> glam::Vec4 {
    glam::vec4(
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        1.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustic_asset::{AssetPath, AssetVec2};

    #[test]
    fn scrolling_backdrop_emits_wrapped_tiles_with_scripted_velocity() {
        let mut object = StageObject::png(
            "phillyScrollingSky",
            AssetPath::new("images/phillyStreets/phillySkybox.png").unwrap(),
        );
        object.position = AssetVec2::new(-650.0, -375.0);
        object.scroll_factor = AssetVec2::new(0.1, 0.1);
        object.scale = AssetVec2::new(0.65, 0.65);
        object.z = 10;
        let sprite = StaticStagePropSprite::new(
            AssetId::new(4),
            object,
            glam::vec2(1639.3, 466.7),
            FilterMode::Linear,
        );

        let start = sprite.commands(rustic_core::time::Samples(0), 48_000, 120.0);
        let later = sprite.commands(rustic_core::time::Samples(48_000), 48_000, 120.0);

        assert_eq!(start.len(), 4);
        assert_eq!(later.len(), 4);
        assert!(later[1].world_pos.x > start[1].world_pos.x);
    }

    #[test]
    fn mist_backdrops_apply_scripted_vertical_wave() {
        let spec = scrolling_backdrop_spec("limoMist1").unwrap();
        let wave = spec.y_wave.unwrap();

        assert_eq!(wave.base_y, 100.0);
        assert_eq!(wave.frequency, 1.0);
        assert_eq!(wave.amplitude, 200.0);
    }
}

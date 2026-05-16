//! Small scripted hooks for the Tankman Battlefield Erect stage.

use crate::preview_song::{PreviewDifficulty, PreviewSelection, PreviewSong, VARIATION_PICO};
use rustic_asset::ChartEventKind;
use rustic_core::ids::AssetId;
use rustic_core::render::RenderLayer;
use rustic_render::DrawCommand;

#[derive(Debug, Default, Clone)]
pub(crate) struct TankmanErectStageState {
    active: bool,
    alt_mask_enabled: bool,
}

impl TankmanErectStageState {
    pub(crate) fn reset_for_selection(&mut self, selection: PreviewSelection) {
        self.active = selection_uses_tankman_erect_stage(selection);
        self.alt_mask_enabled = false;
    }

    pub(crate) fn apply_event(&mut self, kind: &ChartEventKind) -> bool {
        if !matches!(kind, ChartEventKind::EnableMask) {
            return false;
        }
        if self.active {
            self.alt_mask_enabled = true;
        }
        true
    }

    pub(crate) fn apply_character_command(&self, cmd: &mut DrawCommand) {
        if !self.active || cmd.layer != RenderLayer::Characters {
            return;
        }
        // ref: bdedc0aa:assets/preload/scripts/stages/tankmanBattlefieldErect.hxc:33-72
        // The upstream stage uses DropShadowShader for rim light and toggles an
        // alternate mask for Bloody Tankman at the chart's EnableMask event. The
        // renderer does not have masked rim lighting yet, so keep the effect
        // scoped to color channels and expose the event state here.
        cmd.color.x *= 0.86;
        cmd.color.y *= 0.86;
        cmd.color.z *= 0.78;
        cmd.color_offset.x += 0.03;
        cmd.color_offset.y += 0.04;
        cmd.color_offset.z += 0.01;
        if self.alt_mask_enabled && cmd.texture == tankman_bloody_texture_id() {
            cmd.color_offset.x += 0.08;
            cmd.color_offset.y += 0.10;
            cmd.color_offset.z += 0.02;
        }
    }

    #[cfg(test)]
    pub(crate) fn alt_mask_enabled(&self) -> bool {
        self.alt_mask_enabled
    }
}

fn selection_uses_tankman_erect_stage(selection: PreviewSelection) -> bool {
    match selection.song {
        PreviewSong::UGH | PreviewSong::GUNS => {
            matches!(
                selection.difficulty,
                PreviewDifficulty::Erect | PreviewDifficulty::Nightmare
            ) || selection.variation == Some(VARIATION_PICO)
        }
        PreviewSong::STRESS => selection.variation == Some(VARIATION_PICO),
        _ => false,
    }
}

fn tankman_bloody_texture_id() -> AssetId {
    asset_id_for_literal("images/characters/tankman/bloody/spritemap1.png")
}

fn asset_id_for_literal(path: &str) -> AssetId {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in path.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    AssetId::new(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stress_pico_enable_mask_toggles_tankman_alt_mask() {
        let mut state = TankmanErectStageState::default();
        state.reset_for_selection(
            PreviewSelection::new(PreviewSong::STRESS, PreviewDifficulty::Hard)
                .with_variation(Some(VARIATION_PICO)),
        );

        assert!(!state.alt_mask_enabled());
        assert!(state.apply_event(&ChartEventKind::EnableMask));
        assert!(state.alt_mask_enabled());
    }

    #[test]
    fn enable_mask_is_acknowledged_but_inactive_off_tankman_erect() {
        let mut state = TankmanErectStageState::default();
        state.reset_for_selection(PreviewSelection::new(
            PreviewSong::STRESS,
            PreviewDifficulty::Hard,
        ));

        assert!(state.apply_event(&ChartEventKind::EnableMask));
        assert!(!state.alt_mask_enabled());
    }

    #[test]
    fn alt_mask_adds_extra_tint_to_bloody_tankman_texture() {
        let mut state = TankmanErectStageState::default();
        state.reset_for_selection(
            PreviewSelection::new(PreviewSong::STRESS, PreviewDifficulty::Hard)
                .with_variation(Some(VARIATION_PICO)),
        );
        let mut before = DrawCommand::sprite(
            tankman_bloody_texture_id(),
            glam::Vec2::ZERO,
            glam::Vec2::ONE,
        );
        before.layer = RenderLayer::Characters;
        let mut after = before.clone();

        state.apply_character_command(&mut before);
        state.apply_event(&ChartEventKind::EnableMask);
        state.apply_character_command(&mut after);

        assert!(after.color_offset.y > before.color_offset.y);
    }
}

//! Base-game note-kind animation overrides from v0.8.5 scripts.

use rustic_game::NoteKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NoteKindAction {
    Fallthrough,
    Skip,
    SingSuffix(&'static str),
    MissSuffix(&'static str),
    Pose(&'static str),
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct NoteKindAnimState {
    player_high_alt: bool,
    player_low_alt: bool,
    opponent_high_alt: bool,
    opponent_low_alt: bool,
}

impl NoteKindAnimState {
    pub(super) fn player_hit(&mut self, kind: NoteKind, current_pose: &str) -> NoteKindAction {
        match kind {
            NoteKind::NoAnim => NoteKindAction::Skip,
            NoteKind::Censor => NoteKindAction::SingSuffix("censor"),
            NoteKind::SakuraJoint => NoteKindAction::SingSuffix("joint"),
            NoteKind::SakuraBf1 => NoteKindAction::SingSuffix("bf1"),
            NoteKind::SakuraBf2 => NoteKindAction::SingSuffix("bf2"),
            NoteKind::Weekend1CockGun => NoteKindAction::Pose("cock"),
            NoteKind::Weekend1FireGun => NoteKindAction::Pose("shoot"),
            kind => self.player_blazin_hit(kind, current_pose),
        }
    }

    pub(super) fn opponent_hit(&mut self, kind: NoteKind, current_pose: &str) -> NoteKindAction {
        match kind {
            NoteKind::NoAnim => NoteKindAction::Skip,
            NoteKind::Mom => NoteKindAction::SingSuffix("alt"),
            NoteKind::Ugh => NoteKindAction::Pose("ugh"),
            NoteKind::HehPrettyGood => NoteKindAction::Pose("hehPrettyGood"),
            NoteKind::Weekend1LightCan => NoteKindAction::Pose("lightCan"),
            NoteKind::Weekend1KneeCan => NoteKindAction::Pose("kneeCan"),
            NoteKind::Weekend1KickCan => NoteKindAction::Pose("kickCan"),
            kind => self.opponent_blazin_hit(kind, current_pose),
        }
    }

    pub(super) fn player_miss(&mut self, kind: NoteKind, current_pose: &str) -> NoteKindAction {
        match kind {
            NoteKind::Weekend1CockGun => NoteKindAction::Skip,
            NoteKind::Weekend1FireGun => NoteKindAction::Pose("shootMISS"),
            NoteKind::SakuraJoint => NoteKindAction::MissSuffix("joint"),
            NoteKind::SakuraBf1 | NoteKind::SakuraBf2 => NoteKindAction::MissSuffix("bf2"),
            kind => self.player_blazin_miss(kind, current_pose),
        }
    }

    fn player_blazin_hit(&mut self, kind: NoteKind, current_pose: &str) -> NoteKindAction {
        match kind {
            NoteKind::Weekend1PunchLow
            | NoteKind::Weekend1PunchLowBlocked
            | NoteKind::Weekend1PunchLowDodged
            | NoteKind::Weekend1PunchLowSpin => NoteKindAction::Pose(self.player_punch_low()),
            NoteKind::Weekend1PunchHigh
            | NoteKind::Weekend1PunchHighBlocked
            | NoteKind::Weekend1PunchHighDodged
            | NoteKind::Weekend1PunchHighSpin => NoteKindAction::Pose(self.player_punch_high()),
            NoteKind::Weekend1BlockHigh
            | NoteKind::Weekend1BlockLow
            | NoteKind::Weekend1BlockSpin => NoteKindAction::Pose("block"),
            NoteKind::Weekend1DodgeHigh
            | NoteKind::Weekend1DodgeLow
            | NoteKind::Weekend1DodgeSpin => NoteKindAction::Pose("dodge"),
            NoteKind::Weekend1HitHigh => NoteKindAction::Pose("hitHigh"),
            NoteKind::Weekend1HitLow => NoteKindAction::Pose("hitLow"),
            NoteKind::Weekend1HitSpin => NoteKindAction::Pose("hitSpin"),
            NoteKind::Weekend1PicoUppercutPrep => NoteKindAction::Pose("uppercutPrep"),
            NoteKind::Weekend1PicoUppercut => NoteKindAction::Pose("uppercut"),
            NoteKind::Weekend1DarnellUppercutPrep => NoteKindAction::Pose("idle"),
            NoteKind::Weekend1DarnellUppercut => NoteKindAction::Pose("uppercutHit"),
            NoteKind::Weekend1Idle => NoteKindAction::Pose("idle"),
            NoteKind::Weekend1Fakeout => NoteKindAction::Pose("fakeout"),
            NoteKind::Weekend1Taunt => {
                if current_pose == "fakeout" {
                    NoteKindAction::Pose("taunt")
                } else {
                    NoteKindAction::Pose("idle")
                }
            }
            NoteKind::Weekend1TauntForce => NoteKindAction::Pose("taunt"),
            NoteKind::Weekend1ReverseFakeout => NoteKindAction::Pose("idle"),
            _ => NoteKindAction::Fallthrough,
        }
    }

    fn player_blazin_miss(&mut self, kind: NoteKind, current_pose: &str) -> NoteKindAction {
        match kind {
            NoteKind::Weekend1PunchLow
            | NoteKind::Weekend1PunchLowBlocked
            | NoteKind::Weekend1PunchLowDodged => NoteKindAction::Pose("hitLow"),
            NoteKind::Weekend1PunchLowSpin => NoteKindAction::Pose("hitSpin"),
            NoteKind::Weekend1PunchHigh
            | NoteKind::Weekend1PunchHighBlocked
            | NoteKind::Weekend1PunchHighDodged => NoteKindAction::Pose("hitHigh"),
            NoteKind::Weekend1PunchHighSpin => NoteKindAction::Pose("hitSpin"),
            NoteKind::Weekend1BlockHigh | NoteKind::Weekend1DodgeHigh => {
                NoteKindAction::Pose("hitHigh")
            }
            NoteKind::Weekend1BlockLow | NoteKind::Weekend1DodgeLow => {
                NoteKindAction::Pose("hitLow")
            }
            NoteKind::Weekend1BlockSpin | NoteKind::Weekend1DodgeSpin => {
                NoteKindAction::Pose("hitSpin")
            }
            NoteKind::Weekend1HitHigh => NoteKindAction::Pose("hitHigh"),
            NoteKind::Weekend1HitLow => NoteKindAction::Pose("hitLow"),
            NoteKind::Weekend1HitSpin => NoteKindAction::Pose("hitSpin"),
            NoteKind::Weekend1PicoUppercutPrep => NoteKindAction::Pose(self.player_punch_high()),
            NoteKind::Weekend1PicoUppercut => NoteKindAction::Pose("uppercut"),
            NoteKind::Weekend1DarnellUppercutPrep => NoteKindAction::Pose("idle"),
            NoteKind::Weekend1DarnellUppercut => NoteKindAction::Pose("uppercutHit"),
            NoteKind::Weekend1Idle => NoteKindAction::Pose("idle"),
            NoteKind::Weekend1Fakeout => NoteKindAction::Pose("hitHigh"),
            NoteKind::Weekend1Taunt => {
                if current_pose == "fakeout" {
                    NoteKindAction::Pose("taunt")
                } else {
                    NoteKindAction::Pose("idle")
                }
            }
            NoteKind::Weekend1TauntForce => NoteKindAction::Pose("taunt"),
            NoteKind::Weekend1ReverseFakeout => NoteKindAction::Pose("idle"),
            _ => NoteKindAction::Fallthrough,
        }
    }

    fn opponent_blazin_hit(&mut self, kind: NoteKind, current_pose: &str) -> NoteKindAction {
        match kind {
            NoteKind::Weekend1PunchLow => NoteKindAction::Pose("hitLow"),
            NoteKind::Weekend1PunchLowBlocked => NoteKindAction::Pose("block"),
            NoteKind::Weekend1PunchLowDodged => NoteKindAction::Pose("dodge"),
            NoteKind::Weekend1PunchLowSpin => NoteKindAction::Pose("hitSpin"),
            NoteKind::Weekend1PunchHigh => NoteKindAction::Pose("hitHigh"),
            NoteKind::Weekend1PunchHighBlocked => NoteKindAction::Pose("block"),
            NoteKind::Weekend1PunchHighDodged => NoteKindAction::Pose("dodge"),
            NoteKind::Weekend1PunchHighSpin => NoteKindAction::Pose("hitSpin"),
            NoteKind::Weekend1BlockLow | NoteKind::Weekend1DodgeLow | NoteKind::Weekend1HitLow => {
                NoteKindAction::Pose(self.opponent_punch_low())
            }
            NoteKind::Weekend1BlockHigh
            | NoteKind::Weekend1BlockSpin
            | NoteKind::Weekend1DodgeHigh
            | NoteKind::Weekend1DodgeSpin
            | NoteKind::Weekend1HitHigh
            | NoteKind::Weekend1HitSpin => NoteKindAction::Pose(self.opponent_punch_high()),
            NoteKind::Weekend1PicoUppercutPrep => NoteKindAction::Skip,
            NoteKind::Weekend1PicoUppercut => NoteKindAction::Pose("uppercutHit"),
            NoteKind::Weekend1DarnellUppercutPrep => NoteKindAction::Pose("uppercutPrep"),
            NoteKind::Weekend1DarnellUppercut => NoteKindAction::Pose("uppercut"),
            NoteKind::Weekend1Idle => NoteKindAction::Pose("idle"),
            NoteKind::Weekend1Fakeout => NoteKindAction::Pose("cringe"),
            NoteKind::Weekend1Taunt => {
                if current_pose == "cringe" {
                    NoteKindAction::Pose("pissed")
                } else {
                    NoteKindAction::Pose("idle")
                }
            }
            NoteKind::Weekend1TauntForce => NoteKindAction::Pose("pissed"),
            NoteKind::Weekend1ReverseFakeout => NoteKindAction::Pose("fakeout"),
            _ => NoteKindAction::Fallthrough,
        }
    }

    fn player_punch_high(&mut self) -> &'static str {
        self.player_high_alt = !self.player_high_alt;
        if self.player_high_alt {
            "punchHigh1"
        } else {
            "punchHigh2"
        }
    }

    fn player_punch_low(&mut self) -> &'static str {
        self.player_low_alt = !self.player_low_alt;
        if self.player_low_alt {
            "punchLow1"
        } else {
            "punchLow2"
        }
    }

    fn opponent_punch_high(&mut self) -> &'static str {
        self.opponent_high_alt = !self.opponent_high_alt;
        if self.opponent_high_alt {
            "punchHigh1"
        } else {
            "punchHigh2"
        }
    }

    fn opponent_punch_low(&mut self) -> &'static str {
        self.opponent_low_alt = !self.opponent_low_alt;
        if self.opponent_low_alt {
            "punchLow1"
        } else {
            "punchLow2"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn week5_mom_note_uses_alt_sing_suffix() {
        let mut state = NoteKindAnimState::default();

        assert_eq!(
            state.opponent_hit(NoteKind::Mom, "idle"),
            NoteKindAction::SingSuffix("alt")
        );
    }

    #[test]
    fn blazin_punches_alternate_per_character() {
        let mut state = NoteKindAnimState::default();

        assert_eq!(
            state.player_hit(NoteKind::Weekend1PunchHigh, "idle"),
            NoteKindAction::Pose("punchHigh1")
        );
        assert_eq!(
            state.player_hit(NoteKind::Weekend1PunchHigh, "idle"),
            NoteKindAction::Pose("punchHigh2")
        );
        assert_eq!(
            state.opponent_hit(NoteKind::Weekend1HitHigh, "idle"),
            NoteKindAction::Pose("punchHigh1")
        );
    }
}

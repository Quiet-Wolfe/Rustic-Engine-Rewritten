//! Stateful options-menu preferences used by the desktop Options page.

use crate::settings::PreferenceSettings;

const FPS_CHOICES: [u16; 6] = [60, 120, 144, 165, 240, 360];

pub(crate) const PREFERENCE_ITEM_COUNT: usize = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OptionsPreferences {
    pub(crate) downscroll: bool,
    pub(crate) strumline_background: u8,
    pub(crate) flashing_lights: bool,
    pub(crate) camera_zooms: bool,
    pub(crate) subtitles: bool,
    pub(crate) pause_on_unfocus: bool,
    pub(crate) launch_fullscreen: bool,
    pub(crate) vsync: bool,
    pub(crate) unlocked_framerate: bool,
    pub(crate) fps: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreferenceInput {
    Confirm,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreferenceChange {
    Changed,
    Back,
    None,
}

impl Default for OptionsPreferences {
    fn default() -> Self {
        Self {
            downscroll: false,
            strumline_background: 0,
            flashing_lights: true,
            camera_zooms: true,
            subtitles: true,
            pause_on_unfocus: true,
            launch_fullscreen: false,
            vsync: true,
            unlocked_framerate: false,
            fps: 60,
        }
    }
}

impl OptionsPreferences {
    pub(crate) fn from_settings(settings: &PreferenceSettings) -> Self {
        Self {
            downscroll: settings.downscroll,
            strumline_background: settings.strumline_background.min(100),
            flashing_lights: settings.flashing_lights,
            camera_zooms: settings.camera_zooms,
            subtitles: settings.subtitles,
            pause_on_unfocus: settings.pause_on_unfocus,
            launch_fullscreen: settings.launch_fullscreen,
            vsync: settings.vsync,
            unlocked_framerate: settings.unlocked_framerate,
            fps: if FPS_CHOICES.contains(&settings.fps) {
                settings.fps
            } else {
                60
            },
        }
    }

    pub(crate) fn write_to_settings(self, settings: &mut PreferenceSettings) {
        settings.downscroll = self.downscroll;
        settings.strumline_background = self.strumline_background;
        settings.flashing_lights = self.flashing_lights;
        settings.camera_zooms = self.camera_zooms;
        settings.subtitles = self.subtitles;
        settings.pause_on_unfocus = self.pause_on_unfocus;
        settings.launch_fullscreen = self.launch_fullscreen;
        settings.vsync = self.vsync;
        settings.unlocked_framerate = self.unlocked_framerate;
        settings.fps = self.fps;
    }

    pub(crate) fn label_for(self, index: usize) -> Option<String> {
        let label = match index {
            0 => row("DOWNSCROLL", on_off(self.downscroll)),
            1 => row(
                "STRUMLINE BACKGROUND",
                format!("{}%", self.strumline_background),
            ),
            2 => row("FLASHING LIGHTS", on_off(self.flashing_lights)),
            3 => row("CAMERA ZOOMS", on_off(self.camera_zooms)),
            4 => row("SUBTITLES", on_off(self.subtitles)),
            5 => row("PAUSE ON UNFOCUS", on_off(self.pause_on_unfocus)),
            6 => row("LAUNCH IN FULLSCREEN", on_off(self.launch_fullscreen)),
            7 => row("VSYNC", on_off(self.vsync)),
            8 => row("UNLOCKED FRAMERATE", on_off(self.unlocked_framerate)),
            9 => row("FPS", self.fps.to_string()),
            10 => "BACK".to_string(),
            _ => return None,
        };
        Some(label)
    }

    pub(crate) fn apply_input(&mut self, index: usize, input: PreferenceInput) -> PreferenceChange {
        match index {
            0 => toggle(&mut self.downscroll),
            1 => self.adjust_strumline_background(input),
            2 => toggle(&mut self.flashing_lights),
            3 => toggle(&mut self.camera_zooms),
            4 => toggle(&mut self.subtitles),
            5 => toggle(&mut self.pause_on_unfocus),
            6 => toggle(&mut self.launch_fullscreen),
            7 => toggle(&mut self.vsync),
            8 => toggle(&mut self.unlocked_framerate),
            9 => self.adjust_fps(input),
            10 => PreferenceChange::Back,
            _ => PreferenceChange::None,
        }
    }

    fn adjust_strumline_background(&mut self, input: PreferenceInput) -> PreferenceChange {
        let delta = match input {
            PreferenceInput::Left => -10,
            PreferenceInput::Right | PreferenceInput::Confirm => 10,
        };
        let next = (i16::from(self.strumline_background) + delta).clamp(0, 100);
        if next as u8 == self.strumline_background {
            return PreferenceChange::None;
        }
        self.strumline_background = next as u8;
        PreferenceChange::Changed
    }

    fn adjust_fps(&mut self, input: PreferenceInput) -> PreferenceChange {
        let Some(index) = FPS_CHOICES.iter().position(|fps| *fps == self.fps) else {
            self.fps = 60;
            return PreferenceChange::Changed;
        };
        let next = match input {
            PreferenceInput::Left => index.saturating_sub(1),
            PreferenceInput::Right | PreferenceInput::Confirm => {
                (index + 1).min(FPS_CHOICES.len() - 1)
            }
        };
        if next == index {
            return PreferenceChange::None;
        }
        self.fps = FPS_CHOICES[next];
        PreferenceChange::Changed
    }
}

fn toggle(value: &mut bool) -> PreferenceChange {
    *value = !*value;
    PreferenceChange::Changed
}

fn row(label: &str, value: impl AsRef<str>) -> String {
    format!("{label:<22} {}", value.as_ref())
}

fn on_off(value: bool) -> &'static str {
    if value {
        "ON"
    } else {
        "OFF"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_labels_match_desktop_preference_rows() {
        let prefs = OptionsPreferences::default();

        assert_eq!(
            prefs.label_for(0).as_deref(),
            Some("DOWNSCROLL             OFF")
        );
        assert_eq!(
            prefs.label_for(1).as_deref(),
            Some("STRUMLINE BACKGROUND   0%")
        );
        assert_eq!(
            prefs.label_for(7).as_deref(),
            Some("VSYNC                  ON")
        );
    }

    #[test]
    fn preference_inputs_mutate_values_and_back_exits() {
        let mut prefs = OptionsPreferences::default();

        assert_eq!(
            prefs.apply_input(0, PreferenceInput::Confirm),
            PreferenceChange::Changed
        );
        assert!(prefs.downscroll);
        assert_eq!(
            prefs.apply_input(1, PreferenceInput::Right),
            PreferenceChange::Changed
        );
        assert_eq!(prefs.strumline_background, 10);
        assert_eq!(
            prefs.apply_input(9, PreferenceInput::Confirm),
            PreferenceChange::Changed
        );
        assert_eq!(prefs.fps, 120);
        assert_eq!(
            prefs.apply_input(10, PreferenceInput::Confirm),
            PreferenceChange::Back
        );
    }

    #[test]
    fn preferences_round_trip_through_settings() {
        let mut settings = PreferenceSettings {
            camera_zooms: false,
            fps: 144,
            strumline_background: 110,
            ..PreferenceSettings::default()
        };
        let mut prefs = OptionsPreferences::from_settings(&settings);
        assert!(!prefs.camera_zooms);
        assert_eq!(prefs.fps, 144);
        assert_eq!(prefs.strumline_background, 100);

        prefs.camera_zooms = true;
        prefs.fps = 240;
        prefs.write_to_settings(&mut settings);
        assert!(settings.camera_zooms);
        assert_eq!(settings.fps, 240);
    }
}

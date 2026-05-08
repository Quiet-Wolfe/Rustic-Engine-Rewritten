//! Development preview song selection for the current gameplay slice.

use std::env;

const PREVIEW_SONG_ENV: &str = "RUSTIC_PREVIEW_SONG";
const PREVIEW_DIFFICULTY_ENV: &str = "RUSTIC_PREVIEW_DIFFICULTY";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewSong {
    pub id: u32,
    pub folder: &'static str,
    pub audio_prefix: &'static str,
}

impl PreviewSong {
    pub const ALL: [Self; 4] = [Self::TUTORIAL, Self::BOPEEBO, Self::FRESH, Self::DADBATTLE];

    pub fn from_env() -> Self {
        env::var(PREVIEW_SONG_ENV)
            .ok()
            .and_then(|value| Self::from_key(&value))
            .unwrap_or(Self::BOPEEBO)
    }

    pub fn from_key(value: &str) -> Option<Self> {
        match normalized_key(value).as_str() {
            "tutorial" => Some(Self::TUTORIAL),
            "bopeebo" => Some(Self::BOPEEBO),
            "fresh" => Some(Self::FRESH),
            "dadbattle" | "dad-battle" => Some(Self::DADBATTLE),
            _ => None,
        }
    }

    pub fn chart_path(self) -> String {
        format!("data/songs/{}/{}-chart.json", self.folder, self.folder)
    }

    pub fn metadata_path(self) -> String {
        format!("data/songs/{}/{}-metadata.json", self.folder, self.folder)
    }

    pub fn inst_path(self) -> String {
        format!("music/{}_Inst.ogg", self.audio_prefix)
    }

    pub fn voices_path(self) -> String {
        format!("music/{}_Voices.ogg", self.audio_prefix)
    }

    pub fn next(self) -> Self {
        next_in(&Self::ALL, self)
    }

    pub(crate) const TUTORIAL: Self = Self {
        id: 0,
        folder: "tutorial",
        audio_prefix: "Tutorial",
    };
    pub(crate) const BOPEEBO: Self = Self {
        id: 1,
        folder: "bopeebo",
        audio_prefix: "Bopeebo",
    };
    pub(crate) const FRESH: Self = Self {
        id: 2,
        folder: "fresh",
        audio_prefix: "Fresh",
    };
    pub(crate) const DADBATTLE: Self = Self {
        id: 3,
        folder: "dadbattle",
        audio_prefix: "Dadbattle",
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewDifficulty {
    Easy,
    Normal,
    Hard,
}

impl PreviewDifficulty {
    pub const ALL: [Self; 3] = [Self::Easy, Self::Normal, Self::Hard];

    pub fn from_env() -> Self {
        env::var(PREVIEW_DIFFICULTY_ENV)
            .ok()
            .and_then(|value| Self::from_key(&value))
            .unwrap_or(Self::Normal)
    }

    pub fn from_key(value: &str) -> Option<Self> {
        match normalized_key(value).as_str() {
            "easy" => Some(Self::Easy),
            "normal" | "medium" => Some(Self::Normal),
            "hard" => Some(Self::Hard),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
        }
    }

    pub fn next(self) -> Self {
        next_in(&Self::ALL, self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewSelection {
    pub song: PreviewSong,
    pub difficulty: PreviewDifficulty,
}

impl PreviewSelection {
    pub fn from_env() -> Self {
        Self {
            song: PreviewSong::from_env(),
            difficulty: PreviewDifficulty::from_env(),
        }
    }

    pub fn from_keys(song: Option<&str>, difficulty: Option<&str>) -> Self {
        Self {
            song: song
                .and_then(PreviewSong::from_key)
                .unwrap_or(PreviewSong::BOPEEBO),
            difficulty: difficulty
                .and_then(PreviewDifficulty::from_key)
                .unwrap_or(PreviewDifficulty::Normal),
        }
    }

    pub fn next_song(self) -> Self {
        Self {
            song: self.song.next(),
            ..self
        }
    }

    pub fn next_difficulty(self) -> Self {
        Self {
            difficulty: self.difficulty.next(),
            ..self
        }
    }
}

fn normalized_key(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace([' ', '_'], "-")
}

fn next_in<T: Copy + PartialEq, const N: usize>(values: &[T; N], current: T) -> T {
    let index = values
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0);
    values[(index + 1) % N]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_song_key_accepts_week_one_names() {
        assert_eq!(
            PreviewSong::from_key("tutorial"),
            Some(PreviewSong::TUTORIAL)
        );
        assert_eq!(PreviewSong::from_key("Bopeebo"), Some(PreviewSong::BOPEEBO));
        assert_eq!(PreviewSong::from_key("fresh"), Some(PreviewSong::FRESH));
        assert_eq!(
            PreviewSong::from_key("Dad Battle"),
            Some(PreviewSong::DADBATTLE)
        );
    }

    #[test]
    fn preview_song_paths_match_imported_assets() {
        let song = PreviewSong::DADBATTLE;
        assert_eq!(
            song.chart_path(),
            "data/songs/dadbattle/dadbattle-chart.json"
        );
        assert_eq!(song.inst_path(), "music/Dadbattle_Inst.ogg");
    }

    #[test]
    fn preview_difficulty_key_accepts_base_difficulties() {
        assert_eq!(
            PreviewDifficulty::from_key("easy"),
            Some(PreviewDifficulty::Easy)
        );
        assert_eq!(
            PreviewDifficulty::from_key("Medium"),
            Some(PreviewDifficulty::Normal)
        );
        assert_eq!(
            PreviewDifficulty::from_key("HARD"),
            Some(PreviewDifficulty::Hard)
        );
    }

    #[test]
    fn preview_selection_defaults_to_bopeebo_normal() {
        assert_eq!(
            PreviewSelection::from_keys(None, None),
            PreviewSelection {
                song: PreviewSong::BOPEEBO,
                difficulty: PreviewDifficulty::Normal,
            }
        );
    }

    #[test]
    fn preview_selection_accepts_song_and_difficulty_keys() {
        assert_eq!(
            PreviewSelection::from_keys(Some("fresh"), Some("hard")),
            PreviewSelection {
                song: PreviewSong::FRESH,
                difficulty: PreviewDifficulty::Hard,
            }
        );
    }

    #[test]
    fn preview_selection_cycles_songs_and_difficulties() {
        let selection = PreviewSelection::from_keys(Some("dadbattle"), Some("hard"));
        assert_eq!(selection.next_song().song, PreviewSong::TUTORIAL);
        assert_eq!(
            selection.next_difficulty().difficulty,
            PreviewDifficulty::Easy
        );
    }
}

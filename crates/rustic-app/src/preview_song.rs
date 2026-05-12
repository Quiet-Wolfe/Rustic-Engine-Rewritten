//! Development preview song selection for the current gameplay slice.
// LINT-ALLOW: long-file preview song data and focused selection tests stay together.

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
    pub const CYCLABLE_WEEK1: [Self; 4] =
        [Self::TUTORIAL, Self::BOPEEBO, Self::FRESH, Self::DADBATTLE];
    const BASE_DIFFICULTIES: [PreviewDifficulty; 3] = [
        PreviewDifficulty::Easy,
        PreviewDifficulty::Normal,
        PreviewDifficulty::Hard,
    ];
    const WEEK1_VARIANT_DIFFICULTIES: [PreviewDifficulty; 5] = [
        PreviewDifficulty::Easy,
        PreviewDifficulty::Normal,
        PreviewDifficulty::Hard,
        PreviewDifficulty::Erect,
        PreviewDifficulty::Nightmare,
    ];

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
        self.chart_path_for(PreviewDifficulty::Normal)
    }

    pub fn chart_path_for(self, difficulty: PreviewDifficulty) -> String {
        if let Some(suffix) = difficulty.chart_variation_suffix() {
            return format!(
                "data/songs/{}/{}-chart-{suffix}.json",
                self.folder, self.folder
            );
        }
        format!("data/songs/{}/{}-chart.json", self.folder, self.folder)
    }

    pub fn metadata_path(self) -> String {
        self.metadata_path_for(PreviewDifficulty::Normal)
    }

    pub fn metadata_path_for(self, difficulty: PreviewDifficulty) -> String {
        if let Some(suffix) = difficulty.chart_variation_suffix() {
            return format!(
                "data/songs/{}/{}-metadata-{suffix}.json",
                self.folder, self.folder
            );
        }
        format!("data/songs/{}/{}-metadata.json", self.folder, self.folder)
    }

    pub fn inst_path(self) -> String {
        format!("music/{}_Inst.ogg", self.audio_prefix)
    }

    pub fn voices_path(self) -> String {
        format!("music/{}_Voices.ogg", self.audio_prefix)
    }

    pub fn display_name(self) -> &'static str {
        match self.id {
            0 => "Tutorial",
            1 => "Bopeebo",
            2 => "Fresh",
            3 => "Dad Battle",
            _ => self.folder,
        }
    }

    pub fn next(self) -> Self {
        next_in(&Self::CYCLABLE_WEEK1, self)
    }

    pub fn previous(self) -> Self {
        previous_in(&Self::CYCLABLE_WEEK1, self)
    }

    pub fn available_difficulties(self) -> &'static [PreviewDifficulty] {
        match self.id {
            Self::BOPEEBO_ID..=Self::DADBATTLE_ID => &Self::WEEK1_VARIANT_DIFFICULTIES,
            _ => &Self::BASE_DIFFICULTIES,
        }
    }

    const BOPEEBO_ID: u32 = 1;
    const DADBATTLE_ID: u32 = 3;

    pub const TUTORIAL: Self = Self {
        id: 0,
        folder: "tutorial",
        audio_prefix: "Tutorial",
    };
    pub const BOPEEBO: Self = Self {
        id: 1,
        folder: "bopeebo",
        audio_prefix: "Bopeebo",
    };
    pub const FRESH: Self = Self {
        id: 2,
        folder: "fresh",
        audio_prefix: "Fresh",
    };
    pub const DADBATTLE: Self = Self {
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
    Erect,
    Nightmare,
}

impl PreviewDifficulty {
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
            "erect" => Some(Self::Erect),
            "nightmare" => Some(Self::Nightmare),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
            Self::Erect => "erect",
            Self::Nightmare => "nightmare",
        }
    }

    pub fn chart_variation_suffix(self) -> Option<&'static str> {
        match self {
            Self::Erect | Self::Nightmare => Some("erect"),
            Self::Easy | Self::Normal | Self::Hard => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewSelection {
    pub song: PreviewSong,
    pub difficulty: PreviewDifficulty,
}

impl PreviewSelection {
    pub fn from_env() -> Self {
        Self::new(PreviewSong::from_env(), PreviewDifficulty::from_env())
    }

    pub fn from_keys(song: Option<&str>, difficulty: Option<&str>) -> Self {
        let song = song
            .and_then(PreviewSong::from_key)
            .unwrap_or(PreviewSong::BOPEEBO);
        let difficulty = difficulty
            .and_then(PreviewDifficulty::from_key)
            .unwrap_or(PreviewDifficulty::Normal);
        Self::new(song, difficulty)
    }

    pub fn new(song: PreviewSong, difficulty: PreviewDifficulty) -> Self {
        Self {
            song,
            difficulty: difficulty_for_song(song, difficulty),
        }
    }

    pub fn next_song(self) -> Self {
        let song = self.song.next();
        Self {
            song,
            difficulty: difficulty_for_song(song, self.difficulty),
        }
    }

    pub fn previous_song(self) -> Self {
        let song = self.song.previous();
        Self {
            song,
            difficulty: difficulty_for_song(song, self.difficulty),
        }
    }

    pub fn next_difficulty(self) -> Self {
        Self {
            difficulty: next_in_slice(self.song.available_difficulties(), self.difficulty),
            ..self
        }
    }

    pub fn previous_difficulty(self) -> Self {
        Self {
            difficulty: previous_in_slice(self.song.available_difficulties(), self.difficulty),
            ..self
        }
    }
}

fn normalized_key(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace([' ', '_'], "-")
}

fn difficulty_for_song(song: PreviewSong, difficulty: PreviewDifficulty) -> PreviewDifficulty {
    if song.available_difficulties().contains(&difficulty) {
        difficulty
    } else {
        PreviewDifficulty::Normal
    }
}

fn next_in<T: Copy + PartialEq, const N: usize>(values: &[T; N], current: T) -> T {
    next_in_slice(values, current)
}

fn previous_in<T: Copy + PartialEq, const N: usize>(values: &[T; N], current: T) -> T {
    previous_in_slice(values, current)
}

fn next_in_slice<T: Copy + PartialEq>(values: &[T], current: T) -> T {
    assert!(!values.is_empty(), "preview option list must not be empty");
    match values.iter().position(|value| *value == current) {
        Some(index) => values[(index + 1) % values.len()],
        None => values[0],
    }
}

fn previous_in_slice<T: Copy + PartialEq>(values: &[T], current: T) -> T {
    assert!(!values.is_empty(), "preview option list must not be empty");
    match values.iter().position(|value| *value == current) {
        Some(0) => values[values.len() - 1],
        Some(index) => values[index - 1],
        None => values[0],
    }
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
        assert_eq!(song.display_name(), "Dad Battle");
        assert_eq!(
            song.chart_path(),
            "data/songs/dadbattle/dadbattle-chart.json"
        );
        assert_eq!(
            song.chart_path_for(PreviewDifficulty::Nightmare),
            "data/songs/dadbattle/dadbattle-chart-erect.json"
        );
        assert_eq!(
            song.metadata_path_for(PreviewDifficulty::Erect),
            "data/songs/dadbattle/dadbattle-metadata-erect.json"
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
        assert_eq!(
            PreviewDifficulty::from_key("nightmare"),
            Some(PreviewDifficulty::Nightmare)
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
        assert_eq!(
            PreviewSelection::from_keys(Some("tutorial"), Some("nightmare")),
            PreviewSelection {
                song: PreviewSong::TUTORIAL,
                difficulty: PreviewDifficulty::Normal,
            }
        );
    }

    #[test]
    fn preview_selection_cycles_songs_and_difficulties() {
        let selection = PreviewSelection::from_keys(Some("dadbattle"), Some("hard"));
        assert_eq!(selection.next_song().song, PreviewSong::TUTORIAL);
        assert_eq!(
            selection.next_difficulty().difficulty,
            PreviewDifficulty::Erect
        );
        assert_eq!(
            PreviewSelection::from_keys(Some("tutorial"), None)
                .next_song()
                .song,
            PreviewSong::BOPEEBO
        );
        assert_eq!(
            PreviewSelection::from_keys(Some("tutorial"), Some("hard"))
                .next_difficulty()
                .difficulty,
            PreviewDifficulty::Easy
        );
        assert_eq!(
            PreviewSelection::from_keys(Some("dadbattle"), Some("nightmare"))
                .next_song()
                .difficulty,
            PreviewDifficulty::Normal
        );
        assert_eq!(
            PreviewSelection::from_keys(Some("tutorial"), Some("easy"))
                .previous_song()
                .song,
            PreviewSong::DADBATTLE
        );
        assert_eq!(
            PreviewSelection::from_keys(Some("bopeebo"), Some("easy"))
                .previous_difficulty()
                .difficulty,
            PreviewDifficulty::Nightmare
        );
    }
}

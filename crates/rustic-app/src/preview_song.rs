//! Development preview song selection for the current gameplay slice.

use std::env;

const PREVIEW_SONG_ENV: &str = "RUSTIC_PREVIEW_SONG";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewSong {
    pub id: u32,
    pub folder: &'static str,
    pub audio_prefix: &'static str,
}

impl PreviewSong {
    pub fn from_env() -> Self {
        env::var(PREVIEW_SONG_ENV)
            .ok()
            .and_then(|value| Self::from_key(&value))
            .unwrap_or(Self::BOPEEBO)
    }

    pub fn from_key(value: &str) -> Option<Self> {
        match value
            .trim()
            .to_ascii_lowercase()
            .replace([' ', '_'], "-")
            .as_str()
        {
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

    const TUTORIAL: Self = Self {
        id: 0,
        folder: "tutorial",
        audio_prefix: "Tutorial",
    };
    const BOPEEBO: Self = Self {
        id: 1,
        folder: "bopeebo",
        audio_prefix: "Bopeebo",
    };
    const FRESH: Self = Self {
        id: 2,
        folder: "fresh",
        audio_prefix: "Fresh",
    };
    const DADBATTLE: Self = Self {
        id: 3,
        folder: "dadbattle",
        audio_prefix: "Dadbattle",
    };
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
}

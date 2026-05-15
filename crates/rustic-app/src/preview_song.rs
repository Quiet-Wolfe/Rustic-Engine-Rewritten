// LINT-ALLOW: long-file vanilla song registry and focused selection tests stay together.

use std::env;

const PREVIEW_SONG_ENV: &str = "RUSTIC_PREVIEW_SONG";
const PREVIEW_DIFFICULTY_ENV: &str = "RUSTIC_PREVIEW_DIFFICULTY";
pub const VARIATION_BF: &str = "bf";
pub const VARIATION_PICO: &str = "pico";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewSong {
    pub id: u32,
    pub folder: &'static str,
    pub audio_prefix: &'static str,
    display_name: &'static str,
    base_bpm: u16,
    base_ratings: [u8; 3],
    erect_bpm: Option<u16>,
    erect_ratings: Option<[u8; 2]>,
}

impl PreviewSong {
    pub const ALL: [Self; 26] = [
        Self::TUTORIAL,
        Self::BOPEEBO,
        Self::FRESH,
        Self::DADBATTLE,
        Self::SPOOKEEZ,
        Self::SOUTH,
        Self::MONSTER,
        Self::PICO,
        Self::PHILLY_NICE,
        Self::BLAMMED,
        Self::SATIN_PANTIES,
        Self::HIGH,
        Self::MILF,
        Self::COCOA,
        Self::EGGNOG,
        Self::WINTER_HORRORLAND,
        Self::SENPAI,
        Self::ROSES,
        Self::THORNS,
        Self::UGH,
        Self::GUNS,
        Self::STRESS,
        Self::DARNELL,
        Self::LIT_UP,
        Self::TWO_HOT,
        Self::BLAZIN,
    ];
    pub const FREEPLAY_EXTRA: [Self; 1] = [Self::SPAGHETTI];
    pub const CYCLABLE_WEEK1: [Self; 4] =
        [Self::TUTORIAL, Self::BOPEEBO, Self::FRESH, Self::DADBATTLE];
    const BASE_DIFFICULTIES: [PreviewDifficulty; 3] = [
        PreviewDifficulty::Easy,
        PreviewDifficulty::Normal,
        PreviewDifficulty::Hard,
    ];
    const VARIANT_DIFFICULTIES: [PreviewDifficulty; 5] = [
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
        let key = normalized_key(value);
        match key.as_str() {
            "dad-battle" => Some(Self::DADBATTLE),
            "2-hot" => Some(Self::TWO_HOT),
            "spaghetti" => Some(Self::SPAGHETTI),
            _ => Self::ALL
                .iter()
                .copied()
                .find(|song| song.matches_key(&key)),
        }
    }

    pub fn chart_path(self) -> String {
        self.chart_path_for(PreviewDifficulty::Normal)
    }

    pub fn chart_path_for(self, difficulty: PreviewDifficulty) -> String {
        self.chart_path_for_suffix(self.effective_variation_suffix(difficulty, None))
    }

    pub fn metadata_path(self) -> String {
        self.metadata_path_for(PreviewDifficulty::Normal)
    }

    pub fn metadata_path_for(self, difficulty: PreviewDifficulty) -> String {
        self.metadata_path_for_suffix(self.effective_variation_suffix(difficulty, None))
    }

    pub fn chart_path_for_suffix(self, suffix: Option<&str>) -> String {
        match suffix {
            Some(suffix) => format!(
                "data/songs/{}/{}-chart-{suffix}.json",
                self.folder, self.folder
            ),
            None => format!("data/songs/{}/{}-chart.json", self.folder, self.folder),
        }
    }

    pub fn metadata_path_for_suffix(self, suffix: Option<&str>) -> String {
        match suffix {
            Some(suffix) => format!(
                "data/songs/{}/{}-metadata-{suffix}.json",
                self.folder, self.folder
            ),
            None => format!("data/songs/{}/{}-metadata.json", self.folder, self.folder),
        }
    }

    pub fn has_variation(self, variation: &str) -> bool {
        match variation {
            VARIATION_BF => matches!(self.folder, "darnell" | "lit-up"),
            VARIATION_PICO => matches!(
                self.folder,
                "bopeebo"
                    | "fresh"
                    | "dadbattle"
                    | "spookeez"
                    | "south"
                    | "pico"
                    | "philly-nice"
                    | "blammed"
                    | "cocoa"
                    | "eggnog"
                    | "senpai"
                    | "roses"
                    | "ugh"
                    | "guns"
                    | "stress"
            ),
            _ => false,
        }
    }

    pub fn effective_variation_suffix(
        self,
        difficulty: PreviewDifficulty,
        variation: Option<&'static str>,
    ) -> Option<&'static str> {
        difficulty.chart_variation_suffix().or_else(|| {
            variation.and_then(|variation| self.has_variation(variation).then_some(variation))
        })
    }

    pub fn inst_path(self) -> String {
        format!("music/{}_Inst.ogg", self.audio_prefix)
    }

    pub fn voices_path(self) -> String {
        format!("music/{}_Voices.ogg", self.audio_prefix)
    }

    pub fn display_name(self) -> &'static str {
        self.display_name
    }

    pub fn starting_bpm_for(self, difficulty: PreviewDifficulty) -> u16 {
        match difficulty_for_song(self, difficulty) {
            PreviewDifficulty::Erect | PreviewDifficulty::Nightmare => {
                self.erect_bpm.unwrap_or(self.base_bpm)
            }
            PreviewDifficulty::Easy | PreviewDifficulty::Normal | PreviewDifficulty::Hard => {
                self.base_bpm
            }
        }
    }

    pub fn difficulty_rating_for(self, difficulty: PreviewDifficulty) -> u8 {
        match difficulty_for_song(self, difficulty) {
            PreviewDifficulty::Easy => self.base_ratings[0],
            PreviewDifficulty::Normal => self.base_ratings[1],
            PreviewDifficulty::Hard => self.base_ratings[2],
            PreviewDifficulty::Erect => self.erect_ratings.map(|ratings| ratings[0]).unwrap_or(0),
            PreviewDifficulty::Nightmare => {
                self.erect_ratings.map(|ratings| ratings[1]).unwrap_or(0)
            }
        }
    }

    pub fn next(self) -> Self {
        next_in(&Self::ALL, self)
    }

    pub fn previous(self) -> Self {
        previous_in(&Self::ALL, self)
    }

    pub fn available_difficulties(self) -> &'static [PreviewDifficulty] {
        if self.erect_ratings.is_some() {
            &Self::VARIANT_DIFFICULTIES
        } else {
            &Self::BASE_DIFFICULTIES
        }
    }

    fn matches_key(self, key: &str) -> bool {
        key == normalized_key(self.folder) || key == normalized_key(self.display_name)
    }

    pub const TUTORIAL: Self = song(0, "tutorial", "Tutorial", "Tutorial", 100, [0, 0, 1], None);
    pub const BOPEEBO: Self = song(
        1,
        "bopeebo",
        "Bopeebo",
        "Bopeebo",
        100,
        [1, 1, 2],
        Some((123, [7, 8])),
    );
    pub const FRESH: Self = song(
        2,
        "fresh",
        "Fresh",
        "Fresh",
        120,
        [1, 1, 2],
        Some((125, [6, 7])),
    );
    pub const DADBATTLE: Self = song(
        3,
        "dadbattle",
        "Dadbattle",
        "Dad Battle",
        180,
        [1, 2, 3],
        Some((190, [9, 10])),
    );
    pub const SPOOKEEZ: Self = song(
        4,
        "spookeez",
        "Spookeez",
        "Spookeez",
        150,
        [1, 1, 2],
        Some((166, [11, 12])),
    );
    pub const SOUTH: Self = song(
        5,
        "south",
        "South",
        "South",
        165,
        [1, 2, 2],
        Some((177, [8, 9])),
    );
    pub const MONSTER: Self = song(6, "monster", "Monster", "Monster", 95, [1, 2, 2], None);
    pub const PICO: Self = song(
        7,
        "pico",
        "Pico",
        "Pico",
        150,
        [1, 2, 2],
        Some((162, [9, 10])),
    );
    pub const PHILLY_NICE: Self = song(
        8,
        "philly-nice",
        "PhillyNice",
        "Philly Nice",
        175,
        [1, 2, 3],
        Some((175, [8, 9])),
    );
    pub const BLAMMED: Self = song(
        9,
        "blammed",
        "Blammed",
        "Blammed",
        165,
        [1, 2, 3],
        Some((170, [11, 12])),
    );
    pub const SATIN_PANTIES: Self = song(
        10,
        "satin-panties",
        "SatinPanties",
        "Satin Panties",
        110,
        [1, 2, 2],
        Some((135, [11, 12])),
    );
    pub const HIGH: Self = song(
        11,
        "high",
        "High",
        "High",
        125,
        [1, 2, 3],
        Some((125, [8, 9])),
    );
    pub const MILF: Self = song(12, "milf", "Milf", "M.I.L.F", 180, [2, 3, 4], None);
    pub const COCOA: Self = song(
        13,
        "cocoa",
        "Cocoa",
        "Cocoa",
        100,
        [1, 2, 2],
        Some((174, [7, 8])),
    );
    pub const EGGNOG: Self = song(
        14,
        "eggnog",
        "Eggnog",
        "Eggnog",
        150,
        [1, 2, 3],
        Some((140, [6, 7])),
    );
    pub const WINTER_HORRORLAND: Self = song(
        15,
        "winter-horrorland",
        "WinterHorrorland",
        "Winter Horrorland",
        159,
        [1, 2, 2],
        None,
    );
    pub const SENPAI: Self = song(
        16,
        "senpai",
        "Senpai",
        "Senpai",
        144,
        [1, 2, 3],
        Some((158, [6, 7])),
    );
    pub const ROSES: Self = song(
        17,
        "roses",
        "Roses",
        "Roses",
        120,
        [2, 3, 4],
        Some((128, [8, 9])),
    );
    pub const THORNS: Self = song(
        18,
        "thorns",
        "Thorns",
        "Thorns",
        190,
        [2, 3, 4],
        Some((190, [9, 10])),
    );
    pub const UGH: Self = song(19, "ugh", "Ugh", "Ugh", 160, [2, 3, 4], Some((170, [8, 9])));
    pub const GUNS: Self = song(20, "guns", "Guns", "Guns", 185, [3, 4, 5], None);
    pub const STRESS: Self = song(21, "stress", "Stress", "Stress", 178, [3, 4, 5], None);
    pub const DARNELL: Self = song(
        22,
        "darnell",
        "Darnell",
        "Darnell",
        155,
        [2, 3, 4],
        Some((155, [8, 9])),
    );
    pub const LIT_UP: Self = song(23, "lit-up", "LitUp", "Lit Up", 176, [2, 3, 4], None);
    pub const TWO_HOT: Self = song(24, "2hot", "2Hot", "2hot", 182, [3, 4, 5], None);
    pub const BLAZIN: Self = song(25, "blazin", "Blazin", "Blazin'", 180, [3, 4, 5], None);
    #[rustfmt::skip]
    pub const SPAGHETTI: Self = song(26, "spaghetti", "Spaghetti", "Spaghetti", 112, [2, 3, 5], None);
}

const fn song(
    id: u32,
    folder: &'static str,
    audio_prefix: &'static str,
    display_name: &'static str,
    base_bpm: u16,
    base_ratings: [u8; 3],
    erect: Option<(u16, [u8; 2])>,
) -> PreviewSong {
    let (erect_bpm, erect_ratings) = match erect {
        Some((bpm, ratings)) => (Some(bpm), Some(ratings)),
        None => (None, None),
    };
    PreviewSong {
        id,
        folder,
        audio_prefix,
        display_name,
        base_bpm,
        base_ratings,
        erect_bpm,
        erect_ratings,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreviewDifficulty {
    Easy,
    Normal,
    Hard,
    Erect,
    Nightmare,
}

impl PreviewDifficulty {
    const FREEPLAY_ORDER: [Self; 5] = [
        Self::Easy,
        Self::Normal,
        Self::Hard,
        Self::Erect,
        Self::Nightmare,
    ];

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

    pub fn next_freeplay(self) -> Self {
        next_in_slice(&Self::FREEPLAY_ORDER, self)
    }

    pub fn previous_freeplay(self) -> Self {
        previous_in_slice(&Self::FREEPLAY_ORDER, self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewSelection {
    pub song: PreviewSong,
    pub difficulty: PreviewDifficulty,
    pub variation: Option<&'static str>,
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
            variation: None,
        }
    }

    pub fn next_song(self) -> Self {
        let song = self.song.next();
        Self::new(song, self.difficulty).with_variation(self.variation)
    }

    pub fn previous_song(self) -> Self {
        let song = self.song.previous();
        Self::new(song, self.difficulty).with_variation(self.variation)
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

    pub fn with_difficulty(self, difficulty: PreviewDifficulty) -> Self {
        Self {
            difficulty: difficulty_for_song(self.song, difficulty),
            ..self
        }
    }

    pub fn with_variation(self, variation: Option<&'static str>) -> Self {
        Self {
            variation: variation.and_then(|id| self.song.has_variation(id).then_some(id)),
            ..self
        }
    }

    pub fn effective_variation_suffix(self) -> Option<&'static str> {
        self.song
            .effective_variation_suffix(self.difficulty, self.variation)
    }

    pub fn chart_path(self) -> String {
        self.song
            .chart_path_for_suffix(self.effective_variation_suffix())
    }

    pub fn metadata_path(self) -> String {
        self.song
            .metadata_path_for_suffix(self.effective_variation_suffix())
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
    fn preview_song_key_accepts_story_song_names() {
        for (key, song) in [
            ("tutorial", PreviewSong::TUTORIAL),
            ("Bopeebo", PreviewSong::BOPEEBO),
            ("Dad Battle", PreviewSong::DADBATTLE),
            ("philly_nice", PreviewSong::PHILLY_NICE),
            ("2 hot", PreviewSong::TWO_HOT),
            ("Blazin'", PreviewSong::BLAZIN),
            ("spaghetti", PreviewSong::SPAGHETTI),
        ] {
            assert_eq!(PreviewSong::from_key(key), Some(song));
        }
    }

    #[test]
    fn preview_song_registry_covers_vanilla_story_catalog() {
        assert_eq!(PreviewSong::ALL.len(), 26);
        assert_eq!(PreviewSong::ALL[0], PreviewSong::TUTORIAL);
        assert_eq!(PreviewSong::ALL[25], PreviewSong::BLAZIN);
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
                variation: None,
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
                variation: None,
            }
        );
        assert_eq!(
            PreviewSelection::from_keys(Some("tutorial"), Some("nightmare")),
            PreviewSelection {
                song: PreviewSong::TUTORIAL,
                difficulty: PreviewDifficulty::Normal,
                variation: None,
            }
        );
    }

    #[test]
    fn preview_selection_uses_character_variation_suffixes_for_base_difficulties() {
        let pico = PreviewSelection::new(PreviewSong::BOPEEBO, PreviewDifficulty::Hard)
            .with_variation(Some(VARIATION_PICO));
        assert_eq!(
            pico.chart_path(),
            "data/songs/bopeebo/bopeebo-chart-pico.json"
        );
        assert_eq!(
            pico.metadata_path(),
            "data/songs/bopeebo/bopeebo-metadata-pico.json"
        );

        let bf = PreviewSelection::new(PreviewSong::DARNELL, PreviewDifficulty::Normal)
            .with_variation(Some(VARIATION_BF));
        assert_eq!(bf.chart_path(), "data/songs/darnell/darnell-chart-bf.json");
    }

    #[test]
    fn erect_difficulty_takes_precedence_over_character_variation() {
        let selection = PreviewSelection::new(PreviewSong::BOPEEBO, PreviewDifficulty::Erect)
            .with_variation(Some(VARIATION_PICO));
        assert_eq!(
            selection.chart_path(),
            "data/songs/bopeebo/bopeebo-chart-erect.json"
        );
    }

    #[test]
    fn preview_selection_cycles_songs_and_difficulties() {
        let selection = PreviewSelection::from_keys(Some("dadbattle"), Some("hard"));
        assert_eq!(selection.next_song().song, PreviewSong::SPOOKEEZ);
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
            PreviewDifficulty::Nightmare
        );
        assert_eq!(
            PreviewSelection::from_keys(Some("tutorial"), Some("easy"))
                .previous_song()
                .song,
            PreviewSong::BLAZIN
        );
        assert_eq!(
            PreviewSelection::from_keys(Some("monster"), Some("easy"))
                .previous_difficulty()
                .difficulty,
            PreviewDifficulty::Hard
        );
    }

    #[test]
    fn freeplay_difficulty_cycle_matches_og_global_order() {
        assert_eq!(
            PreviewDifficulty::Easy.previous_freeplay(),
            PreviewDifficulty::Nightmare
        );
        assert_eq!(
            PreviewDifficulty::Hard.next_freeplay(),
            PreviewDifficulty::Erect
        );
        assert_eq!(
            PreviewDifficulty::Nightmare.next_freeplay(),
            PreviewDifficulty::Easy
        );
    }

    #[test]
    fn freeplay_metadata_values_match_vanilla_metadata() {
        assert_eq!(
            PreviewSong::TUTORIAL.difficulty_rating_for(PreviewDifficulty::Hard),
            1
        );
        assert_eq!(
            PreviewSong::BOPEEBO.starting_bpm_for(PreviewDifficulty::Nightmare),
            123
        );
        assert_eq!(
            PreviewSong::SPOOKEEZ.difficulty_rating_for(PreviewDifficulty::Nightmare),
            12
        );
        assert_eq!(
            PreviewSong::DADBATTLE.starting_bpm_for(PreviewDifficulty::Erect),
            190
        );
        assert_eq!(
            PreviewSong::WINTER_HORRORLAND.available_difficulties(),
            &PreviewSong::BASE_DIFFICULTIES
        );
        assert_eq!(
            PreviewSong::DARNELL.difficulty_rating_for(PreviewDifficulty::Erect),
            8
        );
    }
}

use crate::preview_song::{PreviewDifficulty, PreviewSelection};
use std::collections::HashMap;

const DIFFICULTY_ORDER: [PreviewDifficulty; 5] = [
    PreviewDifficulty::Easy,
    PreviewDifficulty::Normal,
    PreviewDifficulty::Hard,
    PreviewDifficulty::Erect,
    PreviewDifficulty::Nightmare,
];

#[derive(Debug, Clone)]
pub(super) struct FreeplaySongMetadata {
    base_album: String,
    variant_albums: HashMap<String, String>,
    base_ratings: FreeplayDifficultyRatings,
    variant_ratings: HashMap<String, FreeplayDifficultyRatings>,
    base_icon_id: Option<String>,
    variant_icon_ids: HashMap<String, String>,
}

impl FreeplaySongMetadata {
    pub(super) fn new(
        base_album: String,
        variant_albums: HashMap<String, String>,
        base_ratings: FreeplayDifficultyRatings,
        variant_ratings: HashMap<String, FreeplayDifficultyRatings>,
        base_icon_id: Option<String>,
        variant_icon_ids: HashMap<String, String>,
    ) -> Self {
        Self {
            base_album,
            variant_albums,
            base_ratings,
            variant_ratings,
            base_icon_id,
            variant_icon_ids,
        }
    }

    pub(super) fn album_id_for_selection(&self, selection: PreviewSelection) -> &str {
        selection
            .effective_variation_suffix()
            .and_then(|suffix| self.variant_albums.get(suffix))
            .unwrap_or(&self.base_album)
    }

    pub(super) fn rating_for_selection(&self, selection: PreviewSelection) -> Option<u8> {
        selection
            .effective_variation_suffix()
            .and_then(|suffix| self.variant_ratings.get(suffix))
            .and_then(|ratings| ratings.get(selection.difficulty))
            .or_else(|| self.base_ratings.get(selection.difficulty))
    }

    pub(super) fn icon_id_for_selection(&self, selection: PreviewSelection) -> Option<&str> {
        selection
            .effective_variation_suffix()
            .and_then(|suffix| self.variant_icon_ids.get(suffix))
            .or(self.base_icon_id.as_ref())
            .map(String::as_str)
    }

    pub(super) fn album_ids(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.base_album.as_str())
            .chain(self.variant_albums.values().map(String::as_str))
    }

    pub(super) fn difficulties_for_variation(
        &self,
        variation: Option<&str>,
    ) -> Vec<PreviewDifficulty> {
        let mut difficulties = variation
            .and_then(|suffix| self.variant_ratings.get(suffix))
            .map(|ratings| ratings.difficulties())
            .unwrap_or_else(|| self.base_ratings.difficulties());
        if variation.is_none() {
            if let Some(erect) = self.variant_ratings.get("erect") {
                difficulties.extend(erect.difficulties());
            }
        }
        ordered_unique_difficulties(difficulties)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct FreeplayDifficultyRatings {
    easy: Option<u8>,
    normal: Option<u8>,
    hard: Option<u8>,
    erect: Option<u8>,
    nightmare: Option<u8>,
}

impl FreeplayDifficultyRatings {
    pub(super) fn from_map(ratings: &HashMap<String, u8>) -> Self {
        Self {
            easy: ratings.get("easy").copied(),
            normal: ratings.get("normal").copied(),
            hard: ratings.get("hard").copied(),
            erect: ratings.get("erect").copied(),
            nightmare: ratings.get("nightmare").copied(),
        }
    }

    fn get(self, difficulty: PreviewDifficulty) -> Option<u8> {
        match difficulty {
            PreviewDifficulty::Easy => self.easy,
            PreviewDifficulty::Normal => self.normal,
            PreviewDifficulty::Hard => self.hard,
            PreviewDifficulty::Erect => self.erect,
            PreviewDifficulty::Nightmare => self.nightmare,
        }
    }

    fn difficulties(self) -> Vec<PreviewDifficulty> {
        DIFFICULTY_ORDER
            .into_iter()
            .filter(|difficulty| self.get(*difficulty).is_some())
            .collect()
    }
}

fn ordered_unique_difficulties(values: Vec<PreviewDifficulty>) -> Vec<PreviewDifficulty> {
    let mut ordered = DIFFICULTY_ORDER
        .into_iter()
        .filter(|difficulty| values.contains(difficulty))
        .collect::<Vec<_>>();
    if ordered.is_empty() {
        ordered.push(PreviewDifficulty::Normal);
    }
    ordered
}

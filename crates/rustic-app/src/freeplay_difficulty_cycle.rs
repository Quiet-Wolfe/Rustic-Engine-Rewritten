use super::song_metadata::FreeplaySongMetadata;
use super::FreeplayAssets;
use crate::preview_song::{PreviewDifficulty, PreviewSelection};

impl FreeplayAssets {
    pub fn clamp_selection_difficulty(&self, selection: PreviewSelection) -> PreviewSelection {
        selection.with_difficulty(self.valid_difficulty_for_selection(selection))
    }

    pub fn cycle_selection_difficulty(
        &self,
        selection: PreviewSelection,
        delta: isize,
    ) -> PreviewSelection {
        let difficulties = self.difficulties_for_selection(selection);
        let current = difficulties
            .iter()
            .position(|difficulty| *difficulty == selection.difficulty)
            .unwrap_or_else(|| {
                difficulties
                    .iter()
                    .position(|difficulty| *difficulty == PreviewDifficulty::Normal)
                    .unwrap_or(0)
            });
        let next = (current as isize + delta).rem_euclid(difficulties.len() as isize) as usize;
        selection.with_difficulty(difficulties[next])
    }

    fn valid_difficulty_for_selection(&self, selection: PreviewSelection) -> PreviewDifficulty {
        let difficulties = self.difficulties_for_selection(selection);
        if difficulties.contains(&selection.difficulty) {
            return selection.difficulty;
        }
        difficulties
            .iter()
            .copied()
            .find(|difficulty| *difficulty == PreviewDifficulty::Normal)
            .unwrap_or(difficulties[0])
    }

    fn difficulties_for_selection(&self, selection: PreviewSelection) -> Vec<PreviewDifficulty> {
        self.song_albums
            .get(&selection.song.id)
            .map(|metadata| difficulties_for_metadata(metadata, selection.variation))
            .unwrap_or_else(|| selection.song.available_difficulties().to_vec())
    }
}

fn difficulties_for_metadata(
    metadata: &FreeplaySongMetadata,
    variation: Option<&str>,
) -> Vec<PreviewDifficulty> {
    metadata.difficulties_for_variation(variation)
}

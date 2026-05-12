use super::App;
use crate::preview_song::{PreviewDifficulty, PreviewSelection, PreviewSong};
use rustic_core::time::Samples;

const SONG_END_TAIL_SECONDS: i64 = 2;

impl App {
    pub(super) fn start_story_playlist(
        &mut self,
        songs: Vec<PreviewSong>,
        difficulty: PreviewDifficulty,
    ) {
        let Some(first) = songs.first().copied() else {
            return;
        };
        self.story_playlist = songs;
        self.story_playlist_index = 0;
        self.story_playlist_difficulty = difficulty;
        self.preview_selection = PreviewSelection::new(first, difficulty);
        self.enter_play();
    }

    pub(super) fn clear_story_playlist(&mut self) {
        self.story_playlist.clear();
        self.story_playlist_index = 0;
    }

    pub(super) fn return_to_play_menu(&mut self) {
        if self.story_playlist.is_empty() {
            self.enter_song_select();
        } else {
            self.load_story_menu();
        }
    }

    pub(super) fn finish_song_if_due(&mut self, cursor: Samples, sample_rate: u32) -> bool {
        if self.game_over.is_some() || !self.song_started {
            return false;
        }
        let Some(play_state) = self.play_state.as_ref() else {
            return false;
        };
        let tail = i64::from(sample_rate.max(1)) * SONG_END_TAIL_SECONDS;
        if cursor.0 < play_state.chart_end_cursor().0.saturating_add(tail) {
            return false;
        }
        self.finish_current_song();
        true
    }

    fn finish_current_song(&mut self) {
        if self.story_playlist.is_empty() {
            self.enter_song_select();
            return;
        }
        let next_index = self.story_playlist_index + 1;
        if let Some(next_song) = self.story_playlist.get(next_index).copied() {
            self.story_playlist_index = next_index;
            self.preview_selection =
                PreviewSelection::new(next_song, self.story_playlist_difficulty);
            self.load_selected_song();
        } else {
            self.load_story_menu();
        }
    }
}

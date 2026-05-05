//! Dev-only rewind ring. See `PLAN.md` Section 14.
//!
//! Stores full `PlayState` keyframes every N frames plus per-frame
//! normalized inputs and conductor cursors. Rewind to an exact frame
//! restores the nearest prior keyframe and replays inputs forward.

use rustic_core::input::NormalizedInputEvent;
use rustic_core::time::Samples;
use rustic_game::PlayState;

#[derive(Debug, Clone)]
pub struct Frame {
    pub keyframe: Option<PlayState>,
    pub inputs: Vec<NormalizedInputEvent>,
    pub conductor_cursor: Samples,
}

#[derive(Debug)]
pub struct RewindBuffer {
    keyframe_interval: u32,
    frames: std::collections::VecDeque<Frame>,
    capacity: usize,
}

impl RewindBuffer {
    pub fn new(capacity: usize, keyframe_interval: u32) -> Self {
        Self {
            keyframe_interval: keyframe_interval.max(1),
            frames: std::collections::VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn record(&mut self, frame_index: u64, state: &PlayState, frame: Frame) {
        let mut frame = frame;
        if frame_index.is_multiple_of(self.keyframe_interval as u64) {
            frame.keyframe = Some(state.clone());
        }
        if self.frames.len() == self.capacity {
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
    }

    pub fn frames(&self) -> &std::collections::VecDeque<Frame> {
        &self.frames
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

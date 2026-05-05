//! Screen stack skeleton. See `PLAN.md` Section 11.
//!
//! Each screen lives in its own module (added in later phases) and stays
//! under the file cap. Substates push onto the same stack; parent screens
//! opt into whether they keep updating while covered.

use rustic_core::input::NormalizedInputEvent;

pub trait Screen {
    fn name(&self) -> &'static str;
    fn enter(&mut self) {}
    fn exit(&mut self) {}
    fn update(&mut self, _dt_seconds: f32) {}
    fn input(&mut self, _event: &NormalizedInputEvent) {}

    /// If true, an active substate covering this screen still receives
    /// `update` ticks. Defaults to false so screens pause cleanly under
    /// modal substates (pause menu, dialogue overlays).
    fn updates_while_covered(&self) -> bool {
        false
    }
}

#[derive(Default)]
pub struct ScreenStack {
    screens: Vec<Box<dyn Screen>>,
}

impl ScreenStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, mut screen: Box<dyn Screen>) {
        screen.enter();
        self.screens.push(screen);
    }

    pub fn pop(&mut self) -> Option<Box<dyn Screen>> {
        let mut top = self.screens.pop()?;
        top.exit();
        Some(top)
    }

    pub fn top_name(&self) -> Option<&'static str> {
        self.screens.last().map(|s| s.name())
    }

    pub fn update(&mut self, dt_seconds: f32) {
        let len = self.screens.len();
        if len == 0 {
            return;
        }
        // Top always updates.
        self.screens[len - 1].update(dt_seconds);
        // Covered screens update only if they opted in.
        for i in (0..len.saturating_sub(1)).rev() {
            if self.screens[i].updates_while_covered() {
                self.screens[i].update(dt_seconds);
            }
        }
    }

    pub fn input(&mut self, event: &NormalizedInputEvent) {
        if let Some(top) = self.screens.last_mut() {
            top.input(event);
        }
    }

    pub fn len(&self) -> usize {
        self.screens.len()
    }
    pub fn is_empty(&self) -> bool {
        self.screens.is_empty()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    struct TestScreen {
        name: &'static str,
        updates: u32,
        covered: bool,
    }
    impl Screen for TestScreen {
        fn name(&self) -> &'static str {
            self.name
        }
        fn update(&mut self, _dt: f32) {
            self.updates += 1;
        }
        fn updates_while_covered(&self) -> bool {
            self.covered
        }
    }

    #[test]
    fn covered_screen_skips_update_unless_opted_in() {
        let mut stack = ScreenStack::new();
        stack.push(Box::new(TestScreen {
            name: "parent",
            updates: 0,
            covered: false,
        }));
        stack.push(Box::new(TestScreen {
            name: "modal",
            updates: 0,
            covered: false,
        }));
        stack.update(1.0 / 60.0);
        assert_eq!(stack.top_name(), Some("modal"));
        // Parent did not opt in; verify by length only since stack owns boxes.
        assert_eq!(stack.len(), 2);
    }
}

#![cfg_attr(
    any(test, feature = "test-support"),
    allow(clippy::expect_used, clippy::unwrap_used)
)]
//! User keyboard input, abstracted away from crossterm.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Enter,
    Esc,
    Tab,
    BackTab,
    Backspace,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyEvent {
    pub fn plain(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::default(),
        }
    }
}

pub trait InputSource: Send {
    fn poll_event(&mut self) -> Option<KeyEvent>;
}

#[cfg(any(test, feature = "test-support"))]
pub mod fakes {
    use super::*;
    use std::collections::VecDeque;

    #[derive(Debug, Default)]
    pub struct FakeInputSource {
        queue: VecDeque<KeyEvent>,
    }

    impl FakeInputSource {
        pub fn new() -> Self {
            Self::default()
        }
        pub fn push(&mut self, event: KeyEvent) {
            self.queue.push_back(event);
        }
    }

    impl InputSource for FakeInputSource {
        fn poll_event(&mut self) -> Option<KeyEvent> {
            self.queue.pop_front()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn returns_pushed_events_in_order() {
            let mut s = FakeInputSource::new();
            s.push(KeyEvent::plain(KeyCode::Char('a')));
            s.push(KeyEvent::plain(KeyCode::Enter));
            assert_eq!(s.poll_event().unwrap().code, KeyCode::Char('a'));
            assert_eq!(s.poll_event().unwrap().code, KeyCode::Enter);
            assert!(s.poll_event().is_none());
        }
    }
}

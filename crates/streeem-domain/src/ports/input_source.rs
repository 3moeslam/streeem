#![cfg_attr(
    any(test, feature = "test-support"),
    allow(clippy::expect_used, clippy::unwrap_used)
)]
//! User keyboard and mouse input, abstracted away from crossterm.

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

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    /// Any button not natively represented (e.g. button 4+).
    Other,
}

/// What the mouse did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventKind {
    Down(MouseButton),
    Up(MouseButton),
    Drag(MouseButton),
    Moved,
    ScrollDown,
    ScrollUp,
    ScrollLeft,
    ScrollRight,
}

/// A single mouse event with absolute terminal coordinates (0-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub column: u16,
    pub row: u16,
    pub modifiers: KeyModifiers,
}

/// Top-level input event: either a key press or a mouse event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}

pub trait InputSource: Send {
    fn poll_event(&mut self) -> Option<InputEvent>;
}

#[cfg(any(test, feature = "test-support"))]
pub mod fakes {
    use super::*;
    use std::collections::VecDeque;

    #[derive(Debug, Default)]
    pub struct FakeInputSource {
        queue: VecDeque<InputEvent>,
    }

    impl FakeInputSource {
        pub fn new() -> Self {
            Self::default()
        }
        pub fn push_key(&mut self, event: KeyEvent) {
            self.queue.push_back(InputEvent::Key(event));
        }
        pub fn push_mouse(&mut self, event: MouseEvent) {
            self.queue.push_back(InputEvent::Mouse(event));
        }
    }

    impl InputSource for FakeInputSource {
        fn poll_event(&mut self) -> Option<InputEvent> {
            self.queue.pop_front()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn returns_pushed_events_in_order() {
            let mut s = FakeInputSource::new();
            s.push_key(KeyEvent::plain(KeyCode::Char('a')));
            s.push_key(KeyEvent::plain(KeyCode::Enter));
            assert_eq!(
                s.poll_event().unwrap(),
                InputEvent::Key(KeyEvent::plain(KeyCode::Char('a')))
            );
            assert_eq!(
                s.poll_event().unwrap(),
                InputEvent::Key(KeyEvent::plain(KeyCode::Enter))
            );
            assert!(s.poll_event().is_none());
        }
    }
}

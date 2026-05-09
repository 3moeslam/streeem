use std::time::Duration;

use crossterm::event::{
    self, Event, KeyCode as CtKeyCode, KeyEvent as CtKeyEvent, KeyEventKind, KeyModifiers as CtMods,
};
use streeem_domain::ports::input_source::{InputSource, KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Default, Clone, Copy)]
pub struct CrosstermInputAdapter;

impl CrosstermInputAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl InputSource for CrosstermInputAdapter {
    fn poll_event(&mut self) -> Option<KeyEvent> {
        // Drain non-key events (Resize, Mouse, FocusGained/Lost, Paste) and
        // Release/Repeat key events, so a queued keystroke isn't masked by
        // an event we don't care about. Returns the first Press key event.
        while event::poll(Duration::from_millis(0)).ok()? {
            if let Event::Key(k) = event::read().ok()?
                && k.kind == KeyEventKind::Press
            {
                return Some(translate(k));
            }
        }
        None
    }
}

fn translate(k: CtKeyEvent) -> KeyEvent {
    let code = match k.code {
        CtKeyCode::Char(c) => KeyCode::Char(c),
        CtKeyCode::Enter => KeyCode::Enter,
        CtKeyCode::Esc => KeyCode::Esc,
        CtKeyCode::Tab => KeyCode::Tab,
        CtKeyCode::BackTab => KeyCode::BackTab,
        CtKeyCode::Backspace => KeyCode::Backspace,
        CtKeyCode::Up => KeyCode::Up,
        CtKeyCode::Down => KeyCode::Down,
        CtKeyCode::Left => KeyCode::Left,
        CtKeyCode::Right => KeyCode::Right,
        CtKeyCode::PageUp => KeyCode::PageUp,
        CtKeyCode::PageDown => KeyCode::PageDown,
        CtKeyCode::Home => KeyCode::Home,
        CtKeyCode::End => KeyCode::End,
        _ => KeyCode::Esc,
    };
    let modifiers = KeyModifiers {
        ctrl: k.modifiers.contains(CtMods::CONTROL),
        shift: k.modifiers.contains(CtMods::SHIFT),
        alt: k.modifiers.contains(CtMods::ALT),
    };
    KeyEvent { code, modifiers }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_ctrl_c() {
        let k = CtKeyEvent::new(CtKeyCode::Char('c'), CtMods::CONTROL);
        let out = translate(k);
        assert_eq!(out.code, KeyCode::Char('c'));
        assert!(out.modifiers.ctrl);
    }

    #[test]
    fn translates_tab() {
        let k = CtKeyEvent::new(CtKeyCode::Tab, CtMods::NONE);
        assert_eq!(translate(k).code, KeyCode::Tab);
    }
}

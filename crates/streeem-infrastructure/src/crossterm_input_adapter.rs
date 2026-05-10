use std::time::Duration;

use crossterm::event::{
    self, Event, KeyCode as CtKeyCode, KeyEvent as CtKeyEvent, KeyEventKind,
    KeyModifiers as CtMods, MouseButton as CtMouseButton, MouseEvent as CtMouseEvent,
    MouseEventKind as CtMouseEventKind,
};
use streeem_domain::ports::input_source::{
    InputEvent, InputSource, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct CrosstermInputAdapter;

impl CrosstermInputAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl InputSource for CrosstermInputAdapter {
    fn poll_event(&mut self) -> Option<InputEvent> {
        // Drain events; return the first meaningful one.
        while event::poll(Duration::from_millis(0)).ok()? {
            match event::read().ok()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => {
                    return Some(InputEvent::Key(translate_key(k)));
                }
                Event::Mouse(m) => {
                    if let Some(ev) = translate_mouse(m) {
                        return Some(InputEvent::Mouse(ev));
                    }
                }
                _ => {}
            }
        }
        None
    }
}

fn translate_key(k: CtKeyEvent) -> KeyEvent {
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
    let modifiers = translate_modifiers(k.modifiers);
    KeyEvent { code, modifiers }
}

fn translate_mouse(m: CtMouseEvent) -> Option<MouseEvent> {
    let kind = match m.kind {
        CtMouseEventKind::Down(b) => MouseEventKind::Down(translate_button(b)),
        CtMouseEventKind::Up(b) => MouseEventKind::Up(translate_button(b)),
        CtMouseEventKind::Drag(b) => MouseEventKind::Drag(translate_button(b)),
        CtMouseEventKind::Moved => MouseEventKind::Moved,
        CtMouseEventKind::ScrollDown => MouseEventKind::ScrollDown,
        CtMouseEventKind::ScrollUp => MouseEventKind::ScrollUp,
        CtMouseEventKind::ScrollLeft => MouseEventKind::ScrollLeft,
        CtMouseEventKind::ScrollRight => MouseEventKind::ScrollRight,
    };
    Some(MouseEvent {
        kind,
        column: m.column,
        row: m.row,
        modifiers: translate_modifiers(m.modifiers),
    })
}

fn translate_button(b: CtMouseButton) -> MouseButton {
    match b {
        CtMouseButton::Left => MouseButton::Left,
        CtMouseButton::Middle => MouseButton::Middle,
        CtMouseButton::Right => MouseButton::Right,
    }
}

fn translate_modifiers(m: CtMods) -> KeyModifiers {
    KeyModifiers {
        ctrl: m.contains(CtMods::CONTROL),
        shift: m.contains(CtMods::SHIFT),
        alt: m.contains(CtMods::ALT),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn translates_ctrl_c() {
        let k = CtKeyEvent::new(CtKeyCode::Char('c'), CtMods::CONTROL);
        let out = translate_key(k);
        assert_eq!(out.code, KeyCode::Char('c'));
        assert!(out.modifiers.ctrl);
    }

    #[test]
    fn translates_tab() {
        let k = CtKeyEvent::new(CtKeyCode::Tab, CtMods::NONE);
        assert_eq!(translate_key(k).code, KeyCode::Tab);
    }

    #[test]
    fn translate_mouse_scroll_up_maps_to_wheel_up() {
        let m = CtMouseEvent {
            kind: CtMouseEventKind::ScrollUp,
            column: 10,
            row: 5,
            modifiers: CtMods::NONE,
        };
        let ev = translate_mouse(m).unwrap();
        assert_eq!(ev.kind, MouseEventKind::ScrollUp);
        assert_eq!(ev.column, 10);
        assert_eq!(ev.row, 5);
    }
}

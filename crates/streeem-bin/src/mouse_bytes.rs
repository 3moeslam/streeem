//! Encode a domain [`MouseEvent`] as an SGR 1006 escape sequence
//! (`ESC[<Pb;Px;PyM` on press/drag, `...m` on release).
//!
//! Button codes (SGR 1006):
//! - 0 = left button
//! - 1 = middle button
//! - 2 = right button
//! - 64 = wheel up
//! - 65 = wheel down
//!
//! Modifier bits: shift += 4, alt += 8, ctrl += 16.
//!
//! Returns `None` for events that have no standard SGR representation
//! (`Moved`, `ScrollLeft`, `ScrollRight`, `MouseButton::Other`).

use streeem_domain::ports::input_source::{MouseButton, MouseEvent, MouseEventKind};

/// Convert a [`MouseEvent`] to its SGR 1006 byte representation.
///
/// Returns `None` if the event kind has no standard SGR encoding.
pub fn mouse_to_bytes(m: MouseEvent) -> Option<Vec<u8>> {
    let base_btn = button_press_code(m.kind)?;

    // Modifier bits
    let mut btn = base_btn;
    if m.modifiers.shift {
        btn += 4;
    }
    if m.modifiers.alt {
        btn += 8;
    }
    if m.modifiers.ctrl {
        btn += 16;
    }

    // SGR uses 1-based coordinates
    let col = m.column.saturating_add(1);
    let row = m.row.saturating_add(1);

    // Final character: 'M' for press/scroll/drag, 'm' for release
    let final_char = if is_release(m.kind) { b'm' } else { b'M' };

    let seq = format!("\x1b[<{btn};{col};{row}{}", final_char as char);
    Some(seq.into_bytes())
}

/// Returns the SGR button code for a given event kind, or `None` if
/// the event has no standard SGR representation.
fn button_press_code(kind: MouseEventKind) -> Option<u8> {
    match kind {
        MouseEventKind::Down(b) | MouseEventKind::Up(b) => plain_button_code(b),
        MouseEventKind::Drag(b) => plain_button_code(b).map(|c| c + 32),
        MouseEventKind::ScrollUp => Some(64),
        MouseEventKind::ScrollDown => Some(65),
        // No standard SGR code for lateral scroll or motion
        MouseEventKind::ScrollLeft | MouseEventKind::ScrollRight | MouseEventKind::Moved => None,
    }
}

fn plain_button_code(b: MouseButton) -> Option<u8> {
    match b {
        MouseButton::Left => Some(0),
        MouseButton::Middle => Some(1),
        MouseButton::Right => Some(2),
        MouseButton::Other => None,
    }
}

fn is_release(kind: MouseEventKind) -> bool {
    matches!(kind, MouseEventKind::Up(_))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use streeem_domain::ports::input_source::{
        KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };

    fn plain_mouse(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::default(),
        }
    }

    #[test]
    fn wheel_up() {
        let bytes = mouse_to_bytes(plain_mouse(MouseEventKind::ScrollUp, 4, 2)).unwrap();
        assert_eq!(bytes, b"\x1b[<64;5;3M");
    }

    #[test]
    fn wheel_down() {
        let bytes = mouse_to_bytes(plain_mouse(MouseEventKind::ScrollDown, 0, 0)).unwrap();
        assert_eq!(bytes, b"\x1b[<65;1;1M");
    }

    #[test]
    fn left_click_down() {
        let bytes =
            mouse_to_bytes(plain_mouse(MouseEventKind::Down(MouseButton::Left), 9, 4)).unwrap();
        assert_eq!(bytes, b"\x1b[<0;10;5M");
    }

    #[test]
    fn left_click_up() {
        let bytes =
            mouse_to_bytes(plain_mouse(MouseEventKind::Up(MouseButton::Left), 9, 4)).unwrap();
        assert_eq!(bytes, b"\x1b[<0;10;5m");
    }

    #[test]
    fn moved_returns_none() {
        assert!(mouse_to_bytes(plain_mouse(MouseEventKind::Moved, 0, 0)).is_none());
    }

    #[test]
    fn ctrl_modifier() {
        let ev = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers {
                ctrl: true,
                shift: false,
                alt: false,
            },
        };
        let bytes = mouse_to_bytes(ev).unwrap();
        // ctrl = +16, wheel-up base = 64, total = 80; col=1, row=1
        assert_eq!(bytes, b"\x1b[<80;1;1M");
    }
}

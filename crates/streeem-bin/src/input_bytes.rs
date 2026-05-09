//! Translate KeyEvent → bytes for input-mode forwarding to a child PTY.

use streeem_domain::ports::input_source::{KeyCode, KeyEvent};

/// Returns the byte sequence to send to the PTY for this key, or None if the
/// key is not forwarded (e.g., Esc which exits input mode).
pub fn key_to_bytes(key: KeyEvent) -> Option<Vec<u8>> {
    match key.code {
        KeyCode::Esc => None, // handled as mode-exit by caller
        KeyCode::Enter => Some(b"\r".to_vec()),
        KeyCode::Tab => Some(b"\t".to_vec()),
        KeyCode::BackTab => Some(b"\x1b[Z".to_vec()),
        KeyCode::Backspace => Some(b"\x7f".to_vec()),
        KeyCode::Up => Some(b"\x1b[A".to_vec()),
        KeyCode::Down => Some(b"\x1b[B".to_vec()),
        KeyCode::Right => Some(b"\x1b[C".to_vec()),
        KeyCode::Left => Some(b"\x1b[D".to_vec()),
        KeyCode::Home => Some(b"\x1b[H".to_vec()),
        KeyCode::End => Some(b"\x1b[F".to_vec()),
        KeyCode::PageUp => Some(b"\x1b[5~".to_vec()),
        KeyCode::PageDown => Some(b"\x1b[6~".to_vec()),
        KeyCode::Char(c) => {
            if key.modifiers.ctrl {
                let lc = c.to_ascii_lowercase();
                if lc.is_ascii_lowercase() {
                    Some(vec![(lc as u8) - b'a' + 1])
                } else {
                    Some(c.to_string().into_bytes())
                }
            } else {
                Some(c.to_string().into_bytes())
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use streeem_domain::ports::input_source::KeyModifiers;

    fn k(code: KeyCode) -> KeyEvent {
        KeyEvent::plain(code)
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers {
                ctrl: true,
                shift: false,
                alt: false,
            },
        }
    }

    #[test]
    fn esc_returns_none() {
        assert_eq!(key_to_bytes(k(KeyCode::Esc)), None);
    }

    #[test]
    fn enter_returns_carriage_return() {
        assert_eq!(key_to_bytes(k(KeyCode::Enter)), Some(b"\r".to_vec()));
    }

    #[test]
    fn arrow_keys_map_to_csi_sequences() {
        assert_eq!(key_to_bytes(k(KeyCode::Up)), Some(b"\x1b[A".to_vec()));
        assert_eq!(key_to_bytes(k(KeyCode::Down)), Some(b"\x1b[B".to_vec()));
        assert_eq!(key_to_bytes(k(KeyCode::Right)), Some(b"\x1b[C".to_vec()));
        assert_eq!(key_to_bytes(k(KeyCode::Left)), Some(b"\x1b[D".to_vec()));
    }

    #[test]
    fn ctrl_letter_maps_to_control_byte() {
        assert_eq!(key_to_bytes(ctrl('c')), Some(vec![3]));
        assert_eq!(key_to_bytes(ctrl('d')), Some(vec![4]));
    }

    #[test]
    fn plain_char_maps_to_utf8() {
        assert_eq!(key_to_bytes(k(KeyCode::Char('a'))), Some(b"a".to_vec()));
    }

    #[test]
    fn backspace_maps_to_del() {
        assert_eq!(key_to_bytes(k(KeyCode::Backspace)), Some(b"\x7f".to_vec()));
    }
}

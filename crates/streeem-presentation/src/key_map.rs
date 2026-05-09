//! Pure mapping from KeyEvent + current snapshot to an application Command.
//!
//! v0.2.0+ design: app commands require the Ctrl modifier. Any key without
//! Ctrl, or any Ctrl combo not mapped here, is forwarded to the focused
//! tile's PTY as raw bytes. This eliminates the explicit input/command
//! mode toggle.

use streeem_application::command::{Command, ScrollDelta};
use streeem_application::query::RenderSnapshot;
use streeem_domain::grid::FocusMove;
use streeem_domain::ports::input_source::{KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppIntent {
    Quit,
    PromptAddTile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyOutcome {
    Command(Command),
    Intent(AppIntent),
    Forward,
}

pub const STATUS_BAR_TEXT: &str = "^A:add  ^X:drop  ^N/^P:next/prev  ^F:follow-tail  ^T/^B:scroll  ^Q:quit  (other keys \u{2192} focused tile)";

pub const STATUS_BAR_TEXT_FORWARDING: &str =
    "type to focused tile  \u{2022}  Esc Esc:command mode  \u{2022}  Ctrl+Q:quit";

pub const STATUS_BAR_TEXT_COMMAND: &str =
    "[CMD] a:new shell  x:drop  n:next  p:prev  f:follow  q:quit  Esc:exit";

pub fn map(key: KeyEvent, snap: &RenderSnapshot) -> KeyOutcome {
    use KeyCode::Char;
    let focused = snap.focused;

    if !key.modifiers.ctrl {
        return KeyOutcome::Forward;
    }
    let Char(c) = key.code else {
        return KeyOutcome::Forward;
    };
    let lc = c.to_ascii_lowercase();

    match lc {
        'q' => KeyOutcome::Intent(AppIntent::Quit),
        'a' => KeyOutcome::Intent(AppIntent::PromptAddTile),
        'x' => focused
            .map(|id| KeyOutcome::Command(Command::DropTile(id)))
            .unwrap_or(KeyOutcome::Forward),
        'n' => KeyOutcome::Command(Command::MoveFocus(FocusMove::CycleForward)),
        'p' => KeyOutcome::Command(Command::MoveFocus(FocusMove::CycleBackward)),
        'f' => focused
            .map(|id| KeyOutcome::Command(Command::ToggleFollowTail(id)))
            .unwrap_or(KeyOutcome::Forward),
        't' => focused
            .map(|id| {
                KeyOutcome::Command(Command::ScrollTile {
                    id,
                    delta: ScrollDelta::Top,
                })
            })
            .unwrap_or(KeyOutcome::Forward),
        'b' => focused
            .map(|id| {
                KeyOutcome::Command(Command::ScrollTile {
                    id,
                    delta: ScrollDelta::Bottom,
                })
            })
            .unwrap_or(KeyOutcome::Forward),
        // Ctrl+C, Ctrl+D, Ctrl+I, Ctrl+M, Ctrl+L, Ctrl+R, Ctrl+S, Ctrl+U,
        // Ctrl+W, Ctrl+Z, etc. — fall through to the tile.
        _ => KeyOutcome::Forward,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use streeem_domain::ports::input_source::KeyModifiers;
    use streeem_domain::tile_id::TileId;

    fn snap(focused: Option<TileId>) -> RenderSnapshot {
        RenderSnapshot {
            terminal_size: (80, 30),
            placements: Vec::new(),
            tiles: Vec::new(),
            focused,
            alerts: Vec::new(),
            too_small: false,
        }
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
    fn ctrl_q_means_quit() {
        assert_eq!(
            map(ctrl('q'), &snap(None)),
            KeyOutcome::Intent(AppIntent::Quit)
        );
    }

    #[test]
    fn ctrl_a_opens_prompt() {
        assert_eq!(
            map(ctrl('a'), &snap(None)),
            KeyOutcome::Intent(AppIntent::PromptAddTile)
        );
    }

    #[test]
    fn ctrl_x_drops_focused_tile() {
        let id = TileId::default_from(7);
        assert_eq!(
            map(ctrl('x'), &snap(Some(id))),
            KeyOutcome::Command(Command::DropTile(id))
        );
    }

    #[test]
    fn ctrl_x_with_no_focus_forwards() {
        assert_eq!(map(ctrl('x'), &snap(None)), KeyOutcome::Forward);
    }

    #[test]
    fn ctrl_n_cycles_forward() {
        assert_eq!(
            map(ctrl('n'), &snap(None)),
            KeyOutcome::Command(Command::MoveFocus(FocusMove::CycleForward))
        );
    }

    #[test]
    fn ctrl_p_cycles_backward() {
        assert_eq!(
            map(ctrl('p'), &snap(None)),
            KeyOutcome::Command(Command::MoveFocus(FocusMove::CycleBackward))
        );
    }

    #[test]
    fn plain_letter_forwards() {
        assert_eq!(
            map(KeyEvent::plain(KeyCode::Char('h')), &snap(None)),
            KeyOutcome::Forward
        );
    }

    #[test]
    fn plain_enter_forwards() {
        assert_eq!(
            map(KeyEvent::plain(KeyCode::Enter), &snap(None)),
            KeyOutcome::Forward
        );
    }

    #[test]
    fn ctrl_c_forwards_so_sigint_works() {
        assert_eq!(map(ctrl('c'), &snap(None)), KeyOutcome::Forward);
    }

    #[test]
    fn ctrl_d_forwards_so_eof_works() {
        assert_eq!(map(ctrl('d'), &snap(None)), KeyOutcome::Forward);
    }

    #[test]
    fn tab_forwards() {
        assert_eq!(
            map(KeyEvent::plain(KeyCode::Tab), &snap(None)),
            KeyOutcome::Forward
        );
    }

    #[test]
    fn arrow_keys_forward() {
        assert_eq!(
            map(KeyEvent::plain(KeyCode::Up), &snap(None)),
            KeyOutcome::Forward
        );
        assert_eq!(
            map(KeyEvent::plain(KeyCode::Down), &snap(None)),
            KeyOutcome::Forward
        );
        assert_eq!(
            map(KeyEvent::plain(KeyCode::Left), &snap(None)),
            KeyOutcome::Forward
        );
        assert_eq!(
            map(KeyEvent::plain(KeyCode::Right), &snap(None)),
            KeyOutcome::Forward
        );
    }
}

//! Pure mapping from KeyEvent + current snapshot to an application Command.

#![allow(clippy::cast_possible_truncation)]

use streeem_application::command::{Command, ScrollDelta};
use streeem_application::query::RenderSnapshot;
use streeem_domain::grid::{FocusMove, SpatialDirection};
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
    Ignored,
}

pub fn map(key: KeyEvent, snap: &RenderSnapshot) -> KeyOutcome {
    use KeyCode::*;
    let focused = snap.focused;

    match (key.code, key.modifiers.ctrl) {
        (Char('q'), false) => KeyOutcome::Intent(AppIntent::Quit),
        (Char('c'), true) => KeyOutcome::Intent(AppIntent::Quit),
        (Char('a'), false) => KeyOutcome::Intent(AppIntent::PromptAddTile),
        (Char('d'), false) => focused
            .map(|id| KeyOutcome::Command(Command::DropTile(id)))
            .unwrap_or(KeyOutcome::Ignored),
        (Char('+'), false) => focused
            .map(|id| KeyOutcome::Command(Command::ResizeTile { id, delta_rows: 1 }))
            .unwrap_or(KeyOutcome::Ignored),
        (Char('-'), false) => focused
            .map(|id| KeyOutcome::Command(Command::ResizeTile { id, delta_rows: -1 }))
            .unwrap_or(KeyOutcome::Ignored),
        (Char('f'), false) => focused
            .map(|id| KeyOutcome::Command(Command::ToggleFollowTail(id)))
            .unwrap_or(KeyOutcome::Ignored),
        (Char('g'), false) => focused
            .map(|id| {
                KeyOutcome::Command(Command::ScrollTile {
                    id,
                    delta: ScrollDelta::Top,
                })
            })
            .unwrap_or(KeyOutcome::Ignored),
        (Char('G'), false) => focused
            .map(|id| {
                KeyOutcome::Command(Command::ScrollTile {
                    id,
                    delta: ScrollDelta::Bottom,
                })
            })
            .unwrap_or(KeyOutcome::Ignored),
        (Char(c), false) if c.is_ascii_digit() && c != '0' => {
            let n = c.to_digit(10).unwrap_or(1) as u8;
            KeyOutcome::Command(Command::MoveFocus(FocusMove::Index(n)))
        }
        (Tab, false) => KeyOutcome::Command(Command::MoveFocus(FocusMove::CycleForward)),
        (BackTab, _) => KeyOutcome::Command(Command::MoveFocus(FocusMove::CycleBackward)),
        (PageUp, false) => focused
            .map(|id| {
                KeyOutcome::Command(Command::ScrollTile {
                    id,
                    delta: ScrollDelta::Page(1),
                })
            })
            .unwrap_or(KeyOutcome::Ignored),
        (PageDown, false) => focused
            .map(|id| {
                KeyOutcome::Command(Command::ScrollTile {
                    id,
                    delta: ScrollDelta::Page(-1),
                })
            })
            .unwrap_or(KeyOutcome::Ignored),
        (Left, false) => KeyOutcome::Command(Command::MoveFocus(FocusMove::Spatial(
            SpatialDirection::Left,
        ))),
        (Right, false) => KeyOutcome::Command(Command::MoveFocus(FocusMove::Spatial(
            SpatialDirection::Right,
        ))),
        (Up, false) => {
            KeyOutcome::Command(Command::MoveFocus(FocusMove::Spatial(SpatialDirection::Up)))
        }
        (Down, false) => KeyOutcome::Command(Command::MoveFocus(FocusMove::Spatial(
            SpatialDirection::Down,
        ))),
        _ => KeyOutcome::Ignored,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn q_means_quit() {
        let r = map(KeyEvent::plain(KeyCode::Char('q')), &snap(None));
        assert_eq!(r, KeyOutcome::Intent(AppIntent::Quit));
    }

    #[test]
    fn ctrl_c_means_quit() {
        let mut k = KeyEvent::plain(KeyCode::Char('c'));
        k.modifiers.ctrl = true;
        assert_eq!(map(k, &snap(None)), KeyOutcome::Intent(AppIntent::Quit));
    }

    #[test]
    fn d_with_no_focus_is_ignored() {
        assert_eq!(
            map(KeyEvent::plain(KeyCode::Char('d')), &snap(None)),
            KeyOutcome::Ignored
        );
    }

    #[test]
    fn d_with_focus_drops_focused_tile() {
        let id = TileId::default_from(7);
        assert_eq!(
            map(KeyEvent::plain(KeyCode::Char('d')), &snap(Some(id))),
            KeyOutcome::Command(Command::DropTile(id))
        );
    }

    #[test]
    fn plus_and_minus_resize_focused_tile() {
        let id = TileId::default_from(2);
        assert_eq!(
            map(KeyEvent::plain(KeyCode::Char('+')), &snap(Some(id))),
            KeyOutcome::Command(Command::ResizeTile { id, delta_rows: 1 })
        );
        assert_eq!(
            map(KeyEvent::plain(KeyCode::Char('-')), &snap(Some(id))),
            KeyOutcome::Command(Command::ResizeTile { id, delta_rows: -1 })
        );
    }

    #[test]
    fn digit_keys_jump_focus_by_index() {
        assert_eq!(
            map(KeyEvent::plain(KeyCode::Char('3')), &snap(None)),
            KeyOutcome::Command(Command::MoveFocus(FocusMove::Index(3)))
        );
    }

    #[test]
    fn tab_cycles_focus_forward() {
        assert_eq!(
            map(KeyEvent::plain(KeyCode::Tab), &snap(None)),
            KeyOutcome::Command(Command::MoveFocus(FocusMove::CycleForward))
        );
    }

    #[test]
    fn backtab_cycles_focus_backward() {
        assert_eq!(
            map(KeyEvent::plain(KeyCode::BackTab), &snap(None)),
            KeyOutcome::Command(Command::MoveFocus(FocusMove::CycleBackward))
        );
    }

    #[test]
    fn unknown_key_is_ignored() {
        assert_eq!(
            map(KeyEvent::plain(KeyCode::Esc), &snap(None)),
            KeyOutcome::Ignored
        );
    }

    #[test]
    fn left_arrow_moves_focus_left() {
        let r = map(KeyEvent::plain(KeyCode::Left), &snap(None));
        assert_eq!(
            r,
            KeyOutcome::Command(Command::MoveFocus(FocusMove::Spatial(
                SpatialDirection::Left
            )))
        );
    }

    #[test]
    fn right_arrow_moves_focus_right() {
        let r = map(KeyEvent::plain(KeyCode::Right), &snap(None));
        assert_eq!(
            r,
            KeyOutcome::Command(Command::MoveFocus(FocusMove::Spatial(
                SpatialDirection::Right
            )))
        );
    }

    #[test]
    fn up_arrow_moves_focus_up() {
        let r = map(KeyEvent::plain(KeyCode::Up), &snap(None));
        assert_eq!(
            r,
            KeyOutcome::Command(Command::MoveFocus(FocusMove::Spatial(SpatialDirection::Up)))
        );
    }

    #[test]
    fn down_arrow_moves_focus_down() {
        let r = map(KeyEvent::plain(KeyCode::Down), &snap(None));
        assert_eq!(
            r,
            KeyOutcome::Command(Command::MoveFocus(FocusMove::Spatial(
                SpatialDirection::Down
            )))
        );
    }
}

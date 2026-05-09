#![cfg_attr(test, allow(clippy::unwrap_used))]

use streeem_domain::event::DomainEvent;
use streeem_domain::grid::FocusMove;
use streeem_domain::outbox::OutboxEffect;
use streeem_domain::reducer::reduce;
use streeem_domain::state::State;
use streeem_domain::tile_id::TileId;

use crate::command::ScrollDelta;

pub fn handle_resize(state: &mut State, id: TileId, delta_rows: i16) -> Vec<OutboxEffect> {
    reduce(state, DomainEvent::TileResized { id, delta_rows })
}

pub fn handle_scroll(state: &mut State, id: TileId, delta: ScrollDelta) -> Vec<OutboxEffect> {
    // The reducer computes: new_offset = current_offset - delta_lines.
    // Scrolling "up" (Lines > 0) increases offset, so delta_lines must be negative.
    // Scrolling "down" to bottom resets offset to 0, so we need a large positive delta_lines.
    let delta_lines = match delta {
        ScrollDelta::Lines(n) => n.saturating_neg(),
        ScrollDelta::Page(n) => n.saturating_mul(20).saturating_neg(),
        ScrollDelta::Top => i32::MIN / 2,
        ScrollDelta::Bottom => i32::MAX / 2,
    };
    reduce(state, DomainEvent::TileScrolled { id, delta_lines })
}

pub fn handle_focus(state: &mut State, m: FocusMove) -> Vec<OutboxEffect> {
    reduce(state, DomainEvent::FocusMoved(m))
}

pub fn handle_follow_tail(state: &mut State, id: TileId) -> Vec<OutboxEffect> {
    reduce(state, DomainEvent::FollowTailToggled(id))
}

pub fn handle_toggle_brave(state: &mut State, id: TileId) -> Vec<OutboxEffect> {
    reduce(state, DomainEvent::BraveModeToggled(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::lifecycle::handle_add_tile;
    use streeem_domain::column_count::ColumnCount;
    use streeem_domain::command_spec::CommandSpec;

    fn fresh() -> State {
        State::new(ColumnCount::new(2).unwrap(), 100, 30)
    }

    #[test]
    fn resize_changes_rows_hint() {
        let mut s = fresh();
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("a").unwrap());
        let id = s.grid.tiles[0].id;
        let _ = handle_resize(&mut s, id, 5);
        assert_eq!(s.grid.tiles[0].rows_hint.value(), 15);
    }

    #[test]
    fn scroll_lines_updates_offset_and_disables_follow_tail() {
        let mut s = fresh();
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("a").unwrap());
        let id = s.grid.tiles[0].id;
        let _ = handle_scroll(&mut s, id, ScrollDelta::Lines(3));
        assert_eq!(s.grid.tiles[0].scroll_offset_from_bottom, 3);
        assert!(!s.grid.tiles[0].follow_tail);
    }

    #[test]
    fn scroll_bottom_re_enables_follow_tail() {
        let mut s = fresh();
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("a").unwrap());
        let id = s.grid.tiles[0].id;
        let _ = handle_scroll(&mut s, id, ScrollDelta::Lines(3));
        let _ = handle_scroll(&mut s, id, ScrollDelta::Bottom);
        assert_eq!(s.grid.tiles[0].scroll_offset_from_bottom, 0);
        assert!(s.grid.tiles[0].follow_tail);
    }

    #[test]
    fn focus_cycle_forward_advances() {
        let mut s = fresh();
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("a").unwrap());
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("b").unwrap());
        let _ = handle_focus(&mut s, FocusMove::CycleForward);
        assert_eq!(s.grid.focused, Some(s.grid.tiles[1].id));
    }

    #[test]
    fn follow_tail_toggle_flips_flag() {
        let mut s = fresh();
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("a").unwrap());
        let id = s.grid.tiles[0].id;
        let before = s.grid.tiles[0].follow_tail;
        let _ = handle_follow_tail(&mut s, id);
        assert_ne!(before, s.grid.tiles[0].follow_tail);
    }

    #[test]
    fn brave_mode_toggle_flips_flag() {
        let mut s = fresh();
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("a").unwrap());
        let id = s.grid.tiles[0].id;
        assert!(!s.grid.tiles[0].brave_mode);
        let _ = handle_toggle_brave(&mut s, id);
        assert!(s.grid.tiles[0].brave_mode);
        let _ = handle_toggle_brave(&mut s, id);
        assert!(!s.grid.tiles[0].brave_mode);
    }
}

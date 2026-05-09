//! Read-only snapshot consumed by the presentation layer.

#![cfg_attr(test, allow(clippy::unwrap_used))]
#![allow(clippy::cast_possible_truncation)]

use streeem_domain::layout_packer::{LayoutInput, Placement, pack};
use streeem_domain::output_line::OutputLine;
use streeem_domain::state::State;
use streeem_domain::tile::RunStatus;
use streeem_domain::tile_color::TileColor;
use streeem_domain::tile_id::TileId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileSnapshot {
    pub id: TileId,
    pub focus_index: u8,
    pub color: TileColor,
    pub title_command: String,
    pub run_status: RunStatus,
    pub follow_tail: bool,
    pub scroll_offset_from_bottom: u32,
    pub lines: Vec<OutputLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertSnapshot {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSnapshot {
    pub terminal_size: (u16, u16),
    pub placements: Vec<Placement>,
    pub tiles: Vec<TileSnapshot>,
    pub focused: Option<TileId>,
    pub alerts: Vec<AlertSnapshot>,
    pub too_small: bool,
}

const MIN_WIDTH: u16 = 40;
const MIN_HEIGHT: u16 = 10;

pub fn snapshot(state: &State) -> RenderSnapshot {
    let too_small =
        state.grid.terminal_width < MIN_WIDTH || state.grid.terminal_height < MIN_HEIGHT;
    let tiles_for_packing: Vec<_> = state
        .grid
        .tiles
        .iter()
        .map(|t| (t.id, t.rows_hint))
        .collect();
    let placements = if too_small || tiles_for_packing.is_empty() {
        Vec::new()
    } else {
        pack(LayoutInput {
            tiles: &tiles_for_packing,
            columns: state.grid.columns,
            terminal_width: state.grid.terminal_width,
            terminal_height: state.grid.terminal_height,
        })
    };
    let tiles = state
        .grid
        .tiles
        .iter()
        .enumerate()
        .map(|(i, t)| TileSnapshot {
            id: t.id,
            focus_index: (i + 1).min(255) as u8,
            color: t.color,
            title_command: t.spec.command.clone(),
            run_status: t.run_status,
            follow_tail: t.follow_tail,
            scroll_offset_from_bottom: t.scroll_offset_from_bottom,
            lines: t.scrollback.iter().cloned().collect(),
        })
        .collect();
    RenderSnapshot {
        terminal_size: (state.grid.terminal_width, state.grid.terminal_height),
        placements,
        tiles,
        focused: state.grid.focused,
        alerts: state
            .alerts
            .iter()
            .map(|a| AlertSnapshot {
                message: a.message.clone(),
            })
            .collect(),
        too_small,
    }
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
    fn empty_state_produces_empty_snapshot_with_no_alerts() {
        let s = fresh();
        let snap = snapshot(&s);
        assert!(snap.tiles.is_empty());
        assert!(snap.placements.is_empty());
        assert!(!snap.too_small);
    }

    #[test]
    fn snapshot_includes_one_tile_per_state_tile() {
        let mut s = fresh();
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("echo a").unwrap());
        let snap = snapshot(&s);
        assert_eq!(snap.tiles.len(), 1);
        assert_eq!(snap.placements.len(), 1);
    }

    #[test]
    fn marks_too_small_when_terminal_below_minimum() {
        let mut s = State::new(ColumnCount::new(1).unwrap(), 30, 5);
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("a").unwrap());
        let snap = snapshot(&s);
        assert!(snap.too_small);
        assert!(snap.placements.is_empty());
    }
}

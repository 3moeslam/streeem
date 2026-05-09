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
    pub display_name: String,
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
    let usable_height = state.grid.terminal_height.saturating_sub(1);
    let configured_cols = state.grid.columns.value();
    let effective_cols_value = configured_cols.min(tiles_for_packing.len() as u16).max(1);
    let effective_columns = streeem_domain::column_count::ColumnCount::new(effective_cols_value)
        .unwrap_or(state.grid.columns);
    let placements = if too_small || tiles_for_packing.is_empty() {
        Vec::new()
    } else {
        pack(LayoutInput {
            tiles: &tiles_for_packing,
            columns: effective_columns,
            terminal_width: state.grid.terminal_width,
            terminal_height: usable_height,
        })
    };
    let tiles = state
        .grid
        .tiles
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let focus_index = (i + 1).min(255) as u8;
            let display_name = t.name.clone().unwrap_or_else(|| format!("{}", focus_index));
            TileSnapshot {
                id: t.id,
                focus_index,
                color: t.color,
                title_command: t.spec.command.clone(),
                display_name,
                run_status: t.run_status,
                follow_tail: t.follow_tail,
                scroll_offset_from_bottom: t.scroll_offset_from_bottom,
                lines: t.scrollback.iter().cloned().collect(),
            }
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

    #[test]
    fn effective_columns_caps_at_tile_count() {
        // 4 columns configured, but only 1 tile — placement should be in column 0 with full width.
        let mut s = State::new(ColumnCount::new(4).unwrap(), 200, 30);
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("a").unwrap());
        let snap = snapshot(&s);
        assert_eq!(snap.placements.len(), 1);
        assert_eq!(snap.placements[0].column, 0);
        assert_eq!(
            snap.placements[0].width, 200,
            "1 tile should get full terminal width"
        );
    }

    #[test]
    fn tile_snapshot_uses_focus_index_when_no_name_given() {
        let mut s = fresh();
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("echo a").unwrap());
        let snap = snapshot(&s);
        assert_eq!(snap.tiles[0].display_name, "1");
    }

    #[test]
    fn tile_snapshot_uses_provided_name_when_set() {
        use streeem_domain::rows_hint::RowsHint;
        let mut s = fresh();
        let spec =
            CommandSpec::new_with_name("echo a", Some("foo".to_string()), RowsHint::default())
                .unwrap();
        let _ = handle_add_tile(&mut s, spec);
        let snap = snapshot(&s);
        assert_eq!(snap.tiles[0].display_name, "foo");
    }
}

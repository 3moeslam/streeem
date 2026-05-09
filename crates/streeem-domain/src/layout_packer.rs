#![cfg_attr(test, allow(clippy::unwrap_used))]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
//! Pure placement of tiles into a staggered (column-flow) grid.

use crate::column_count::ColumnCount;
use crate::rows_hint::RowsHint;
use crate::tile_id::TileId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub tile_id: TileId,
    pub column: u16,
    pub row_offset: u16,
    pub height: u16,
    pub width: u16,
    pub is_clipped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutInput<'a> {
    pub tiles: &'a [(TileId, RowsHint)],
    pub columns: ColumnCount,
    pub terminal_width: u16,
    pub terminal_height: u16,
}

pub fn pack(input: LayoutInput<'_>) -> Vec<Placement> {
    let cols = input.columns.value();
    let width = input.terminal_width / cols.max(1);
    let mut col_heights: Vec<u32> = vec![0; cols as usize];
    let mut placements = Vec::with_capacity(input.tiles.len());
    for &(id, hint) in input.tiles {
        let (col_idx, _) = col_heights
            .iter()
            .enumerate()
            .min_by_key(|(idx, h)| (**h, *idx))
            .map(|(i, h)| (i as u16, *h))
            .unwrap_or((0, 0));
        let row_offset = col_heights[col_idx as usize];
        let height = hint.value();
        let bottom = row_offset.saturating_add(height as u32);
        let is_clipped = bottom > input.terminal_height as u32;
        let visible_height = if is_clipped {
            (input.terminal_height as u32).saturating_sub(row_offset) as u16
        } else {
            height
        };
        placements.push(Placement {
            tile_id: id,
            column: col_idx,
            row_offset: row_offset.try_into().unwrap_or(u16::MAX),
            height: visible_height,
            width,
            is_clipped,
        });
        col_heights[col_idx as usize] = bottom;
    }
    placements
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u32) -> TileId {
        crate::tile_id::TileId::default_from(n)
    }
    fn rh(n: u16) -> RowsHint {
        RowsHint::new(n).unwrap()
    }
    fn cc(n: u16) -> ColumnCount {
        ColumnCount::new(n).unwrap()
    }

    #[test]
    fn single_column_stacks_in_order() {
        let tiles = vec![(id(0), rh(10)), (id(1), rh(8))];
        let placements = pack(LayoutInput {
            tiles: &tiles,
            columns: cc(1),
            terminal_width: 80,
            terminal_height: 100,
        });
        assert_eq!(placements[0].row_offset, 0);
        assert_eq!(placements[1].row_offset, 10);
        assert!(!placements.iter().any(|p| p.is_clipped));
    }

    #[test]
    fn picks_shortest_column_then_lowest_index_on_tie() {
        let tiles = vec![
            (id(0), rh(20)), // -> col 0
            (id(1), rh(8)),  // -> col 1 (tie with col 2; lowest idx)
            (id(2), rh(12)), // -> col 2
            (id(3), rh(5)),  // -> col 1 (height 8) shortest
            (id(4), rh(15)), // -> col 2 (height 12) shortest
        ];
        let placements = pack(LayoutInput {
            tiles: &tiles,
            columns: cc(3),
            terminal_width: 120,
            terminal_height: 60,
        });
        assert_eq!(placements[0].column, 0);
        assert_eq!(placements[1].column, 1);
        assert_eq!(placements[2].column, 2);
        assert_eq!(placements[3].column, 1);
        assert_eq!(placements[3].row_offset, 8);
        assert_eq!(placements[4].column, 2);
        assert_eq!(placements[4].row_offset, 12);
    }

    #[test]
    fn marks_clipped_when_total_exceeds_height() {
        let tiles = vec![(id(0), rh(40)), (id(1), rh(40))];
        let placements = pack(LayoutInput {
            tiles: &tiles,
            columns: cc(1),
            terminal_width: 80,
            terminal_height: 50,
        });
        assert!(!placements[0].is_clipped);
        assert!(placements[1].is_clipped);
        assert_eq!(placements[1].height, 10);
    }

    #[test]
    fn divides_terminal_width_evenly_across_columns() {
        let tiles = vec![(id(0), rh(5)), (id(1), rh(5)), (id(2), rh(5))];
        let placements = pack(LayoutInput {
            tiles: &tiles,
            columns: cc(3),
            terminal_width: 120,
            terminal_height: 30,
        });
        assert!(placements.iter().all(|p| p.width == 40));
    }
}

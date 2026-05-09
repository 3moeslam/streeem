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
        placements.push(Placement {
            tile_id: id,
            column: col_idx,
            row_offset: row_offset.try_into().unwrap_or(u16::MAX),
            height,
            width,
            is_clipped: false,
        });
        col_heights[col_idx as usize] = bottom;
    }

    // Second pass: rescale heights per column to fill the full terminal height.
    for col_idx in 0..cols {
        let col_indices: Vec<usize> = placements
            .iter()
            .enumerate()
            .filter(|(_, p)| p.column == col_idx)
            .map(|(i, _)| i)
            .collect();
        if col_indices.is_empty() {
            continue;
        }
        let total: u32 = col_indices
            .iter()
            .map(|&i| placements[i].height as u32)
            .sum();
        if total == 0 {
            continue;
        }
        let mut acc: u32 = 0;
        let last = col_indices.len() - 1;
        let visible = input.terminal_height as u32;
        for (n, &i) in col_indices.iter().enumerate() {
            placements[i].row_offset = acc.try_into().unwrap_or(u16::MAX);
            let new_height = if n == last {
                visible.saturating_sub(acc)
            } else {
                placements[i].height as u32 * visible / total
            };
            placements[i].height = new_height.try_into().unwrap_or(u16::MAX);
            acc += placements[i].height as u32;
        }
    }

    // is_clipped is always false because we always fill exactly.
    for p in &mut placements {
        p.is_clipped = false;
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
        // tiles [(0, rh=10), (1, rh=8)] in 1 col, terminal 80x100.
        // total=18. tile0: height=10*100/18=55, row_offset=0.
        // tile1 (last): height=100-55=45, row_offset=55.
        let tiles = vec![(id(0), rh(10)), (id(1), rh(8))];
        let placements = pack(LayoutInput {
            tiles: &tiles,
            columns: cc(1),
            terminal_width: 80,
            terminal_height: 100,
        });
        assert_eq!(placements[0].row_offset, 0);
        assert_eq!(placements[0].height, 55);
        assert_eq!(placements[1].row_offset, 55);
        assert_eq!(placements[1].height, 45);
        assert!(!placements.iter().any(|p| p.is_clipped));
    }

    #[test]
    fn picks_shortest_column_then_lowest_index_on_tie() {
        // Tiles: [(0,20), (1,8), (2,12), (3,5), (4,15)], 3 cols, terminal_height=60.
        // col 0: tile 0 alone (rh=20). After scaling: tile0 height=60, row_offset=0.
        // col 1: tile 1 (rh=8) and tile 3 (rh=5), total=13.
        //   tile1 height=8*60/13=36, row_offset=0. tile3 height=60-36=24, row_offset=36.
        // col 2: tile 2 (rh=12) and tile 4 (rh=15), total=27.
        //   tile2 height=12*60/27=26, row_offset=0. tile4 height=60-26=34, row_offset=26.
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
        assert_eq!(placements[0].row_offset, 0);
        assert_eq!(placements[0].height, 60);
        assert_eq!(placements[1].column, 1);
        assert_eq!(placements[1].row_offset, 0);
        assert_eq!(placements[1].height, 36);
        assert_eq!(placements[2].column, 2);
        assert_eq!(placements[2].row_offset, 0);
        assert_eq!(placements[2].height, 26);
        assert_eq!(placements[3].column, 1);
        assert_eq!(placements[3].row_offset, 36);
        assert_eq!(placements[3].height, 24);
        assert_eq!(placements[4].column, 2);
        assert_eq!(placements[4].row_offset, 26);
        assert_eq!(placements[4].height, 34);
    }

    #[test]
    fn fills_column_height_when_summed_hints_undershoot() {
        // 1 tile of rh=10 in 1 col, terminal_height=50 => tile.height=50.
        let tiles = vec![(id(0), rh(10))];
        let placements = pack(LayoutInput {
            tiles: &tiles,
            columns: cc(1),
            terminal_width: 80,
            terminal_height: 50,
        });
        assert_eq!(placements[0].height, 50);
        assert!(!placements[0].is_clipped);
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

    #[test]
    fn single_tile_fills_entire_column_when_hint_is_small() {
        // 1 tile rh=5, 1 col, terminal_height=30 → tile.height=30.
        let tiles = vec![(id(0), rh(5))];
        let placements = pack(LayoutInput {
            tiles: &tiles,
            columns: cc(1),
            terminal_width: 80,
            terminal_height: 30,
        });
        assert_eq!(placements[0].height, 30);
        assert_eq!(placements[0].row_offset, 0);
    }

    #[test]
    fn two_tiles_in_one_column_split_proportionally() {
        // 2 tiles rh=10 and rh=20 in 1 col, terminal_height=60.
        // total=30. tile0: height=10*60/30=20, row_offset=0.
        // tile1 (last): height=60-20=40, row_offset=20.
        let tiles = vec![(id(0), rh(10)), (id(1), rh(20))];
        let placements = pack(LayoutInput {
            tiles: &tiles,
            columns: cc(1),
            terminal_width: 80,
            terminal_height: 60,
        });
        assert_eq!(placements[0].height, 20);
        assert_eq!(placements[0].row_offset, 0);
        assert_eq!(placements[1].height, 40);
        assert_eq!(placements[1].row_offset, 20);
    }
}

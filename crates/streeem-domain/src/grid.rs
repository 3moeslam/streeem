#![cfg_attr(test, allow(clippy::unwrap_used))]
#![allow(clippy::should_implement_trait)]
//! Collection of tiles plus focus and viewport state.

use crate::column_count::ColumnCount;
use crate::tile::Tile;
use crate::tile_id::TileId;

#[derive(Debug, Clone)]
pub struct Grid {
    pub tiles: Vec<Tile>,
    pub focused: Option<TileId>,
    pub columns: ColumnCount,
    pub terminal_width: u16,
    pub terminal_height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusMove {
    CycleForward,
    CycleBackward,
    Index(u8),
}

impl Grid {
    pub fn new(columns: ColumnCount, terminal_width: u16, terminal_height: u16) -> Self {
        Self {
            tiles: Vec::new(),
            focused: None,
            columns,
            terminal_width,
            terminal_height,
        }
    }

    pub fn add(&mut self, tile: Tile) {
        let id = tile.id;
        self.tiles.push(tile);
        if self.focused.is_none() {
            self.focused = Some(id);
        }
    }

    pub fn drop(&mut self, id: TileId) {
        let pos = match self.tiles.iter().position(|t| t.id == id) {
            Some(p) => p,
            None => return,
        };
        self.tiles.remove(pos);
        if self.focused == Some(id) {
            self.focused = self
                .tiles
                .get(pos)
                .or_else(|| self.tiles.last())
                .map(|t| t.id);
        }
    }

    pub fn move_focus(&mut self, m: FocusMove) {
        if self.tiles.is_empty() {
            self.focused = None;
            return;
        }
        let current = self
            .focused
            .and_then(|id| self.tiles.iter().position(|t| t.id == id))
            .unwrap_or(0);
        let new_index = match m {
            FocusMove::CycleForward => (current + 1) % self.tiles.len(),
            FocusMove::CycleBackward => (current + self.tiles.len() - 1) % self.tiles.len(),
            FocusMove::Index(n) => {
                let n = n.saturating_sub(1) as usize;
                n.min(self.tiles.len() - 1)
            }
        };
        self.focused = Some(self.tiles[new_index].id);
    }

    pub fn focused_tile(&self) -> Option<&Tile> {
        self.focused
            .and_then(|id| self.tiles.iter().find(|t| t.id == id))
    }

    pub fn focused_tile_mut(&mut self) -> Option<&mut Tile> {
        let id = self.focused?;
        self.tiles.iter_mut().find(|t| t.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_spec::CommandSpec;
    use crate::scrollback_capacity::ScrollbackCapacity;
    use crate::tile_color::TileColor;

    fn make_tile(id: u32, color: TileColor) -> Tile {
        Tile::new(
            TileId::default_from(id),
            color,
            CommandSpec::with_default_rows("echo").unwrap(),
            ScrollbackCapacity::default(),
        )
    }

    fn empty_grid() -> Grid {
        Grid::new(ColumnCount::new(2).unwrap(), 80, 30)
    }

    #[test]
    fn empty_grid_has_no_focus() {
        assert!(empty_grid().focused.is_none());
    }

    #[test]
    fn first_added_tile_becomes_focused() {
        let mut g = empty_grid();
        g.add(make_tile(0, TileColor::Red));
        assert_eq!(g.focused, Some(TileId::default_from(0)));
    }

    #[test]
    fn drop_focused_falls_back_to_neighbour() {
        let mut g = empty_grid();
        g.add(make_tile(0, TileColor::Red));
        g.add(make_tile(1, TileColor::Green));
        g.add(make_tile(2, TileColor::Blue));
        g.move_focus(FocusMove::Index(2));
        g.drop(TileId::default_from(1));
        assert_eq!(g.focused, Some(TileId::default_from(2)));
    }

    #[test]
    fn drop_last_tile_clears_focus() {
        let mut g = empty_grid();
        g.add(make_tile(0, TileColor::Red));
        g.drop(TileId::default_from(0));
        assert!(g.focused.is_none());
    }

    #[test]
    fn cycle_forward_wraps() {
        let mut g = empty_grid();
        g.add(make_tile(0, TileColor::Red));
        g.add(make_tile(1, TileColor::Green));
        g.move_focus(FocusMove::CycleForward);
        assert_eq!(g.focused, Some(TileId::default_from(1)));
        g.move_focus(FocusMove::CycleForward);
        assert_eq!(g.focused, Some(TileId::default_from(0)));
    }

    #[test]
    fn index_clamps_to_last() {
        let mut g = empty_grid();
        g.add(make_tile(0, TileColor::Red));
        g.add(make_tile(1, TileColor::Green));
        g.move_focus(FocusMove::Index(9));
        assert_eq!(g.focused, Some(TileId::default_from(1)));
    }
}

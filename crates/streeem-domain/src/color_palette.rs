//! Assigns colours from PALETTE deterministically.
//!
//! Rule (per spec §7.2): scan PALETTE in order and return the first colour
//! not currently in use. When all 12 are in use, the next request reuses
//! the colour at PALETTE[0] (deterministic wrap; two tiles may share).

use crate::tile_color::{PALETTE, TileColor};

#[derive(Debug, Clone, Default)]
pub struct ColorPalette {
    in_use: Vec<TileColor>,
}

impl ColorPalette {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn assign(&mut self) -> TileColor {
        for &c in PALETTE.iter() {
            if !self.in_use.contains(&c) {
                self.in_use.push(c);
                return c;
            }
        }
        let wrapped = PALETTE[0];
        self.in_use.push(wrapped);
        wrapped
    }

    pub fn release(&mut self, color: TileColor) {
        if let Some(pos) = self.in_use.iter().position(|&c| c == color) {
            self.in_use.swap_remove(pos);
        }
    }

    pub fn in_use_count(&self) -> usize {
        self.in_use.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_assignment_is_red() {
        let mut p = ColorPalette::new();
        assert_eq!(p.assign(), TileColor::Red);
    }

    #[test]
    fn assignments_are_distinct_until_palette_exhausted() {
        let mut p = ColorPalette::new();
        let mut seen: Vec<TileColor> = (0..PALETTE.len()).map(|_| p.assign()).collect();
        seen.sort_by_key(|c| format!("{c:?}"));
        let mut expected = PALETTE.to_vec();
        expected.sort_by_key(|c| format!("{c:?}"));
        assert_eq!(seen, expected);
    }

    #[test]
    fn release_returns_color_to_pool() {
        let mut p = ColorPalette::new();
        let first = p.assign(); // Red
        let second = p.assign(); // Green
        p.release(first);
        assert_eq!(
            p.assign(),
            TileColor::Red,
            "released colour reassigned first"
        );
        let _ = second;
    }

    #[test]
    fn release_of_unassigned_color_is_noop() {
        let mut p = ColorPalette::new();
        p.release(TileColor::Magenta);
        assert_eq!(p.in_use_count(), 0);
    }

    #[test]
    fn wraps_to_first_palette_entry_when_exhausted() {
        let mut p = ColorPalette::new();
        for _ in 0..PALETTE.len() {
            p.assign();
        }
        assert_eq!(p.assign(), TileColor::Red);
        assert_eq!(p.in_use_count(), PALETTE.len() + 1);
    }
}

//! The fixed palette of tile identification colours.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileColor {
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
}

pub const PALETTE: [TileColor; 12] = [
    TileColor::Red,
    TileColor::Green,
    TileColor::Yellow,
    TileColor::Blue,
    TileColor::Magenta,
    TileColor::Cyan,
    TileColor::LightRed,
    TileColor::LightGreen,
    TileColor::LightYellow,
    TileColor::LightBlue,
    TileColor::LightMagenta,
    TileColor::LightCyan,
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn palette_has_twelve_entries() {
        assert_eq!(PALETTE.len(), 12);
    }

    #[test]
    fn palette_entries_are_unique() {
        let set: HashSet<_> = PALETTE.iter().collect();
        assert_eq!(set.len(), PALETTE.len());
    }

    #[test]
    fn palette_first_entry_is_red_by_convention() {
        assert_eq!(PALETTE[0], TileColor::Red);
    }
}

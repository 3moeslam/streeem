//! Foreground/background colour and font weight for a styled span of text.

use crate::tile_color::TileColor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Style {
    pub fg: Option<TileColor>,
    pub bg: Option<TileColor>,
    pub bold: bool,
    pub underline: bool,
}

impl Style {
    pub const RESET: Self = Self {
        fg: None,
        bg: None,
        bold: false,
        underline: false,
    };

    pub fn with_fg(mut self, fg: TileColor) -> Self {
        self.fg = Some(fg);
        self
    }

    pub fn with_bg(mut self, bg: TileColor) -> Self {
        self.bg = Some(bg);
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_no_colours_no_decoration() {
        let s = Style::default();
        assert!(s.fg.is_none() && s.bg.is_none() && !s.bold && !s.underline);
    }

    #[test]
    fn reset_equals_default() {
        assert_eq!(Style::RESET, Style::default());
    }

    #[test]
    fn builders_chain() {
        let s = Style::default().with_fg(TileColor::Red).bold().underline();
        assert_eq!(s.fg, Some(TileColor::Red));
        assert!(s.bold && s.underline);
    }
}

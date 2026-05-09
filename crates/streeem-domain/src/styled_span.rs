//! A run of text with a single style.

use crate::style::Style;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StyledSpan {
    pub text: String,
    pub style: Style,
}

impl StyledSpan {
    pub fn new(text: impl Into<String>, style: Style) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Self::new(text, Style::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile_color::TileColor;

    #[test]
    fn plain_uses_default_style() {
        let s = StyledSpan::plain("hi");
        assert_eq!(s.style, Style::default());
        assert_eq!(s.text, "hi");
    }

    #[test]
    fn new_keeps_supplied_style() {
        let style = Style::default().with_fg(TileColor::Green);
        let s = StyledSpan::new("ok", style);
        assert_eq!(s.style, style);
    }
}

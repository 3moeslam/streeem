#![cfg_attr(test, allow(clippy::panic))]
//! One logical line of output composed of one or more styled spans, plus optional markers.

use crate::styled_span::StyledSpan;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OutputLine {
    Text(Vec<StyledSpan>),
    LinesDropped(usize),
}

impl OutputLine {
    pub fn from_text(spans: Vec<StyledSpan>) -> Self {
        Self::Text(spans)
    }

    pub fn plain_text(text: impl Into<String>) -> Self {
        Self::Text(vec![StyledSpan::plain(text)])
    }

    pub fn dropped(count: usize) -> Self {
        Self::LinesDropped(count)
    }

    pub fn is_marker(&self) -> bool {
        matches!(self, Self::LinesDropped(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_wraps_a_single_span() {
        let line = OutputLine::plain_text("hello");
        match line {
            OutputLine::Text(spans) => {
                assert_eq!(spans.len(), 1);
                assert_eq!(spans[0].text, "hello");
            }
            _ => panic!("expected Text variant"),
        }
    }

    #[test]
    fn dropped_marker_is_recognised() {
        assert!(OutputLine::dropped(7).is_marker());
        assert!(!OutputLine::plain_text("x").is_marker());
    }
}

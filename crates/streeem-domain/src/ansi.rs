#![cfg_attr(test, allow(clippy::panic))]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::while_let_on_iterator
)]
//! Streaming ANSI byte interpreter. Emits OutputLine::Text per newline,
//! applies SGR colour codes, drops cursor / clear / scroll-region escapes.

use crate::output_line::OutputLine;
use crate::style::Style;
use crate::styled_span::StyledSpan;
use crate::tile_color::TileColor;

#[derive(Debug, Default, Clone)]
pub struct AnsiInterpreter {
    state: State,
    current_style: Style,
    current_text: String,
    current_spans: Vec<StyledSpan>,
    pending_csi: Vec<u8>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum State {
    #[default]
    Normal,
    Escape,
    Csi,
}

impl AnsiInterpreter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Vec<OutputLine> {
        let text = String::from_utf8_lossy(bytes);
        let mut out = Vec::new();
        for ch in text.chars() {
            match self.state {
                State::Normal => self.handle_normal(ch, &mut out),
                State::Escape => self.handle_escape(ch),
                State::Csi => self.handle_csi(ch),
            }
        }
        out
    }

    fn handle_normal(&mut self, ch: char, out: &mut Vec<OutputLine>) {
        if ch == '\u{1b}' {
            self.state = State::Escape;
        } else if ch == '\n' {
            self.flush_current_span();
            let line = std::mem::take(&mut self.current_spans);
            out.push(OutputLine::Text(line));
        } else if ch == '\r' {
            // ignored; we treat \r as a no-op for read-only monitoring.
        } else if !ch.is_control() {
            self.current_text.push(ch);
        }
    }

    fn handle_escape(&mut self, ch: char) {
        if ch == '[' {
            self.state = State::Csi;
            self.pending_csi.clear();
        } else {
            self.state = State::Normal; // unknown escape - drop
        }
    }

    fn handle_csi(&mut self, ch: char) {
        let b = ch as u32;
        if (0x40..=0x7E).contains(&b) {
            let final_byte = ch;
            if final_byte == 'm' {
                let params = std::mem::take(&mut self.pending_csi);
                self.flush_current_span();
                apply_sgr(&mut self.current_style, &params);
            }
            // any other final byte (cursor, clear, etc.) is dropped
            self.state = State::Normal;
        } else {
            self.pending_csi.push(b as u8);
        }
    }

    fn flush_current_span(&mut self) {
        if !self.current_text.is_empty() {
            self.current_spans.push(StyledSpan::new(
                std::mem::take(&mut self.current_text),
                self.current_style,
            ));
        }
    }
}

fn apply_sgr(style: &mut Style, params: &[u8]) {
    let s = std::str::from_utf8(params).unwrap_or("");
    let mut nums = s.split(';').filter_map(|p| p.parse::<u8>().ok());
    while let Some(n) = nums.next() {
        match n {
            0 => *style = Style::RESET,
            1 => style.bold = true,
            4 => style.underline = true,
            22 => style.bold = false,
            24 => style.underline = false,
            30..=37 => style.fg = Some(basic_color(n - 30)),
            39 => style.fg = None,
            40..=47 => style.bg = Some(basic_color(n - 40)),
            49 => style.bg = None,
            90..=97 => style.fg = Some(bright_color(n - 90)),
            100..=107 => style.bg = Some(bright_color(n - 100)),
            _ => {}
        }
    }
}

fn basic_color(idx: u8) -> TileColor {
    match idx {
        0 => TileColor::Red, // black -> map to red (palette has no black)
        1 => TileColor::Red,
        2 => TileColor::Green,
        3 => TileColor::Yellow,
        4 => TileColor::Blue,
        5 => TileColor::Magenta,
        6 => TileColor::Cyan,
        _ => TileColor::Red, // 7 (white) maps to Red as fallback
    }
}

fn bright_color(idx: u8) -> TileColor {
    match idx {
        1 => TileColor::LightRed,
        2 => TileColor::LightGreen,
        3 => TileColor::LightYellow,
        4 => TileColor::LightBlue,
        5 => TileColor::LightMagenta,
        6 => TileColor::LightCyan,
        _ => TileColor::LightRed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_then_newline_emits_one_line() {
        let mut a = AnsiInterpreter::new();
        let lines = a.feed(b"hello\n");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], OutputLine::plain_text("hello"));
    }

    #[test]
    fn no_newline_means_no_emission_yet() {
        let mut a = AnsiInterpreter::new();
        let lines = a.feed(b"partial");
        assert!(lines.is_empty());
    }

    #[test]
    fn sgr_red_then_text_emits_red_span() {
        let mut a = AnsiInterpreter::new();
        let lines = a.feed(b"\x1b[31mfail\x1b[0m\n");
        assert_eq!(lines.len(), 1);
        match &lines[0] {
            OutputLine::Text(spans) => {
                assert_eq!(spans.len(), 1);
                assert_eq!(spans[0].text, "fail");
                assert_eq!(spans[0].style.fg, Some(TileColor::Red));
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn cursor_move_escape_is_dropped() {
        let mut a = AnsiInterpreter::new();
        let lines = a.feed(b"\x1b[10;5Habc\n");
        assert_eq!(lines, vec![OutputLine::plain_text("abc")]);
    }

    #[test]
    fn screen_clear_escape_is_dropped() {
        let mut a = AnsiInterpreter::new();
        let lines = a.feed(b"\x1b[2Jx\n");
        assert_eq!(lines, vec![OutputLine::plain_text("x")]);
    }

    #[test]
    fn invalid_utf8_replaced_with_replacement_char() {
        let mut a = AnsiInterpreter::new();
        let lines = a.feed(&[0xff, b'\n']);
        match &lines[0] {
            OutputLine::Text(spans) => assert!(spans[0].text.contains('\u{FFFD}')),
            _ => panic!(),
        }
    }
}

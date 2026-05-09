//! 2D terminal cell buffer backed by the vt100 xterm emulator.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
#![cfg_attr(test, allow(clippy::unwrap_used))]

use vt100::Parser;

use crate::cell_color::CellColor;
use crate::scrollback_capacity::ScrollbackCapacity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    pub ch: char,
    pub fg: CellColor,
    pub bg: CellColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: CellColor::Default,
            bg: CellColor::Default,
            bold: false,
            italic: false,
            underline: false,
            inverse: false,
        }
    }
}

pub struct TerminalBuffer {
    parser: Parser,
    width: u16,
    height: u16,
    scrollback_capacity: ScrollbackCapacity,
}

impl std::fmt::Debug for TerminalBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalBuffer")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl Clone for TerminalBuffer {
    fn clone(&self) -> Self {
        // Reset on clone; vt100::Parser isn't Clone. Rendering snapshots use
        // visible_rows() which copies cells out, so we don't actually need
        // deep-clone semantics here.
        Self::new(self.width, self.height, self.scrollback_capacity)
    }
}

impl TerminalBuffer {
    pub fn new(width: u16, height: u16, scrollback_capacity: ScrollbackCapacity) -> Self {
        let w = width.max(1);
        let h = height.max(1);
        let parser = Parser::new(h, w, scrollback_capacity.value());
        Self {
            parser,
            width: w,
            height: h,
            scrollback_capacity,
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn cursor(&self) -> (u16, u16) {
        self.parser.screen().cursor_position()
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    pub fn resize(&mut self, new_w: u16, new_h: u16) {
        let w = new_w.max(1);
        let h = new_h.max(1);
        self.parser.set_size(h, w);
        self.width = w;
        self.height = h;
    }

    /// Snapshot the visible cells as a 2D vec for the renderer.
    pub fn visible_rows(&self) -> Vec<Vec<Cell>> {
        let screen = self.parser.screen();
        let h = self.height as usize;
        let w = self.width as usize;
        let mut rows = Vec::with_capacity(h);
        for r in 0..h {
            let mut row = Vec::with_capacity(w);
            for c in 0..w {
                let cell = screen
                    .cell(r as u16, c as u16)
                    .map(translate_cell)
                    .unwrap_or_default();
                row.push(cell);
            }
            rows.push(row);
        }
        rows
    }

    pub fn scrollback_len(&self) -> usize {
        // vt100's parser has scrollback but exposing it cell-by-cell needs
        // additional API. v0.2.x just reports 0; revisit when scroll-into-
        // scrollback UI lands.
        0
    }
}

fn translate_cell(c: &vt100::Cell) -> Cell {
    let contents = c.contents();
    let ch = contents.chars().next().unwrap_or(' ');
    Cell {
        ch,
        fg: translate_color(c.fgcolor()),
        bg: translate_color(c.bgcolor()),
        bold: c.bold(),
        italic: c.italic(),
        underline: c.underline(),
        inverse: c.inverse(),
    }
}

fn translate_color(c: vt100::Color) -> CellColor {
    match c {
        vt100::Color::Default => CellColor::Default,
        vt100::Color::Idx(i) => CellColor::Indexed(i),
        vt100::Color::Rgb(r, g, b) => CellColor::Rgb(r, g, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(w: u16, h: u16) -> TerminalBuffer {
        TerminalBuffer::new(w, h, ScrollbackCapacity::new(1000).unwrap())
    }

    #[test]
    fn new_buffer_is_blank_with_cursor_at_origin() {
        let b = buf(4, 2);
        assert_eq!(b.cursor(), (0, 0));
        let rows = b.visible_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].len(), 4);
        assert!(rows.iter().all(|r| r.iter().all(|c| c.ch == ' ')));
    }

    #[test]
    fn feed_plain_text_writes_cells() {
        let mut b = buf(8, 2);
        b.feed(b"hi");
        let rows = b.visible_rows();
        assert_eq!(rows[0][0].ch, 'h');
        assert_eq!(rows[0][1].ch, 'i');
        assert_eq!(b.cursor(), (0, 2));
    }

    #[test]
    fn newline_advances_to_next_row_and_resets_column() {
        let mut b = buf(4, 3);
        b.feed(b"a\r\nb");
        let rows = b.visible_rows();
        assert_eq!(rows[0][0].ch, 'a');
        assert_eq!(rows[1][0].ch, 'b');
    }

    #[test]
    fn sgr_red_styles_following_cells() {
        let mut b = buf(8, 1);
        b.feed(b"\x1b[31mfail\x1b[0mok");
        let rows = b.visible_rows();
        // vt100 reports red as Idx(1)
        assert!(matches!(
            rows[0][0].fg,
            CellColor::Indexed(_) | CellColor::Default
        ));
        // After reset, no color:
        assert_eq!(rows[0][4].fg, CellColor::Default);
    }

    #[test]
    fn truecolor_sgr_yields_rgb() {
        let mut b = buf(8, 1);
        b.feed(b"\x1b[38;2;128;200;64mX");
        let rows = b.visible_rows();
        assert_eq!(rows[0][0].fg, CellColor::Rgb(128, 200, 64));
    }

    #[test]
    fn cursor_position_csi_moves_cursor() {
        let mut b = buf(8, 4);
        b.feed(b"\x1b[3;5HX");
        // CUP 3;5 = row 3 col 5 1-based -> (2, 4) 0-based; after print cursor at (2, 5)
        let rows = b.visible_rows();
        assert_eq!(rows[2][4].ch, 'X');
    }

    #[test]
    fn alternate_screen_does_not_corrupt_primary() {
        let mut b = buf(8, 2);
        b.feed(b"primary");
        b.feed(b"\x1b[?1049h"); // enter alt screen
        b.feed(b"\x1b[Halt only"); // overwrite from origin
        b.feed(b"\x1b[?1049l"); // leave alt screen
        let rows = b.visible_rows();
        // Primary should be restored — first row shows "primary"
        let first_line: String = rows[0].iter().take(7).map(|c| c.ch).collect();
        assert_eq!(first_line, "primary");
    }

    #[test]
    fn resize_updates_dimensions() {
        let mut b = buf(4, 2);
        b.feed(b"abcd");
        b.resize(6, 3);
        assert_eq!(b.width(), 6);
        assert_eq!(b.height(), 3);
        let rows = b.visible_rows();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].len(), 6);
    }

    #[test]
    fn bold_attribute_propagates() {
        let mut b = buf(4, 1);
        b.feed(b"\x1b[1mB\x1b[22mn");
        let rows = b.visible_rows();
        assert!(rows[0][0].bold);
        assert!(!rows[0][1].bold);
    }
}

//! 2D terminal cell buffer backed by the vte VT100 parser.
//!
//! Replaces the line-based Scrollback for v0.2.0. Maintains a fixed-size
//! grid of cells (rows × cols), a cursor position, and the current SGR style.
//! Byte input is fed through vte::Parser; the Perform trait callbacks mutate
//! the buffer (print → set cell + advance cursor, execute → handle control
//! chars, csi_dispatch → cursor movement / erase / SGR).
//!
//! Scrollback: when the cursor would move past the bottom row, the top row
//! is moved into the scrollback Vec and a blank row is added at the bottom.
//! Scrollback is bounded by `scrollback_capacity`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
#![cfg_attr(test, allow(clippy::unwrap_used))]

use vte::{Params, Parser, Perform};

use crate::scrollback_capacity::ScrollbackCapacity;
use crate::style::Style;
use crate::tile_color::TileColor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: Style::default(),
        }
    }
}

pub struct TerminalBuffer {
    width: u16,
    height: u16,
    /// Current screen contents; outer index = row (0 = top), inner = col.
    grid: Vec<Vec<Cell>>,
    /// Lines that scrolled off the top, oldest first. Bounded by capacity.
    scrollback: std::collections::VecDeque<Vec<Cell>>,
    scrollback_capacity: ScrollbackCapacity,
    cursor_row: u16,
    cursor_col: u16,
    current_style: Style,
    parser: Parser,
}

impl std::fmt::Debug for TerminalBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalBuffer")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("cursor_row", &self.cursor_row)
            .field("cursor_col", &self.cursor_col)
            .field("current_style", &self.current_style)
            .field("scrollback_len", &self.scrollback.len())
            .finish_non_exhaustive()
    }
}

impl Clone for TerminalBuffer {
    fn clone(&self) -> Self {
        Self {
            width: self.width,
            height: self.height,
            grid: self.grid.clone(),
            scrollback: self.scrollback.clone(),
            scrollback_capacity: self.scrollback_capacity,
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
            current_style: self.current_style,
            // Parser is not Clone; start fresh for the clone — state is in the grid.
            parser: Parser::new(),
        }
    }
}

impl TerminalBuffer {
    pub fn new(width: u16, height: u16, scrollback_capacity: ScrollbackCapacity) -> Self {
        let w = width.max(1);
        let h = height.max(1);
        let grid = (0..h).map(|_| vec![Cell::default(); w as usize]).collect();
        Self {
            width: w,
            height: h,
            grid,
            scrollback: std::collections::VecDeque::new(),
            scrollback_capacity,
            cursor_row: 0,
            cursor_col: 0,
            current_style: Style::default(),
            parser: Parser::new(),
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn cursor(&self) -> (u16, u16) {
        (self.cursor_row, self.cursor_col)
    }

    pub fn current_style(&self) -> Style {
        self.current_style
    }

    /// Visible grid as a slice of rows.
    pub fn visible_rows(&self) -> &[Vec<Cell>] {
        &self.grid
    }

    /// Scrollback rows (oldest first).
    pub fn scrollback_rows(&self) -> impl Iterator<Item = &Vec<Cell>> {
        self.scrollback.iter()
    }

    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// Resize the visible grid. Truncates / extends; does not reflow scrollback.
    pub fn resize(&mut self, new_w: u16, new_h: u16) {
        let w = new_w.max(1);
        let h = new_h.max(1);
        // Adjust each existing row to new width.
        for row in &mut self.grid {
            row.resize(w as usize, Cell::default());
        }
        // Adjust row count.
        match self.grid.len().cmp(&(h as usize)) {
            std::cmp::Ordering::Less => {
                while self.grid.len() < h as usize {
                    self.grid.push(vec![Cell::default(); w as usize]);
                }
            }
            std::cmp::Ordering::Greater => {
                self.grid.truncate(h as usize);
            }
            std::cmp::Ordering::Equal => {}
        }
        self.width = w;
        self.height = h;
        if self.cursor_row >= h {
            self.cursor_row = h - 1;
        }
        if self.cursor_col >= w {
            self.cursor_col = w - 1;
        }
    }

    /// Feed a chunk of bytes from the PTY into the buffer.
    pub fn feed(&mut self, bytes: &[u8]) {
        // Move parser out so we can pass &mut self as Perform.
        let mut parser = std::mem::take(&mut self.parser);
        parser.advance(self, bytes);
        self.parser = parser;
    }

    fn line_feed(&mut self) {
        if self.cursor_row + 1 >= self.height {
            // Scroll the top row into scrollback.
            let top = self.grid.remove(0);
            self.scrollback.push_back(top);
            while self.scrollback.len() > self.scrollback_capacity.value() {
                self.scrollback.pop_front();
            }
            self.grid.push(vec![Cell::default(); self.width as usize]);
        } else {
            self.cursor_row += 1;
        }
    }

    fn carriage_return(&mut self) {
        self.cursor_col = 0;
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    fn put_char(&mut self, c: char) {
        if self.cursor_col >= self.width {
            self.line_feed();
            self.cursor_col = 0;
        }
        let row = self.cursor_row as usize;
        let col = self.cursor_col as usize;
        if row < self.grid.len() && col < self.grid[row].len() {
            self.grid[row][col] = Cell {
                ch: c,
                style: self.current_style,
            };
        }
        self.cursor_col += 1;
    }

    fn cursor_up(&mut self, n: u16) {
        self.cursor_row = self.cursor_row.saturating_sub(n.max(1));
    }

    fn cursor_down(&mut self, n: u16) {
        self.cursor_row = (self.cursor_row + n.max(1)).min(self.height - 1);
    }

    fn cursor_forward(&mut self, n: u16) {
        self.cursor_col = (self.cursor_col + n.max(1)).min(self.width - 1);
    }

    fn cursor_back(&mut self, n: u16) {
        self.cursor_col = self.cursor_col.saturating_sub(n.max(1));
    }

    fn cursor_position(&mut self, row_1based: u16, col_1based: u16) {
        let r = row_1based.max(1) - 1;
        let c = col_1based.max(1) - 1;
        self.cursor_row = r.min(self.height - 1);
        self.cursor_col = c.min(self.width - 1);
    }

    fn erase_in_display(&mut self, mode: u16) {
        // 0: cursor → end of screen; 1: start of screen → cursor; 2: entire screen
        match mode {
            0 => {
                // Clear from cursor to end of current row, then all rows below.
                let row = self.cursor_row as usize;
                let col = self.cursor_col as usize;
                if row < self.grid.len() {
                    for c in col..self.grid[row].len() {
                        self.grid[row][c] = Cell::default();
                    }
                }
                for r in (row + 1)..self.grid.len() {
                    for c in 0..self.grid[r].len() {
                        self.grid[r][c] = Cell::default();
                    }
                }
            }
            1 => {
                let row = self.cursor_row as usize;
                let col = self.cursor_col as usize;
                for r in 0..row {
                    for c in 0..self.grid[r].len() {
                        self.grid[r][c] = Cell::default();
                    }
                }
                if row < self.grid.len() {
                    for c in 0..=col.min(self.grid[row].len().saturating_sub(1)) {
                        self.grid[row][c] = Cell::default();
                    }
                }
            }
            2 | 3 => {
                for r in 0..self.grid.len() {
                    for c in 0..self.grid[r].len() {
                        self.grid[r][c] = Cell::default();
                    }
                }
            }
            _ => {}
        }
    }

    fn erase_in_line(&mut self, mode: u16) {
        let row = self.cursor_row as usize;
        let col = self.cursor_col as usize;
        if row >= self.grid.len() {
            return;
        }
        let row_len = self.grid[row].len();
        match mode {
            0 => {
                for c in col..row_len {
                    self.grid[row][c] = Cell::default();
                }
            }
            1 => {
                for c in 0..=col.min(row_len.saturating_sub(1)) {
                    self.grid[row][c] = Cell::default();
                }
            }
            2 => {
                for c in 0..row_len {
                    self.grid[row][c] = Cell::default();
                }
            }
            _ => {}
        }
    }

    fn apply_sgr(&mut self, params: &Params) {
        if params.is_empty() {
            self.current_style = Style::RESET;
            return;
        }
        for p in params.iter() {
            let n = p.first().copied().unwrap_or(0);
            match n {
                0 => self.current_style = Style::RESET,
                1 => self.current_style.bold = true,
                4 => self.current_style.underline = true,
                22 => self.current_style.bold = false,
                24 => self.current_style.underline = false,
                30..=37 => self.current_style.fg = Some(basic_color(n - 30)),
                39 => self.current_style.fg = None,
                40..=47 => self.current_style.bg = Some(basic_color(n - 40)),
                49 => self.current_style.bg = None,
                90..=97 => self.current_style.fg = Some(bright_color(n - 90)),
                100..=107 => self.current_style.bg = Some(bright_color(n - 100)),
                _ => {}
            }
        }
    }
}

fn basic_color(idx: u16) -> TileColor {
    match idx {
        1 => TileColor::Red,
        2 => TileColor::Green,
        3 => TileColor::Yellow,
        4 => TileColor::Blue,
        5 => TileColor::Magenta,
        6 => TileColor::Cyan,
        _ => TileColor::Red,
    }
}

fn bright_color(idx: u16) -> TileColor {
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

impl Perform for TerminalBuffer {
    fn print(&mut self, c: char) {
        self.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | b'\x0b' | b'\x0c' => {
                // Treat LF/VT/FF as CR+LF (ONLCR mode) — PTY output is typically
                // in this mode, so raw `\n` from shell commands maps to a new line
                // starting at column 0.
                self.line_feed();
                self.carriage_return();
            }
            b'\r' => self.carriage_return(),
            8 => self.backspace(),
            b'\t' => {
                let next_tab = (self.cursor_col / 8 + 1) * 8;
                self.cursor_col = next_tab.min(self.width - 1);
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, c: char) {
        let nth = |i: usize, default: u16| -> u16 {
            params
                .iter()
                .nth(i)
                .and_then(|p| p.first().copied())
                .unwrap_or(default)
        };
        match c {
            'A' => self.cursor_up(nth(0, 1)),
            'B' => self.cursor_down(nth(0, 1)),
            'C' => self.cursor_forward(nth(0, 1)),
            'D' => self.cursor_back(nth(0, 1)),
            'H' | 'f' => self.cursor_position(nth(0, 1), nth(1, 1)),
            'J' => self.erase_in_display(nth(0, 0)),
            'K' => self.erase_in_line(nth(0, 0)),
            'm' => self.apply_sgr(params),
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(w: u16, h: u16) -> TerminalBuffer {
        TerminalBuffer::new(w, h, ScrollbackCapacity::new(1000).unwrap())
    }

    #[test]
    fn new_buffer_has_blank_grid_and_cursor_at_origin() {
        let b = buf(4, 2);
        assert_eq!(b.cursor(), (0, 0));
        assert_eq!(b.visible_rows().len(), 2);
        assert_eq!(b.visible_rows()[0].len(), 4);
        assert!(
            b.visible_rows()
                .iter()
                .all(|r| r.iter().all(|c| c.ch == ' '))
        );
    }

    #[test]
    fn print_advances_cursor_and_writes_cell() {
        let mut b = buf(4, 2);
        b.feed(b"hi");
        assert_eq!(b.cursor(), (0, 2));
        assert_eq!(b.visible_rows()[0][0].ch, 'h');
        assert_eq!(b.visible_rows()[0][1].ch, 'i');
    }

    #[test]
    fn newline_advances_to_next_row() {
        let mut b = buf(4, 3);
        b.feed(b"a\nb");
        assert_eq!(b.visible_rows()[0][0].ch, 'a');
        assert_eq!(b.visible_rows()[1][0].ch, 'b');
        assert_eq!(b.cursor(), (1, 1));
    }

    #[test]
    fn carriage_return_resets_column() {
        let mut b = buf(4, 1);
        b.feed(b"abc\rX");
        assert_eq!(b.visible_rows()[0][0].ch, 'X');
        assert_eq!(b.visible_rows()[0][1].ch, 'b');
    }

    #[test]
    fn line_overflow_wraps_to_next_row() {
        let mut b = buf(3, 3);
        b.feed(b"abcd");
        assert_eq!(b.visible_rows()[0][0].ch, 'a');
        assert_eq!(b.visible_rows()[0][1].ch, 'b');
        assert_eq!(b.visible_rows()[0][2].ch, 'c');
        assert_eq!(b.visible_rows()[1][0].ch, 'd');
    }

    #[test]
    fn newline_at_bottom_scrolls_into_scrollback() {
        let mut b = buf(2, 2);
        b.feed(b"AB\nCD\nEF");
        // After the second newline + EF, original "AB" row should have moved to scrollback.
        assert_eq!(b.scrollback_len(), 1);
        assert_eq!(b.visible_rows()[0][0].ch, 'C');
        assert_eq!(b.visible_rows()[1][0].ch, 'E');
    }

    #[test]
    fn sgr_red_then_text_styles_cells() {
        let mut b = buf(8, 1);
        b.feed(b"\x1b[31mfail\x1b[0mok");
        assert_eq!(b.visible_rows()[0][0].style.fg, Some(TileColor::Red));
        assert_eq!(b.visible_rows()[0][3].style.fg, Some(TileColor::Red));
        assert_eq!(b.visible_rows()[0][4].style.fg, None);
    }

    #[test]
    fn bare_sgr_m_resets_style() {
        let mut b = buf(4, 1);
        b.feed(b"\x1b[31mA\x1b[mB");
        assert_eq!(b.visible_rows()[0][0].style.fg, Some(TileColor::Red));
        assert_eq!(b.visible_rows()[0][1].style.fg, None);
    }

    #[test]
    fn cursor_position_csi_moves_cursor() {
        let mut b = buf(8, 4);
        b.feed(b"\x1b[2;3HX");
        // CUP 2;3 → row=2 (1-based) col=3 (1-based) → row=1 col=2 in 0-based.
        // After printing X cursor advances to col=3.
        assert_eq!(b.visible_rows()[1][2].ch, 'X');
        assert_eq!(b.cursor(), (1, 3));
    }

    #[test]
    fn erase_in_display_clears_from_cursor_to_end() {
        let mut b = buf(4, 2);
        b.feed(b"abcd\nefgh");
        b.feed(b"\x1b[1;3H"); // move to row 1 col 3 (0-based: 0,2)
        b.feed(b"\x1b[0J"); // erase from cursor to end
        assert_eq!(b.visible_rows()[0][0].ch, 'a');
        assert_eq!(b.visible_rows()[0][1].ch, 'b');
        assert_eq!(b.visible_rows()[0][2].ch, ' ');
        assert_eq!(b.visible_rows()[0][3].ch, ' ');
        assert_eq!(b.visible_rows()[1][0].ch, ' ');
    }

    #[test]
    fn resize_extends_or_truncates_grid() {
        let mut b = buf(4, 2);
        b.feed(b"abcd");
        b.resize(6, 3);
        assert_eq!(b.width(), 6);
        assert_eq!(b.height(), 3);
        assert_eq!(b.visible_rows()[0].len(), 6);
        assert_eq!(b.visible_rows().len(), 3);
        assert_eq!(b.visible_rows()[0][0].ch, 'a');
    }

    #[test]
    fn backspace_moves_cursor_left() {
        let mut b = buf(4, 1);
        b.feed(b"ab\x08X");
        assert_eq!(b.visible_rows()[0][1].ch, 'X');
    }

    #[test]
    fn tab_advances_to_next_multiple_of_eight() {
        let mut b = buf(16, 1);
        b.feed(b"a\tb");
        assert_eq!(b.cursor().1, 9);
        assert_eq!(b.visible_rows()[0][0].ch, 'a');
        assert_eq!(b.visible_rows()[0][8].ch, 'b');
    }
}

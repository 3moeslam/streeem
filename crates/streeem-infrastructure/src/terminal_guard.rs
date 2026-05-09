use std::io::{self, Stdout, Write, stdout};

use crossterm::cursor::Show;
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

#[derive(Debug)]
pub struct TerminalGuard {
    out: Stdout,
}

impl TerminalGuard {
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut out = stdout();
        execute!(out, EnterAlternateScreen)?;
        Ok(Self { out })
    }

    pub fn out_mut(&mut self) -> &mut Stdout {
        &mut self.out
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.out, LeaveAlternateScreen, Show);
        let _ = self.out.flush();
    }
}

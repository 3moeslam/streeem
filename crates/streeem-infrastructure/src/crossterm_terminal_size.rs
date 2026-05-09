use streeem_domain::ports::terminal_size::TerminalSize;

#[derive(Debug, Default, Clone, Copy)]
pub struct CrosstermTerminalSize;

impl TerminalSize for CrosstermTerminalSize {
    fn size(&self) -> (u16, u16) {
        crossterm::terminal::size().unwrap_or((80, 24))
    }
}

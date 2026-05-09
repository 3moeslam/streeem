//! Bin-side input/command mode toggle.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Command,
    Input,
}

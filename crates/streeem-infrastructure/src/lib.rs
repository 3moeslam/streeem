#![doc = "Adapters: PTY, terminal IO, clock, ratatui rendering. Implements ports defined inward."]

pub mod crossterm_input_adapter;
pub mod crossterm_terminal_size;
pub mod portable_pty_spawner;
pub mod ratatui_renderer;
pub mod system_clock;
pub mod terminal_guard;

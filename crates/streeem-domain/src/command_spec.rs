#![cfg_attr(test, allow(clippy::unwrap_used))]
//! User-supplied command + per-tile rows hint.

use crate::rows_hint::RowsHint;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandSpec {
    pub command: String,
    pub rows_hint: RowsHint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSpecError {
    EmptyCommand,
}

impl CommandSpec {
    pub fn new(command: impl Into<String>, rows_hint: RowsHint) -> Result<Self, CommandSpecError> {
        let command = command.into();
        if command.trim().is_empty() {
            return Err(CommandSpecError::EmptyCommand);
        }
        Ok(Self { command, rows_hint })
    }

    pub fn with_default_rows(command: impl Into<String>) -> Result<Self, CommandSpecError> {
        Self::new(command, RowsHint::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_command() {
        assert_eq!(
            CommandSpec::with_default_rows(""),
            Err(CommandSpecError::EmptyCommand)
        );
    }

    #[test]
    fn rejects_whitespace_only_command() {
        assert_eq!(
            CommandSpec::with_default_rows("   \t  "),
            Err(CommandSpecError::EmptyCommand)
        );
    }

    #[test]
    fn accepts_normal_command_with_default_rows() {
        let s = CommandSpec::with_default_rows("echo hi").unwrap();
        assert_eq!(s.command, "echo hi");
        assert_eq!(s.rows_hint, RowsHint::default());
    }

    #[test]
    fn accepts_explicit_rows_hint() {
        let s = CommandSpec::new("cargo watch", RowsHint::new(20).unwrap()).unwrap();
        assert_eq!(s.rows_hint.value(), 20);
    }
}

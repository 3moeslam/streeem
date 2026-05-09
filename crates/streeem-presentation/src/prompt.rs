#![cfg_attr(test, allow(clippy::panic))]
//! Pure state machine for the in-app "add tile" prompt.

use streeem_application::command::Command;
use streeem_domain::command_spec::{CommandSpec, CommandSpecError};
use streeem_domain::ports::input_source::{KeyCode, KeyEvent};
use streeem_domain::rows_hint::RowsHint;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PromptState {
    pub buffer: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptOutcome {
    Continue,
    Cancelled,
    Submitted(Command),
    InvalidSubmission(CommandSpecError),
}

impl PromptState {
    pub fn open(&mut self) {
        self.active = true;
        self.buffer.clear();
    }

    pub fn handle(&mut self, key: KeyEvent) -> PromptOutcome {
        if !self.active {
            return PromptOutcome::Continue;
        }
        match key.code {
            KeyCode::Esc => {
                self.active = false;
                self.buffer.clear();
                PromptOutcome::Cancelled
            }
            KeyCode::Enter => {
                let text = std::mem::take(&mut self.buffer);
                self.active = false;
                match CommandSpec::new(text, RowsHint::default()) {
                    Ok(spec) => PromptOutcome::Submitted(Command::AddTile(spec)),
                    Err(e) => PromptOutcome::InvalidSubmission(e),
                }
            }
            KeyCode::Backspace => {
                self.buffer.pop();
                PromptOutcome::Continue
            }
            KeyCode::Char(c) => {
                self.buffer.push(c);
                PromptOutcome::Continue
            }
            _ => PromptOutcome::Continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(c: char) -> KeyEvent {
        KeyEvent::plain(KeyCode::Char(c))
    }

    #[test]
    fn typing_appends_to_buffer() {
        let mut p = PromptState::default();
        p.open();
        p.handle(key('a'));
        p.handle(key('b'));
        assert_eq!(p.buffer, "ab");
    }

    #[test]
    fn backspace_pops_last_char() {
        let mut p = PromptState::default();
        p.open();
        p.handle(key('h'));
        p.handle(key('i'));
        p.handle(KeyEvent::plain(KeyCode::Backspace));
        assert_eq!(p.buffer, "h");
    }

    #[test]
    fn enter_submits_add_tile_command() {
        let mut p = PromptState::default();
        p.open();
        for c in "echo hi".chars() {
            p.handle(key(c));
        }
        match p.handle(KeyEvent::plain(KeyCode::Enter)) {
            PromptOutcome::Submitted(Command::AddTile(spec)) => {
                assert_eq!(spec.command, "echo hi");
            }
            other => panic!("expected Submitted(AddTile), got {other:?}"),
        }
        assert!(!p.active);
    }

    #[test]
    fn enter_with_empty_buffer_yields_invalid_submission() {
        let mut p = PromptState::default();
        p.open();
        match p.handle(KeyEvent::plain(KeyCode::Enter)) {
            PromptOutcome::InvalidSubmission(_) => {}
            other => panic!("expected InvalidSubmission, got {other:?}"),
        }
    }

    #[test]
    fn esc_cancels_and_clears_buffer() {
        let mut p = PromptState::default();
        p.open();
        p.handle(key('x'));
        match p.handle(KeyEvent::plain(KeyCode::Esc)) {
            PromptOutcome::Cancelled => {}
            other => panic!("expected Cancelled, got {other:?}"),
        }
        assert!(!p.active);
        assert!(p.buffer.is_empty());
    }
}

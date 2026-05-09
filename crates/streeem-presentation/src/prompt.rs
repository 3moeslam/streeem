#![cfg_attr(test, allow(clippy::panic))]
//! Pure state machine for the in-app prompt (add tile / rename tile).

use streeem_application::command::Command;
use streeem_domain::command_spec::{CommandSpec, CommandSpecError};
use streeem_domain::ports::input_source::{KeyCode, KeyEvent};
use streeem_domain::rows_hint::RowsHint;
use streeem_domain::tile_id::TileId;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PromptPurpose {
    #[default]
    AddTile,
    RenameTile(TileId),
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PromptState {
    pub buffer: String,
    pub active: bool,
    pub purpose: PromptPurpose,
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
        self.purpose = PromptPurpose::AddTile;
    }

    pub fn open_for_rename(&mut self, tile_id: TileId) {
        self.active = true;
        self.buffer.clear();
        self.purpose = PromptPurpose::RenameTile(tile_id);
    }

    /// Returns a short label like "add" or "rename" — used for the on-screen prompt prefix.
    pub fn label(&self) -> &'static str {
        match self.purpose {
            PromptPurpose::AddTile => "add",
            PromptPurpose::RenameTile(_) => "rename",
        }
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
                let purpose = std::mem::take(&mut self.purpose);
                self.active = false;
                match purpose {
                    PromptPurpose::AddTile => match CommandSpec::new(text, RowsHint::default()) {
                        Ok(spec) => PromptOutcome::Submitted(Command::AddTile(spec)),
                        Err(e) => PromptOutcome::InvalidSubmission(e),
                    },
                    PromptPurpose::RenameTile(id) => {
                        let trimmed = text.trim().to_string();
                        if trimmed.is_empty() {
                            PromptOutcome::Cancelled
                        } else {
                            PromptOutcome::Submitted(Command::RenameTile { id, name: trimmed })
                        }
                    }
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

    #[test]
    fn open_for_rename_sets_rename_purpose() {
        let id = TileId::default_from(42);
        let mut p = PromptState::default();
        p.open_for_rename(id);
        p.handle(key('f'));
        p.handle(key('o'));
        p.handle(key('o'));
        match p.handle(KeyEvent::plain(KeyCode::Enter)) {
            PromptOutcome::Submitted(Command::RenameTile { id: rid, name }) => {
                assert_eq!(rid, id);
                assert_eq!(name, "foo");
            }
            other => panic!("expected Submitted(RenameTile), got {other:?}"),
        }
        assert!(!p.active);
    }

    #[test]
    fn rename_with_empty_input_cancels() {
        let id = TileId::default_from(7);
        let mut p = PromptState::default();
        p.open_for_rename(id);
        match p.handle(KeyEvent::plain(KeyCode::Enter)) {
            PromptOutcome::Cancelled => {}
            other => panic!("expected Cancelled, got {other:?}"),
        }
    }

    #[test]
    fn label_reflects_purpose() {
        let mut p = PromptState::default();
        assert_eq!(p.label(), "add");
        p.open_for_rename(TileId::default_from(1));
        assert_eq!(p.label(), "rename");
        p.open();
        assert_eq!(p.label(), "add");
    }
}

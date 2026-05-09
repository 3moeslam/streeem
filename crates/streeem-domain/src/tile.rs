#![cfg_attr(test, allow(clippy::unwrap_used))]
//! A single hosted tile: identity, colour, command, scrollback, run status.

use crate::command_spec::CommandSpec;
use crate::exit_status::ExitStatus;
use crate::output_line::OutputLine;
use crate::rows_hint::RowsHint;
use crate::scrollback::Scrollback;
use crate::scrollback_capacity::ScrollbackCapacity;
use crate::tile_color::TileColor;
use crate::tile_id::TileId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Spawning,
    Running,
    Exited(ExitStatus),
}

#[derive(Debug, Clone)]
pub struct Tile {
    pub id: TileId,
    pub color: TileColor,
    pub spec: CommandSpec,
    pub rows_hint: RowsHint,
    pub scrollback: Scrollback,
    pub run_status: RunStatus,
    pub follow_tail: bool,
    pub scroll_offset_from_bottom: u32,
}

impl Tile {
    pub fn new(
        id: TileId,
        color: TileColor,
        spec: CommandSpec,
        capacity: ScrollbackCapacity,
    ) -> Self {
        let rows_hint = spec.rows_hint;
        Self {
            id,
            color,
            spec,
            rows_hint,
            scrollback: Scrollback::new(capacity),
            run_status: RunStatus::Spawning,
            follow_tail: true,
            scroll_offset_from_bottom: 0,
        }
    }

    pub fn mark_running(&mut self) {
        self.run_status = RunStatus::Running;
    }

    pub fn mark_exited(&mut self, status: ExitStatus) {
        self.run_status = RunStatus::Exited(status);
    }

    pub fn append_output(&mut self, line: OutputLine) {
        self.scrollback.push(line);
    }

    pub fn resize(&mut self, delta: i16) {
        self.rows_hint = self.rows_hint.saturating_add(delta);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec() -> CommandSpec {
        CommandSpec::with_default_rows("echo hi").unwrap()
    }

    fn make_tile() -> Tile {
        Tile::new(
            TileId::default_from(7),
            TileColor::Red,
            sample_spec(),
            ScrollbackCapacity::default(),
        )
    }

    #[test]
    fn newly_created_tile_is_spawning() {
        assert_eq!(make_tile().run_status, RunStatus::Spawning);
    }

    #[test]
    fn newly_created_tile_follows_tail() {
        assert!(make_tile().follow_tail);
    }

    #[test]
    fn mark_running_transitions_status() {
        let mut t = make_tile();
        t.mark_running();
        assert_eq!(t.run_status, RunStatus::Running);
    }

    #[test]
    fn mark_exited_records_status() {
        let mut t = make_tile();
        t.mark_exited(ExitStatus::Code(0));
        assert_eq!(t.run_status, RunStatus::Exited(ExitStatus::Code(0)));
    }

    #[test]
    fn append_output_pushes_into_scrollback() {
        let mut t = make_tile();
        t.append_output(OutputLine::plain_text("first"));
        assert_eq!(t.scrollback.len(), 1);
    }

    #[test]
    fn resize_clamps_via_rows_hint() {
        let mut t = make_tile();
        t.resize(-100);
        assert_eq!(t.rows_hint, RowsHint::new(1).unwrap());
    }
}

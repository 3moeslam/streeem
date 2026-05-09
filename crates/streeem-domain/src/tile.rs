#![cfg_attr(test, allow(clippy::unwrap_used))]
//! A single hosted tile: identity, colour, command, terminal buffer, run status.

use crate::command_spec::CommandSpec;
use crate::exit_status::ExitStatus;
use crate::rows_hint::RowsHint;
use crate::scrollback_capacity::ScrollbackCapacity;
use crate::terminal_buffer::TerminalBuffer;
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
    pub buffer: TerminalBuffer,
    pub run_status: RunStatus,
    pub follow_tail: bool,
    pub scroll_offset_from_bottom: u32,
    pub name: Option<String>,
    pub brave_mode: bool,
}

impl Tile {
    pub fn new(
        id: TileId,
        color: TileColor,
        spec: CommandSpec,
        capacity: ScrollbackCapacity,
    ) -> Self {
        let rows_hint = spec.rows_hint;
        let name = spec.name.clone();
        Self {
            id,
            color,
            spec,
            rows_hint,
            buffer: TerminalBuffer::new(80, 24, capacity),
            run_status: RunStatus::Spawning,
            follow_tail: true,
            scroll_offset_from_bottom: 0,
            name,
            brave_mode: false,
        }
    }

    pub fn mark_running(&mut self) {
        self.run_status = RunStatus::Running;
    }

    pub fn mark_exited(&mut self, status: ExitStatus) {
        self.run_status = RunStatus::Exited(status);
    }

    pub fn feed_bytes(&mut self, bytes: &[u8]) {
        self.buffer.feed(bytes);
    }

    pub fn resize(&mut self, delta: i16) {
        self.rows_hint = self.rows_hint.saturating_add(delta);
    }

    pub fn resize_buffer(&mut self, width: u16, height: u16) {
        self.buffer.resize(width, height);
    }

    pub fn toggle_brave_mode(&mut self) {
        self.brave_mode = !self.brave_mode;
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
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
    fn feed_bytes_appends_to_buffer() {
        let mut t = make_tile();
        t.feed_bytes(b"hi");
        assert_eq!(t.buffer.visible_rows()[0][0].ch, 'h');
    }

    #[test]
    fn resize_clamps_via_rows_hint() {
        let mut t = make_tile();
        t.resize(-100);
        assert_eq!(t.rows_hint, RowsHint::new(1).unwrap());
    }

    #[test]
    fn name_is_none_when_not_provided() {
        let spec = CommandSpec::with_default_rows("echo hi").unwrap();
        let tile = Tile::new(
            TileId::default_from(0),
            TileColor::Red,
            spec,
            ScrollbackCapacity::default(),
        );
        assert_eq!(tile.name, None);
    }

    #[test]
    fn set_name_replaces_name() {
        let mut tile = make_tile();
        assert_eq!(tile.name, None);
        tile.set_name("foo".to_string());
        assert_eq!(tile.name, Some("foo".to_string()));
        tile.set_name("bar".to_string());
        assert_eq!(tile.name, Some("bar".to_string()));
    }

    #[test]
    fn name_field_holds_provided_name() {
        let spec =
            CommandSpec::new_with_name("echo hi", Some("foo".to_string()), RowsHint::default())
                .unwrap();
        let tile = Tile::new(
            TileId::default_from(0),
            TileColor::Red,
            spec,
            ScrollbackCapacity::default(),
        );
        assert_eq!(tile.name, Some("foo".to_string()));
    }
}

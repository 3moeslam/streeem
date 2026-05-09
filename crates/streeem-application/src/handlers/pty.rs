#![cfg_attr(test, allow(clippy::unwrap_used))]

use streeem_domain::event::DomainEvent;
use streeem_domain::exit_status::ExitStatus;
use streeem_domain::outbox::OutboxEffect;
use streeem_domain::reducer::reduce;
use streeem_domain::state::State;
use streeem_domain::tile_id::TileId;

pub fn handle_bytes(state: &mut State, id: TileId, bytes: Vec<u8>) -> Vec<OutboxEffect> {
    reduce(state, DomainEvent::BytesReceived { id, bytes })
}

pub fn handle_spawned(state: &mut State, id: TileId) -> Vec<OutboxEffect> {
    reduce(state, DomainEvent::TileMarkedRunning(id))
}

pub fn handle_exited(state: &mut State, id: TileId, status: ExitStatus) -> Vec<OutboxEffect> {
    reduce(state, DomainEvent::TileExited { id, status })
}

pub fn handle_terminal_resized(state: &mut State, width: u16, height: u16) -> Vec<OutboxEffect> {
    reduce(state, DomainEvent::TerminalResized { width, height })
}

pub fn handle_buffer_resize(
    state: &mut State,
    id: TileId,
    width: u16,
    height: u16,
) -> Vec<OutboxEffect> {
    reduce(state, DomainEvent::TileBufferResized { id, width, height })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::lifecycle::handle_add_tile;
    use streeem_domain::column_count::ColumnCount;
    use streeem_domain::command_spec::CommandSpec;

    fn fresh() -> State {
        State::new(ColumnCount::new(2).unwrap(), 100, 30)
    }

    #[test]
    fn bytes_appended_to_tile_buffer() {
        let mut s = fresh();
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("a").unwrap());
        let id = s.grid.tiles[0].id;
        let _ = handle_bytes(&mut s, id, b"hi".to_vec());
        assert_eq!(s.grid.tiles[0].buffer.visible_rows()[0][0].ch, 'h');
    }

    #[test]
    fn spawned_marks_tile_running() {
        let mut s = fresh();
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("a").unwrap());
        let id = s.grid.tiles[0].id;
        let _ = handle_spawned(&mut s, id);
        assert!(matches!(
            s.grid.tiles[0].run_status,
            streeem_domain::tile::RunStatus::Running
        ));
    }

    #[test]
    fn exited_removes_tile_and_emits_abort() {
        let mut s = fresh();
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("a").unwrap());
        let id = s.grid.tiles[0].id;
        let out = handle_exited(&mut s, id, ExitStatus::Code(0));
        assert!(s.grid.tiles.is_empty());
        assert!(out.iter().any(|e| matches!(e, OutboxEffect::AbortPty(_))));
    }

    #[test]
    fn terminal_resized_updates_grid_size() {
        let mut s = fresh();
        let _ = handle_terminal_resized(&mut s, 200, 50);
        assert_eq!(s.grid.terminal_width, 200);
        assert_eq!(s.grid.terminal_height, 50);
    }

    #[test]
    fn handle_buffer_resize_changes_tile_buffer_dimensions() {
        let mut s = fresh();
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("a").unwrap());
        let id = s.grid.tiles[0].id;
        let out = handle_buffer_resize(&mut s, id, 60, 15);
        assert_eq!(s.grid.tiles[0].buffer.width(), 60);
        assert_eq!(s.grid.tiles[0].buffer.height(), 15);
        assert!(
            out.iter()
                .any(|e| matches!(e, OutboxEffect::ResizePty { .. }))
        );
    }

    #[test]
    fn handle_buffer_resize_for_unknown_id_does_not_emit_resize_pty() {
        let mut s = fresh();
        let out = handle_buffer_resize(
            &mut s,
            streeem_domain::tile_id::TileId::default_from(99),
            50,
            10,
        );
        assert!(
            !out.iter()
                .any(|e| matches!(e, OutboxEffect::ResizePty { .. }))
        );
    }
}

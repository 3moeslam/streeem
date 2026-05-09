#![cfg_attr(test, allow(clippy::unwrap_used))]

use streeem_domain::event::DomainEvent;
use streeem_domain::outbox::OutboxEffect;
use streeem_domain::reducer::reduce;
use streeem_domain::state::State;
use streeem_domain::tile_id::TileId;

use streeem_domain::command_spec::CommandSpec;

pub fn handle_add_tile(state: &mut State, spec: CommandSpec) -> Vec<OutboxEffect> {
    let id = state.id_factory.next_id();
    reduce(state, DomainEvent::TileAdded { id, spec })
}

pub fn handle_drop_tile(state: &mut State, id: TileId) -> Vec<OutboxEffect> {
    reduce(state, DomainEvent::TileDropped(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use streeem_domain::column_count::ColumnCount;

    fn fresh() -> State {
        State::new(ColumnCount::new(2).unwrap(), 100, 30)
    }

    #[test]
    fn add_tile_assigns_a_new_id_and_emits_spawn_pty() {
        let mut s = fresh();
        let spec = CommandSpec::with_default_rows("echo a").unwrap();
        let out = handle_add_tile(&mut s, spec);
        assert_eq!(s.grid.tiles.len(), 1);
        assert!(
            out.iter()
                .any(|e| matches!(e, OutboxEffect::SpawnPty { .. }))
        );
    }

    #[test]
    fn drop_tile_removes_and_emits_abort_pty() {
        let mut s = fresh();
        let _ = handle_add_tile(&mut s, CommandSpec::with_default_rows("a").unwrap());
        let id = s.grid.tiles[0].id;
        let out = handle_drop_tile(&mut s, id);
        assert!(s.grid.tiles.is_empty());
        assert!(out.iter().any(|e| matches!(e, OutboxEffect::AbortPty(_))));
    }
}

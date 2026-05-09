use streeem_domain::outbox::OutboxEffect;
use streeem_domain::state::State;

use crate::command::Command;

pub mod lifecycle;

pub fn handle(state: &mut State, command: Command) -> Vec<OutboxEffect> {
    match command {
        Command::AddTile(spec) => lifecycle::handle_add_tile(state, spec),
        Command::DropTile(id) => lifecycle::handle_drop_tile(state, id),
        _ => Vec::new(),
    }
}

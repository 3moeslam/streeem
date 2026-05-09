use streeem_domain::outbox::OutboxEffect;
use streeem_domain::state::State;

use crate::command::Command;

pub fn handle(_state: &mut State, _command: Command) -> Vec<OutboxEffect> {
    Vec::new()
}

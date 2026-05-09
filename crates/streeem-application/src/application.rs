//! Application shell that owns the State and dispatches Commands through handlers.

use streeem_domain::outbox::OutboxEffect;
use streeem_domain::state::State;

use crate::command::Command;
use crate::handlers;

pub struct Application {
    state: State,
}

impl Application {
    pub fn new(state: State) -> Self {
        Self { state }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn dispatch(&mut self, command: Command) -> Vec<OutboxEffect> {
        handlers::handle(&mut self.state, command)
    }
}

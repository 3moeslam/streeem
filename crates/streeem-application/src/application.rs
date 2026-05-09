//! Application shell that owns the State and dispatches Commands through handlers.

use streeem_domain::outbox::OutboxEffect;
use streeem_domain::state::State;

use crate::command::Command;
use crate::handlers;
use crate::query::{RenderSnapshot, snapshot as build_snapshot};

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

impl Application {
    pub fn snapshot(&self) -> RenderSnapshot {
        build_snapshot(&self.state)
    }
}

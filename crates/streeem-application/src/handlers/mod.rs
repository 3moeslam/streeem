use streeem_domain::outbox::OutboxEffect;
use streeem_domain::state::State;

use crate::command::Command;

pub mod interaction;
pub mod lifecycle;
pub mod pty;

pub fn handle(state: &mut State, command: Command) -> Vec<OutboxEffect> {
    match command {
        Command::AddTile(spec) => lifecycle::handle_add_tile(state, spec),
        Command::DropTile(id) => lifecycle::handle_drop_tile(state, id),
        Command::ResizeTile { id, delta_rows } => interaction::handle_resize(state, id, delta_rows),
        Command::ScrollTile { id, delta } => interaction::handle_scroll(state, id, delta),
        Command::MoveFocus(m) => interaction::handle_focus(state, m),
        Command::ToggleFollowTail(id) => interaction::handle_follow_tail(state, id),
        Command::OnPtyOutput { id, lines } => pty::handle_output(state, id, lines),
        Command::OnPtySpawned(id) => pty::handle_spawned(state, id),
        Command::OnPtyExited { id, status } => pty::handle_exited(state, id, status),
        Command::OnTerminalResized { width, height } => {
            pty::handle_terminal_resized(state, width, height)
        }
    }
}

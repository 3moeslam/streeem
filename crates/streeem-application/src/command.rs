//! External requests dispatched into the application.

use streeem_domain::command_spec::CommandSpec;
use streeem_domain::exit_status::ExitStatus;
use streeem_domain::grid::FocusMove;
use streeem_domain::output_line::OutputLine;
use streeem_domain::tile_id::TileId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrollDelta {
    Lines(i32),
    Page(i32),
    Top,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    AddTile(CommandSpec),
    DropTile(TileId),
    ResizeTile { id: TileId, delta_rows: i16 },
    ScrollTile { id: TileId, delta: ScrollDelta },
    MoveFocus(FocusMove),
    ToggleFollowTail(TileId),
    OnPtyOutput { id: TileId, lines: Vec<OutputLine> },
    OnPtySpawned(TileId),
    OnPtyExited { id: TileId, status: ExitStatus },
    OnTerminalResized { width: u16, height: u16 },
}

//! Events the reducer accepts.

use crate::command_spec::CommandSpec;
use crate::exit_status::ExitStatus;
use crate::grid::FocusMove;
use crate::output_line::OutputLine;
use crate::tile_id::TileId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    TileAdded { id: TileId, spec: CommandSpec },
    TileSpawnFailed { spec: CommandSpec, reason: String },
    TileMarkedRunning(TileId),
    TileExited { id: TileId, status: ExitStatus },
    OutputAppended { id: TileId, lines: Vec<OutputLine> },
    TileDropped(TileId),
    TileResized { id: TileId, delta_rows: i16 },
    FocusMoved(FocusMove),
    TileScrolled { id: TileId, delta_lines: i32 },
    FollowTailToggled(TileId),
    TerminalResized { width: u16, height: u16 },
}

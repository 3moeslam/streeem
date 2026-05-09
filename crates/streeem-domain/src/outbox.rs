//! Side effects the reducer asks the outer world to perform after a transition.

use crate::command_spec::CommandSpec;
use crate::tile_id::TileId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboxEffect {
    SpawnPty { id: TileId, spec: CommandSpec },
    AbortPty(TileId),
    ResizePty { id: TileId, cols: u16, rows: u16 },
    RecordAlert(String),
    MarkFrameDirty,
}

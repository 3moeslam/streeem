//! All mutable domain state, bundled for the reducer.

use crate::color_palette::ColorPalette;
use crate::column_count::ColumnCount;
use crate::grid::Grid;
use crate::scrollback_capacity::ScrollbackCapacity;
use crate::tile_id::TileIdFactory;

#[derive(Debug, Clone)]
pub struct Alert {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct State {
    pub grid: Grid,
    pub palette: ColorPalette,
    pub id_factory: TileIdFactory,
    pub scrollback_capacity: ScrollbackCapacity,
    pub alerts: Vec<Alert>,
    pub dirty: bool,
    pub max_alerts: usize,
}

impl State {
    pub fn new(columns: ColumnCount, terminal_width: u16, terminal_height: u16) -> Self {
        Self {
            grid: Grid::new(columns, terminal_width, terminal_height),
            palette: ColorPalette::new(),
            id_factory: TileIdFactory::new(),
            scrollback_capacity: ScrollbackCapacity::default(),
            alerts: Vec::new(),
            dirty: true,
            max_alerts: 3,
        }
    }
}

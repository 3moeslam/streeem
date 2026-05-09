//! Cell-level color for terminal output. Supports default, indexed (256),
//! and truecolor (RGB).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CellColor {
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

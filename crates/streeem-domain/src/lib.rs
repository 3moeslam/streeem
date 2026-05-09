#![doc = "Pure domain layer for streeem. No I/O, no async, no UI types."]

pub mod ansi;
pub mod color_palette;
pub mod column_count;
pub mod command_spec;
pub mod exit_status;
pub mod layout_packer;
pub mod output_line;
pub mod rows_hint;
pub mod scrollback;
pub mod scrollback_capacity;
pub mod style;
pub mod styled_span;
pub mod tile;
pub mod tile_color;
pub mod tile_id;

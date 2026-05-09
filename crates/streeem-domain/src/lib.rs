#![doc = "Pure domain layer for streeem. No I/O, no async, no UI types."]

pub mod ports;

pub mod color_palette;
pub mod column_count;
pub mod command_spec;
pub mod event;
pub mod exit_status;
pub mod grid;
pub mod layout_packer;
pub mod outbox;
pub mod reducer;
pub mod rows_hint;
pub mod scrollback_capacity;
pub mod state;
pub mod style;
pub mod terminal_buffer;
pub mod tile;
pub mod tile_color;
pub mod tile_id;

#![cfg_attr(test, allow(clippy::unwrap_used))]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]
//! Pure state machine: applies a DomainEvent to State and emits OutboxEffects.

use crate::event::DomainEvent;
use crate::outbox::OutboxEffect;
use crate::state::{Alert, State};
use crate::tile::Tile;

pub fn reduce(state: &mut State, event: DomainEvent) -> Vec<OutboxEffect> {
    let mut out = Vec::new();
    match event {
        DomainEvent::TileAdded { id, spec } => {
            let color = state.palette.assign();
            let tile = Tile::new(id, color, spec.clone(), state.scrollback_capacity);
            state.grid.add(tile);
            out.push(OutboxEffect::SpawnPty { id, spec });
            state.dirty = true;
        }
        DomainEvent::TileSpawnFailed { spec, reason } => {
            state.alerts.push(Alert {
                message: format!("spawn failed: {} ({reason})", spec.command),
            });
            while state.alerts.len() > state.max_alerts {
                state.alerts.remove(0);
            }
            state.dirty = true;
        }
        DomainEvent::TileMarkedRunning(id) => {
            if let Some(tile) = state.grid.tiles.iter_mut().find(|t| t.id == id) {
                tile.mark_running();
                state.dirty = true;
            }
        }
        DomainEvent::TileExited { id, status } => {
            if let Some(tile) = state.grid.tiles.iter().find(|t| t.id == id) {
                let color = tile.color;
                state.grid.drop(id);
                state.palette.release(color);
                out.push(OutboxEffect::AbortPty(id));
                state.dirty = true;
                let _ = status;
            }
        }
        DomainEvent::OutputAppended { id, lines } => {
            if let Some(tile) = state.grid.tiles.iter_mut().find(|t| t.id == id) {
                for line in lines {
                    tile.append_output(line);
                }
                state.dirty = true;
            }
        }
        DomainEvent::TileDropped(id) => {
            if let Some(tile) = state.grid.tiles.iter().find(|t| t.id == id) {
                let color = tile.color;
                state.grid.drop(id);
                state.palette.release(color);
                out.push(OutboxEffect::AbortPty(id));
                state.dirty = true;
            }
        }
        DomainEvent::TileResized { id, delta_rows } => {
            if let Some(tile) = state.grid.tiles.iter_mut().find(|t| t.id == id) {
                tile.resize(delta_rows);
                state.dirty = true;
            }
        }
        DomainEvent::FocusMoved(m) => {
            use crate::grid::FocusMove;
            use crate::layout_packer::{LayoutInput, pack};
            if matches!(m, FocusMove::Spatial(_)) {
                let tiles_for_packing: Vec<_> = state
                    .grid
                    .tiles
                    .iter()
                    .map(|t| (t.id, t.rows_hint))
                    .collect();
                let placements = pack(LayoutInput {
                    tiles: &tiles_for_packing,
                    columns: state.grid.columns,
                    terminal_width: state.grid.terminal_width,
                    terminal_height: state.grid.terminal_height,
                });
                state.grid.move_focus_with_placements(m, &placements);
            } else {
                state.grid.move_focus(m);
            }
            state.dirty = true;
        }
        DomainEvent::TileScrolled { id, delta_lines } => {
            if let Some(tile) = state.grid.tiles.iter_mut().find(|t| t.id == id) {
                let new_offset = (tile.scroll_offset_from_bottom as i64) - (delta_lines as i64);
                tile.scroll_offset_from_bottom = new_offset.max(0) as u32;
                tile.follow_tail = tile.scroll_offset_from_bottom == 0;
                state.dirty = true;
            }
        }
        DomainEvent::FollowTailToggled(id) => {
            if let Some(tile) = state.grid.tiles.iter_mut().find(|t| t.id == id) {
                tile.follow_tail = !tile.follow_tail;
                if tile.follow_tail {
                    tile.scroll_offset_from_bottom = 0;
                }
                state.dirty = true;
            }
        }
        DomainEvent::TerminalResized { width, height } => {
            state.grid.terminal_width = width;
            state.grid.terminal_height = height;
            if state.columns_override.is_none() {
                let new_cols = (width / state.min_tile_width.max(1)).max(1);
                if let Ok(cc) = crate::column_count::ColumnCount::new(new_cols) {
                    state.grid.columns = cc;
                }
            }
            state.dirty = true;
        }
    }
    out.push(OutboxEffect::MarkFrameDirty);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column_count::ColumnCount;
    use crate::command_spec::CommandSpec;
    use crate::exit_status::ExitStatus;
    use crate::output_line::OutputLine;
    use crate::tile_id::TileId;

    fn fresh_state() -> State {
        State::new(ColumnCount::new(2).unwrap(), 100, 30)
    }

    fn spec(name: &str) -> CommandSpec {
        CommandSpec::with_default_rows(name).unwrap()
    }

    #[test]
    fn tile_added_creates_tile_and_requests_spawn() {
        let mut state = fresh_state();
        let id = state.id_factory.next_id();
        let out = reduce(
            &mut state,
            DomainEvent::TileAdded {
                id,
                spec: spec("echo a"),
            },
        );
        assert_eq!(state.grid.tiles.len(), 1);
        assert!(matches!(out[0], OutboxEffect::SpawnPty { .. }));
        assert!(state.dirty);
    }

    #[test]
    fn tile_spawn_failed_records_an_alert() {
        let mut state = fresh_state();
        let _ = reduce(
            &mut state,
            DomainEvent::TileSpawnFailed {
                spec: spec("nope"),
                reason: "not found".to_string(),
            },
        );
        assert_eq!(state.alerts.len(), 1);
    }

    #[test]
    fn alerts_are_capped_at_max_alerts() {
        let mut state = fresh_state();
        for i in 0..10 {
            let _ = reduce(
                &mut state,
                DomainEvent::TileSpawnFailed {
                    spec: spec(&format!("c{i}")),
                    reason: "x".to_string(),
                },
            );
        }
        assert_eq!(state.alerts.len(), state.max_alerts);
    }

    #[test]
    fn tile_exited_removes_tile_releases_color_and_aborts_pty() {
        let mut state = fresh_state();
        let id = state.id_factory.next_id();
        let _ = reduce(
            &mut state,
            DomainEvent::TileAdded {
                id,
                spec: spec("a"),
            },
        );
        let out = reduce(
            &mut state,
            DomainEvent::TileExited {
                id,
                status: ExitStatus::Code(0),
            },
        );
        assert!(state.grid.tiles.is_empty());
        assert_eq!(state.palette.in_use_count(), 0);
        assert!(out.iter().any(|e| matches!(e, OutboxEffect::AbortPty(_))));
    }

    #[test]
    fn output_appended_pushes_lines_into_tile_scrollback() {
        let mut state = fresh_state();
        let id = state.id_factory.next_id();
        let _ = reduce(
            &mut state,
            DomainEvent::TileAdded {
                id,
                spec: spec("a"),
            },
        );
        let _ = reduce(
            &mut state,
            DomainEvent::OutputAppended {
                id,
                lines: vec![OutputLine::plain_text("x"), OutputLine::plain_text("y")],
            },
        );
        let tile = state.grid.tiles.iter().find(|t| t.id == id).unwrap();
        assert_eq!(tile.scrollback.len(), 2);
    }

    #[test]
    fn terminal_resized_updates_grid_dimensions() {
        let mut state = fresh_state();
        let _ = reduce(
            &mut state,
            DomainEvent::TerminalResized {
                width: 200,
                height: 50,
            },
        );
        assert_eq!(state.grid.terminal_width, 200);
        assert_eq!(state.grid.terminal_height, 50);
    }

    #[test]
    fn output_appended_for_unknown_id_is_noop() {
        let mut state = fresh_state();
        let _ = reduce(
            &mut state,
            DomainEvent::OutputAppended {
                id: TileId::default_from(99),
                lines: vec![OutputLine::plain_text("ghost")],
            },
        );
        assert!(state.grid.tiles.is_empty());
    }

    #[test]
    fn terminal_resize_recomputes_column_count_when_no_override() {
        let mut state = State::with_layout_config(
            ColumnCount::new(2).unwrap(),
            80,
            30,
            None, // no override
            40,   // min_tile_width
        );
        let _ = reduce(
            &mut state,
            DomainEvent::TerminalResized {
                width: 200,
                height: 40,
            },
        );
        assert_eq!(state.grid.columns, ColumnCount::new(5).unwrap()); // 200/40 = 5
    }

    #[test]
    fn terminal_resize_preserves_column_count_when_override_set() {
        let mut state = State::with_layout_config(
            ColumnCount::new(3).unwrap(),
            120,
            30,
            Some(3), // user override
            40,
        );
        let _ = reduce(
            &mut state,
            DomainEvent::TerminalResized {
                width: 200,
                height: 40,
            },
        );
        assert_eq!(state.grid.columns, ColumnCount::new(3).unwrap()); // unchanged
    }
}

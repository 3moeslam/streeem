//! Pure builder: RenderSnapshot -> FrameDescription.

#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used))]
#![allow(clippy::type_complexity)]

use streeem_application::query::{AlertSnapshot, RenderSnapshot, TileSnapshot};
use streeem_domain::layout_packer::Placement;
use streeem_domain::terminal_buffer::Cell;
use streeem_domain::tile::RunStatus;
use streeem_domain::tile_color::TileColor;

use crate::key_map;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameDescription {
    Tiles {
        alerts: Vec<String>,
        tiles: Vec<TileWidget>,
        prompt: Option<String>,
        status_bar: String,
    },
    TooSmallBanner {
        width: u16,
        height: u16,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileWidget {
    pub placement: Placement,
    pub border_color: TileColor,
    pub title: String,
    pub focused: bool,
    pub cells: Vec<Vec<Cell>>,
    pub cursor: (u16, u16),
    pub clipped: bool,
    pub paused: bool,
}

pub fn build(snap: &RenderSnapshot) -> FrameDescription {
    if snap.too_small {
        return FrameDescription::TooSmallBanner {
            width: snap.terminal_size.0,
            height: snap.terminal_size.1,
            message: "terminal too small (need 40x10)".to_string(),
        };
    }
    let tiles = snap
        .tiles
        .iter()
        .map(|tile_snap| build_tile_widget(snap, tile_snap))
        .collect();
    let alerts = snap
        .alerts
        .iter()
        .map(|a: &AlertSnapshot| a.message.clone())
        .collect();
    FrameDescription::Tiles {
        alerts,
        tiles,
        prompt: None,
        status_bar: key_map::STATUS_BAR_TEXT.to_string(),
    }
}

pub fn build_with_prompt(snap: &RenderSnapshot, prompt_text: Option<String>) -> FrameDescription {
    let mut frame = build(snap);
    if let FrameDescription::Tiles { ref mut prompt, .. } = frame {
        *prompt = prompt_text;
    }
    frame
}

fn build_tile_widget(snap: &RenderSnapshot, tile: &TileSnapshot) -> TileWidget {
    let placement = snap
        .placements
        .iter()
        .copied()
        .find(|p| p.tile_id == tile.id)
        .unwrap_or(Placement {
            tile_id: tile.id,
            column: 0,
            row_offset: 0,
            height: 0,
            width: 0,
            is_clipped: false,
        });
    let row_count = tile.cells.len();
    let status_badges = match (tile.follow_tail, placement.is_clipped, tile.run_status) {
        (false, _, _) => " [paused]".to_string(),
        (_, true, _) => " [clipped]".to_string(),
        (_, _, RunStatus::Spawning) => " [spawning]".to_string(),
        _ => String::new(),
    };
    let is_auto_name = tile.display_name == format!("{}", tile.focus_index);
    let title = if is_auto_name {
        format!(
            "[{n}] {cmd}  (rows {rows}, {lines} lines){badges}",
            n = tile.focus_index,
            cmd = tile.title_command,
            rows = placement.height,
            lines = row_count,
            badges = status_badges,
        )
    } else {
        format!(
            "[{n}] {name}: {cmd}  (rows {rows}, {lines} lines){badges}",
            n = tile.focus_index,
            name = tile.display_name,
            cmd = tile.title_command,
            rows = placement.height,
            lines = row_count,
            badges = status_badges,
        )
    };
    TileWidget {
        placement,
        border_color: tile.color,
        title,
        focused: snap.focused == Some(tile.id),
        cells: tile.cells.clone(),
        cursor: tile.cursor,
        clipped: placement.is_clipped,
        paused: !tile.follow_tail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use streeem_domain::command_spec::CommandSpec;
    use streeem_domain::scrollback_capacity::ScrollbackCapacity;
    use streeem_domain::style::Style;
    use streeem_domain::tile::Tile;
    use streeem_domain::tile_id::TileId;

    fn cells_with(text: &str, width: usize) -> Vec<Vec<Cell>> {
        let mut row: Vec<Cell> = text
            .chars()
            .map(|c| Cell {
                ch: c,
                style: Style::default(),
            })
            .collect();
        while row.len() < width {
            row.push(Cell::default());
        }
        vec![row]
    }

    fn snap_with_one_tile(too_small: bool) -> RenderSnapshot {
        let id = TileId::default_from(0);
        let placement = Placement {
            tile_id: id,
            column: 0,
            row_offset: 0,
            height: 10,
            width: 80,
            is_clipped: false,
        };
        let tile_snap = TileSnapshot {
            id,
            focus_index: 1,
            color: TileColor::Red,
            title_command: "echo a".to_string(),
            display_name: "1".to_string(),
            run_status: RunStatus::Running,
            follow_tail: true,
            scroll_offset_from_bottom: 0,
            cells: cells_with("hello", 80),
            cursor: (0, 5),
        };
        RenderSnapshot {
            terminal_size: if too_small { (20, 5) } else { (80, 30) },
            placements: if too_small {
                Vec::new()
            } else {
                vec![placement]
            },
            tiles: vec![tile_snap],
            focused: Some(id),
            alerts: Vec::new(),
            too_small,
        }
    }

    #[test]
    fn too_small_snapshot_yields_banner() {
        let frame = build(&snap_with_one_tile(true));
        assert!(matches!(frame, FrameDescription::TooSmallBanner { .. }));
    }

    #[test]
    fn normal_snapshot_yields_one_tile_widget() {
        let frame = build(&snap_with_one_tile(false));
        match frame {
            FrameDescription::Tiles {
                tiles,
                alerts,
                prompt: _,
                status_bar: _,
            } => {
                assert_eq!(tiles.len(), 1);
                assert_eq!(tiles[0].border_color, TileColor::Red);
                assert!(tiles[0].title.contains("echo a"));
                assert!(tiles[0].title.starts_with("[1]"));
                // display_name="1" matches focus_index=1, so name segment is omitted
                assert!(!tiles[0].title.contains("1: echo a"));
                assert!(alerts.is_empty());
                assert!(tiles[0].focused);
                assert_eq!(tiles[0].cells[0][0].ch, 'h');
            }
            _ => panic!("expected Tiles"),
        }
    }

    #[test]
    fn paused_tile_shows_paused_badge_in_title() {
        let mut s = snap_with_one_tile(false);
        s.tiles[0].follow_tail = false;
        let frame = build(&s);
        if let FrameDescription::Tiles { tiles, .. } = frame {
            assert!(tiles[0].title.contains("[paused]"));
        }
    }

    #[test]
    fn clipped_tile_shows_clipped_badge_in_title() {
        let mut s = snap_with_one_tile(false);
        s.placements[0].is_clipped = true;
        let frame = build(&s);
        if let FrameDescription::Tiles { tiles, .. } = frame {
            assert!(tiles[0].title.contains("[clipped]"));
        }
    }

    #[test]
    fn alerts_pass_through() {
        let mut s = snap_with_one_tile(false);
        s.alerts.push(AlertSnapshot {
            message: "boom".to_string(),
        });
        if let FrameDescription::Tiles { alerts, .. } = build(&s) {
            assert_eq!(alerts, vec!["boom".to_string()]);
        }
    }

    #[test]
    fn prompt_text_included_in_frame_when_set() {
        let snap = snap_with_one_tile(false);
        let frame = build_with_prompt(&snap, Some("echo".to_string()));
        if let FrameDescription::Tiles { prompt, .. } = frame {
            assert_eq!(prompt, Some("echo".to_string()));
        }
    }

    fn snap_with_named_tile(display_name: &str, focus_index: u8) -> RenderSnapshot {
        let id = TileId::default_from(0);
        let placement = Placement {
            tile_id: id,
            column: 0,
            row_offset: 0,
            height: 10,
            width: 80,
            is_clipped: false,
        };
        let tile_snap = TileSnapshot {
            id,
            focus_index,
            color: TileColor::Red,
            title_command: "echo a".to_string(),
            display_name: display_name.to_string(),
            run_status: RunStatus::Running,
            follow_tail: true,
            scroll_offset_from_bottom: 0,
            cells: Vec::new(),
            cursor: (0, 0),
        };
        RenderSnapshot {
            terminal_size: (80, 30),
            placements: vec![placement],
            tiles: vec![tile_snap],
            focused: Some(id),
            alerts: Vec::new(),
            too_small: false,
        }
    }

    #[test]
    fn unnamed_tile_title_omits_name_segment() {
        // display_name="1" matches focus_index=1: auto-generated, so name segment is hidden.
        let snap = snap_with_named_tile("1", 1);
        let frame = build(&snap);
        if let FrameDescription::Tiles { tiles, .. } = frame {
            assert!(tiles[0].title.starts_with("[1] "));
            assert!(
                !tiles[0].title.contains("1: "),
                "auto name should not appear as 'N: '"
            );
        }
    }

    #[test]
    fn named_tile_title_includes_name_segment() {
        // display_name="alpha" does not match focus_index=1: user-provided name shown.
        let snap = snap_with_named_tile("alpha", 1);
        let frame = build(&snap);
        if let FrameDescription::Tiles { tiles, .. } = frame {
            assert!(tiles[0].title.contains("[1] alpha: "));
        }
    }

    // unused; kept to ensure imports compile in case of refactor
    fn _example_tile() -> Tile {
        Tile::new(
            TileId::default_from(0),
            TileColor::Red,
            CommandSpec::with_default_rows("x").unwrap(),
            ScrollbackCapacity::default(),
        )
    }
}

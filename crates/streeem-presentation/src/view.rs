//! Pure builder: RenderSnapshot -> FrameDescription.

#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used))]

use streeem_application::query::{AlertSnapshot, RenderSnapshot, TileSnapshot};
use streeem_domain::layout_packer::Placement;
use streeem_domain::output_line::OutputLine;
use streeem_domain::tile::RunStatus;
use streeem_domain::tile_color::TileColor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameDescription {
    Tiles {
        alerts: Vec<String>,
        tiles: Vec<TileWidget>,
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
    pub body: Vec<OutputLine>,
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
    FrameDescription::Tiles { alerts, tiles }
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
    let line_count = tile.lines.len();
    let status_badges = match (tile.follow_tail, placement.is_clipped, tile.run_status) {
        (false, _, _) => " [paused]".to_string(),
        (_, true, _) => " [clipped]".to_string(),
        (_, _, RunStatus::Spawning) => " [spawning]".to_string(),
        _ => String::new(),
    };
    let title = format!(
        "[{n}] {cmd}  (rows {rows}, {lines} lines){badges}",
        n = tile.focus_index,
        cmd = tile.title_command,
        rows = placement.height,
        lines = line_count,
        badges = status_badges,
    );
    TileWidget {
        placement,
        border_color: tile.color,
        title,
        focused: snap.focused == Some(tile.id),
        body: tile.lines.clone(),
        clipped: placement.is_clipped,
        paused: !tile.follow_tail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use streeem_domain::command_spec::CommandSpec;
    use streeem_domain::scrollback_capacity::ScrollbackCapacity;
    use streeem_domain::tile::Tile;
    use streeem_domain::tile_id::TileId;

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
            run_status: RunStatus::Running,
            follow_tail: true,
            scroll_offset_from_bottom: 0,
            lines: vec![OutputLine::plain_text("hello")],
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
            FrameDescription::Tiles { tiles, alerts } => {
                assert_eq!(tiles.len(), 1);
                assert_eq!(tiles[0].border_color, TileColor::Red);
                assert!(tiles[0].title.contains("echo a"));
                assert!(tiles[0].title.starts_with("[1]"));
                assert!(alerts.is_empty());
                assert!(tiles[0].focused);
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

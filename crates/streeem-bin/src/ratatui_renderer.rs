use std::io::{Stdout, stdout};

use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style as RStyle};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use streeem_domain::cell_color::CellColor;
use streeem_domain::ports::renderer::{RenderError, Renderer};
use streeem_domain::terminal_buffer::Cell;
use streeem_domain::tile_color::TileColor;
use streeem_presentation::view::{FrameDescription, TileWidget};

use streeem_infrastructure::terminal_guard::TerminalGuard;

pub struct RatatuiRenderer {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    _guard: TerminalGuard,
}

impl RatatuiRenderer {
    pub fn enter() -> Result<Self, RenderError> {
        let guard = TerminalGuard::enter().map_err(|e| RenderError(e.to_string()))?;
        let backend = CrosstermBackend::new(stdout());
        let terminal = Terminal::new(backend).map_err(|e| RenderError(e.to_string()))?;
        Ok(Self {
            terminal,
            _guard: guard,
        })
    }
}

impl Renderer<FrameDescription> for RatatuiRenderer {
    fn render(&mut self, frame: &FrameDescription) -> Result<(), RenderError> {
        self.terminal
            .draw(|f| draw(f.area(), f, frame))
            .map_err(|e| RenderError(e.to_string()))?;
        Ok(())
    }
}

fn draw(area: Rect, f: &mut ratatui::Frame<'_>, desc: &FrameDescription) {
    match desc {
        FrameDescription::TooSmallBanner { message, .. } => {
            let p = Paragraph::new(message.clone()).block(Block::default().borders(Borders::ALL));
            f.render_widget(p, area);
        }
        FrameDescription::Tiles {
            alerts,
            tiles,
            prompt,
            status_bar,
        } => {
            let alert_height = if alerts.is_empty() { 0 } else { 1 };
            if alert_height > 0 {
                let r = Rect {
                    x: area.x,
                    y: area.y,
                    width: area.width,
                    height: alert_height,
                };
                let text = alerts.join(" | ");
                f.render_widget(Paragraph::new(text), r);
            }
            for t in tiles {
                draw_tile(area, f, t, alert_height);
            }
            if let Some(text) = prompt {
                let r = Rect {
                    x: area.x,
                    y: area.y + area.height.saturating_sub(2),
                    width: area.width,
                    height: 1,
                };
                let line = text.clone();
                f.render_widget(Paragraph::new(line), r);
            }
            // Status bar always rendered at the bottom row
            {
                let r = Rect {
                    x: area.x,
                    y: area.y + area.height.saturating_sub(1),
                    width: area.width,
                    height: 1,
                };
                f.render_widget(Paragraph::new(status_bar.clone()), r);
            }
        }
    }
}

fn draw_tile(area: Rect, f: &mut ratatui::Frame<'_>, t: &TileWidget, alert_height: u16) {
    let col_w = area.width / area.width.clamp(1, 255);
    let _ = col_w;
    let r = Rect {
        x: area.x + t.placement.column * t.placement.width,
        y: area.y + alert_height + t.placement.row_offset,
        width: t.placement.width,
        height: t.placement.height,
    };
    let border_color = translate_color(t.border_color);
    let border_style = RStyle::default().fg(border_color);
    let (border_type, title_prefix, title_style) = if t.focused {
        (
            BorderType::Double,
            "▶ ",
            border_style.add_modifier(Modifier::BOLD),
        )
    } else {
        (BorderType::Plain, "", border_style)
    };
    let title_text = format!("{}{}", title_prefix, t.title);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(border_style)
        .title(Span::styled(title_text, title_style));
    let lines: Vec<Line<'_>> = t
        .cells
        .iter()
        .map(|row| {
            let spans: Vec<Span<'_>> = row
                .iter()
                .map(|cell| Span::styled(cell.ch.to_string(), translate_cell_style(cell)))
                .collect();
            Line::from(spans)
        })
        .collect();
    let p = Paragraph::new(lines).block(block);
    f.render_widget(p, r);

    if t.focused {
        let (cur_row, cur_col) = t.cursor;
        // +1 to skip the border; r.x/r.y are the tile's outer top-left corner.
        let cursor_x = r.x + 1 + cur_col;
        let cursor_y = r.y + 1 + cur_row;
        // Clamp into the tile's interior to avoid drawing on/over the border.
        if cursor_x < r.x + r.width.saturating_sub(1) && cursor_y < r.y + r.height.saturating_sub(1)
        {
            f.set_cursor_position(Position {
                x: cursor_x,
                y: cursor_y,
            });
        }
    }
}

fn translate_cell_style(cell: &Cell) -> RStyle {
    let mut style = RStyle::default()
        .fg(translate_cell_color(cell.fg))
        .bg(translate_cell_color(cell.bg));
    if cell.bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if cell.italic {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if cell.underline {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if cell.inverse {
        style = style.add_modifier(Modifier::REVERSED);
    }
    style
}

fn translate_cell_color(c: CellColor) -> Color {
    match c {
        CellColor::Default => Color::Reset,
        CellColor::Indexed(i) => Color::Indexed(i),
        CellColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

fn translate_color(c: TileColor) -> Color {
    match c {
        TileColor::Red => Color::Red,
        TileColor::Green => Color::Green,
        TileColor::Yellow => Color::Yellow,
        TileColor::Blue => Color::Blue,
        TileColor::Magenta => Color::Magenta,
        TileColor::Cyan => Color::Cyan,
        TileColor::LightRed => Color::LightRed,
        TileColor::LightGreen => Color::LightGreen,
        TileColor::LightYellow => Color::LightYellow,
        TileColor::LightBlue => Color::LightBlue,
        TileColor::LightMagenta => Color::LightMagenta,
        TileColor::LightCyan => Color::LightCyan,
    }
}

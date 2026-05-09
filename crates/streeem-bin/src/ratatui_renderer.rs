use std::io::{Stdout, stdout};

use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style as RStyle};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use streeem_domain::ports::renderer::{RenderError, Renderer};
use streeem_domain::style::Style as DStyle;
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
                let line = format!("prompt> {text}");
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
                .map(|cell| Span::styled(cell.ch.to_string(), translate_style(&cell.style)))
                .collect();
            Line::from(spans)
        })
        .collect();
    let p = Paragraph::new(lines).block(block);
    f.render_widget(p, r);
}

fn translate_style(s: &DStyle) -> RStyle {
    let mut style = RStyle::default();
    if let Some(fg) = s.fg {
        style = style.fg(translate_color(fg));
    }
    if let Some(bg) = s.bg {
        style = style.bg(translate_color(bg));
    }
    if s.bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if s.underline {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    style
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

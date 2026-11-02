//! Rendering: the two-column layout.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::{App, Focus};

/// Draw the whole frame: shell left, file browser right.
pub fn draw(f: &mut Frame, app: &mut App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(f.area());

    draw_shell(f, app, cols[0]);
    draw_files(f, app, cols[1]);
}

fn draw_shell(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" shell ")
        .border_style(border_style(app.focus == Focus::Shell));

    let lines: Vec<Line> = app
        .shell_lines
        .iter()
        .map(|l| Line::from(l.as_str()))
        .collect();

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn draw_files(f: &mut Frame, app: &mut App, area: Rect) {
    let title = format!(" {} ", app.cwd.display());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style(app.focus == Focus::Files));

    let items: Vec<ListItem> = app
        .entries
        .iter()
        .map(|e| {
            let style = if e.is_dir {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let icon = if e.is_dir { "▸ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(icon, style),
                Span::styled(e.name.clone(), style),
            ]))
        })
        .collect();

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn border_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

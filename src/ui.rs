//! Rendering: tabbed left pane, file tree on the right, status bar at the bottom.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::{App, Focus, Tab};

pub fn draw(f: &mut Frame, app: &mut App) {
    let rows = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(f.area());
    let cols =
        Layout::horizontal([Constraint::Percentage(68), Constraint::Percentage(32)]).split(rows[0]);
    let left = Layout::vertical([Constraint::Length(1), Constraint::Min(2)]).split(cols[0]);

    draw_tabs(f, app, left[0]);
    draw_content(f, app, left[1]);
    draw_tree(f, app, cols[1]);
    draw_status(f, app, rows[1]);
}

fn draw_tabs(f: &mut Frame, app: &mut App, area: Rect) {
    let buf = f.buffer_mut();
    buf.set_style(area, Style::default().bg(Color::Black));
    app.rects.tabs.clear();
    let mut x = area.x;
    let end = area.right();
    for (idx, tab) in app.tabs.iter().enumerate() {
        let label = match tab {
            Tab::Term(_) => format!(" {} terminal ", idx + 1),
            Tab::Editor(e) => format!(
                " {} {}{} ",
                idx + 1,
                e.file_name(),
                if e.dirty { " ●" } else { "" }
            ),
        };
        let closable = idx > 0;
        let style = if idx == app.active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray).bg(Color::DarkGray)
        };
        let lw = (label.chars().count() as u16).min(end.saturating_sub(x));
        if lw == 0 {
            break;
        }
        buf.set_stringn(x, area.y, &label, lw as usize, style);
        let tab = Rect::new(x, area.y, lw, 1);
        x += lw;
        let close = (closable && x + 2 <= end).then(|| {
            buf.set_string(x, area.y, "× ", style.fg(Color::Red));
            x += 2;
            Rect::new(x - 2, area.y, 1, 1)
        });
        app.rects.tabs.push((idx, tab, close));
        x = (x + 1).min(end);
    }
}

fn draw_content(f: &mut Frame, app: &mut App, area: Rect) {
    let title = match app.editor() {
        Some(e) => format!(" {} ", e.path.display()),
        None => " terminal ".into(),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style(app.focus == Focus::Left));
    let inner = block.inner(area);
    f.render_widget(block, area);
    app.rects.content = inner;
    let cursor = match &mut app.tabs[app.active] {
        Tab::Term(t) => t.render(inner, f.buffer_mut()),
        Tab::Editor(e) => e.render(inner, f.buffer_mut()),
    };
    if let (Some((x, y)), Focus::Left) = (cursor, app.focus) {
        f.set_cursor_position((x, y));
    }
}

fn draw_tree(f: &mut Frame, app: &mut App, area: Rect) {
    app.rects.tree = area;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", app.tree.cwd.display()))
        .border_style(border_style(app.focus == Focus::Tree));
    let items: Vec<ListItem> = app
        .tree
        .entries
        .iter()
        .map(|e| {
            let indent = "  ".repeat(e.depth);
            if e.is_dir {
                let s = Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD);
                let icon = if app.tree.expanded.contains(&e.path) && e.name != ".." {
                    "▾"
                } else {
                    "▸"
                };
                ListItem::new(Line::styled(format!("{indent}{icon} {}", e.name), s))
            } else {
                ListItem::new(format!("{indent}  {}", e.name))
            }
        })
        .collect();
    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );
    f.render_stateful_widget(list, area, &mut app.tree.state);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let (text, style) = if !app.status.is_empty() {
        (
            app.status.clone(),
            Style::default().fg(Color::Black).bg(Color::Yellow),
        )
    } else {
        let hint = match (app.focus, &app.tabs[app.active]) {
            (Focus::Tree, _) => {
                "j/k move · →/Space expand · Enter open · h up · click expand, double-click enter"
            }
            (_, Tab::Term(_)) => {
                "Alt+T new terminal · Alt+W close · Alt+E tree · Alt+←/→ tabs · Ctrl+Q quit"
            }
            _ => "Ctrl+S save · Alt+W close · Alt+E tree · Alt+←/→ tabs · Ctrl+Q quit",
        };
        (
            hint.to_string(),
            Style::default().fg(Color::Gray).bg(Color::Black),
        )
    };
    f.render_widget(Paragraph::new(text).style(style), area);
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

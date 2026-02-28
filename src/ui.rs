use crate::ansi::parse_ansi_line;
use crate::app::App;
use crate::preview::PreviewContent;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Split into left (35%) and right (65%)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(65),
        ])
        .split(area);

    draw_tree(f, app, chunks[0]);
    draw_preview(f, app, chunks[1]);
}

fn draw_tree(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let rows = &app.visible_rows;

    // Reserve 1 line for status bar at the bottom
    let list_height = area.height.saturating_sub(1) as usize;

    // Calculate scroll window (center selected item)
    let mut scroll_offset: usize = 0;
    if rows.len() > list_height {
        scroll_offset = app
            .selected_index
            .saturating_sub(list_height / 2);
        scroll_offset = scroll_offset.min(rows.len().saturating_sub(list_height));
    }

    let mut lines: Vec<Line> = Vec::new();

    // Header
    let header = format!(" {}", app.display_root());
    lines.push(Line::from(Span::styled(
        header,
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    )));

    // Separator
    lines.push(Line::from(Span::styled(
        "─".repeat(area.width as usize),
        Style::default().fg(Color::DarkGray),
    )));

    // File entries
    // Account for header (1) + separator (1) = 2 lines
    let entries_to_show = list_height.saturating_sub(2);
    let visible_end = (scroll_offset + entries_to_show).min(rows.len());

    for i in scroll_offset..visible_end {
        let row = &rows[i];
        let is_selected = i == app.selected_index;

        let indent = "  ".repeat(row.depth);
        let icon = if row.is_directory {
            if row.is_expanded {
                "▼ "
            } else {
                "▶ "
            }
        } else {
            "  "
        };
        let suffix = if row.is_directory { "/" } else { "" };
        let symlink = if row.is_symlink { " →" } else { "" };
        let text = format!(" {}{}{}{}{} ", indent, icon, row.name, suffix, symlink);

        let style = if is_selected {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else if row.is_directory {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        lines.push(Line::from(Span::styled(text, style)));
    }

    // Pad remaining space
    while lines.len() < (area.height.saturating_sub(1)) as usize {
        lines.push(Line::from(""));
    }

    // Status bar
    let status = format!(
        " {} items | j/k:nav l:expand h:collapse q:quit",
        rows.len()
    );
    lines.push(Line::from(Span::styled(
        status,
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

fn draw_preview(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default().borders(Borders::LEFT);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let (content, _total_lines) = &app.preview_cache;

    let text = match content {
        PreviewContent::Text(s) => s.clone(),
        PreviewContent::Directory(s) => s.clone(),
        PreviewContent::Binary(s) => s.clone(),
        PreviewContent::Empty => "(empty file)".to_string(),
        PreviewContent::Error(s) => s.clone(),
    };

    // Split into lines and apply scroll offset
    let all_lines: Vec<&str> = text.lines().collect();
    let scroll = app.preview_scroll.min(all_lines.len().saturating_sub(1));
    let visible_lines = &all_lines[scroll..];

    // Parse ANSI escape sequences into styled ratatui spans
    let lines: Vec<Line> = visible_lines
        .iter()
        .take(inner.height as usize)
        .map(|l| parse_ansi_line(l))
        .collect();

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

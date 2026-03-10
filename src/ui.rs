use crate::ansi::{parse_ansi_line, wrap_ansi_line};
use crate::app::App;
use crate::preview::PreviewContent;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    if app.select_mode {
        // Full-width preview only — so terminal selection doesn't cross panes
        draw_preview(f, app, area);
        return;
    }

    if app.diff_mode {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(35),
                Constraint::Percentage(35),
            ])
            .split(area);

        draw_tree(f, app, chunks[0]);
        draw_preview(f, app, chunks[1]);
        draw_diff(f, app, chunks[2]);
        return;
    }

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
    if app.select_mode {
        lines.push(Line::from(Span::styled(
            " SELECT MODE — highlight text with mouse, copy with Ctrl+Shift+C, press s to exit",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
    } else {
        let status = if app.diff_mode {
            format!(
                " {} items | j/k:nav D:diff-off d/u:scroll-diff q:quit",
                rows.len()
            )
        } else {
            format!(
                " {} items | j/k:nav l:expand h:collapse y:path D:diff s:select q:quit",
                rows.len()
            )
        };
        lines.push(Line::from(Span::styled(
            status,
            Style::default().fg(Color::DarkGray),
        )));
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

fn draw_diff(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default().borders(Borders::LEFT);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.diff_cache.is_empty() {
        return;
    }

    let all_lines: Vec<&str> = app.diff_cache.lines().collect();
    let scroll = app.diff_scroll.min(all_lines.len().saturating_sub(1));
    let visible = &all_lines[scroll..];
    let max_width = inner.width as usize;
    let display_height = inner.height as usize;

    let mut lines: Vec<Line> = Vec::new();
    let mut source_idx = 0;

    while lines.len() < display_height && source_idx < visible.len() {
        let raw = visible[source_idx];
        let style = if raw.starts_with("diff ") || raw.starts_with("index ") {
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        } else if raw.starts_with("--- ") || raw.starts_with("+++ ") {
            Style::default().add_modifier(Modifier::BOLD)
        } else if raw.starts_with("@@") {
            Style::default().fg(Color::Cyan)
        } else if raw.starts_with('+') {
            Style::default().fg(Color::Green)
        } else if raw.starts_with('-') {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        };

        let wrapped = wrap_ansi_line(raw, max_width);
        for wrapped_line in &wrapped {
            if lines.len() >= display_height {
                break;
            }
            lines.push(Line::from(Span::styled(wrapped_line.clone(), style)));
        }

        source_idx += 1;
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

fn draw_preview(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let inner = if app.select_mode {
        // No border in select mode — full width for clean selection
        area
    } else {
        let block = Block::default().borders(Borders::LEFT);
        let inner = block.inner(area);
        f.render_widget(block, area);
        inner
    };

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

    // Determine gutter width for line numbers
    let show_nums = app.show_line_numbers && matches!(content, PreviewContent::Text(_));
    let gutter_width = if show_nums {
        format!("{}", all_lines.len()).len() + 1 // digits + 1 space
    } else {
        0
    };

    let content_width = (inner.width as usize).saturating_sub(gutter_width);
    let display_height = inner.height as usize;

    // Wrap and parse lines, filling up to the display height
    let mut lines: Vec<Line> = Vec::new();
    let mut source_idx = 0;

    while lines.len() < display_height && source_idx < visible_lines.len() {
        let raw = visible_lines[source_idx];
        let wrapped = wrap_ansi_line(raw, content_width);

        for (wrap_idx, wrapped_line) in wrapped.iter().enumerate() {
            if lines.len() >= display_height {
                break;
            }
            let parsed = parse_ansi_line(wrapped_line);
            if gutter_width > 0 {
                let mut spans = if wrap_idx == 0 {
                    // First wrapped segment gets the line number
                    let line_num = scroll + source_idx + 1;
                    let num_str =
                        format!("{:>width$} ", line_num, width = gutter_width - 1);
                    vec![Span::styled(num_str, Style::default().fg(Color::DarkGray))]
                } else {
                    // Continuation lines get blank gutter
                    vec![Span::styled(
                        " ".repeat(gutter_width),
                        Style::default().fg(Color::DarkGray),
                    )]
                };
                spans.extend(parsed.spans);
                lines.push(Line::from(spans));
            } else {
                lines.push(parsed);
            }
        }

        source_idx += 1;
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Parse a string containing ANSI escape sequences into a ratatui Line
pub fn parse_ansi_line(input: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_style = Style::default();
    let mut buf = String::new();

    // Work with bytes for ESC detection but use char boundaries for text
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'[' {
            // Flush text before this escape
            if !buf.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut buf), current_style));
            }

            // Parse CSI sequence: ESC [ params m
            i += 2; // skip ESC [
            let mut params: Vec<u16> = Vec::new();
            let mut num: Option<u16> = None;

            while i < len {
                let b = bytes[i];
                if b.is_ascii_digit() {
                    num = Some(num.unwrap_or(0) * 10 + (b - b'0') as u16);
                    i += 1;
                } else if b == b';' {
                    params.push(num.unwrap_or(0));
                    num = None;
                    i += 1;
                } else if b == b'm' {
                    params.push(num.unwrap_or(0));
                    i += 1;
                    break;
                } else {
                    // Unknown terminator, skip
                    i += 1;
                    break;
                }
            }

            apply_sgr_params(&params, &mut current_style);
        } else {
            // Decode the UTF-8 character starting at byte position i
            let ch = &input[i..];
            if let Some(c) = ch.chars().next() {
                buf.push(c);
                i += c.len_utf8();
            } else {
                i += 1;
            }
        }
    }

    // Flush remaining text
    if !buf.is_empty() {
        spans.push(Span::styled(buf, current_style));
    }

    Line::from(spans)
}

fn apply_sgr_params(params: &[u16], style: &mut Style) {
    let mut pi = 0;
    while pi < params.len() {
        match params[pi] {
            0 => *style = Style::default(),
            1 => *style = style.add_modifier(Modifier::BOLD),
            2 => *style = style.add_modifier(Modifier::DIM),
            3 => *style = style.add_modifier(Modifier::ITALIC),
            4 => *style = style.add_modifier(Modifier::UNDERLINED),
            7 => *style = style.add_modifier(Modifier::REVERSED),
            9 => *style = style.add_modifier(Modifier::CROSSED_OUT),
            22 => *style = style.remove_modifier(Modifier::BOLD | Modifier::DIM),
            23 => *style = style.remove_modifier(Modifier::ITALIC),
            24 => *style = style.remove_modifier(Modifier::UNDERLINED),
            27 => *style = style.remove_modifier(Modifier::REVERSED),
            30..=37 => *style = style.fg(basic_color(params[pi] - 30)),
            40..=47 => *style = style.bg(basic_color(params[pi] - 40)),
            90..=97 => *style = style.fg(bright_color(params[pi] - 90)),
            100..=107 => *style = style.bg(bright_color(params[pi] - 100)),
            38 => {
                if pi + 1 < params.len() {
                    if params[pi + 1] == 5 && pi + 2 < params.len() {
                        *style = style.fg(Color::Indexed(params[pi + 2] as u8));
                        pi += 2;
                    } else if params[pi + 1] == 2 && pi + 4 < params.len() {
                        *style = style.fg(Color::Rgb(
                            params[pi + 2] as u8,
                            params[pi + 3] as u8,
                            params[pi + 4] as u8,
                        ));
                        pi += 4;
                    }
                }
            }
            48 => {
                if pi + 1 < params.len() {
                    if params[pi + 1] == 5 && pi + 2 < params.len() {
                        *style = style.bg(Color::Indexed(params[pi + 2] as u8));
                        pi += 2;
                    } else if params[pi + 1] == 2 && pi + 4 < params.len() {
                        *style = style.bg(Color::Rgb(
                            params[pi + 2] as u8,
                            params[pi + 3] as u8,
                            params[pi + 4] as u8,
                        ));
                        pi += 4;
                    }
                }
            }
            39 => *style = style.fg(Color::Reset),
            49 => *style = style.bg(Color::Reset),
            _ => {}
        }
        pi += 1;
    }
}

fn basic_color(n: u16) -> Color {
    match n {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::White,
        _ => Color::Reset,
    }
}

fn bright_color(n: u16) -> Color {
    match n {
        0 => Color::DarkGray,
        1 => Color::LightRed,
        2 => Color::LightGreen,
        3 => Color::LightYellow,
        4 => Color::LightBlue,
        5 => Color::LightMagenta,
        6 => Color::LightCyan,
        7 => Color::White,
        _ => Color::Reset,
    }
}

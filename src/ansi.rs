use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Parse a string containing ANSI escape sequences into a ratatui Line
pub fn parse_ansi_line(input: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_style = Style::default();
    let mut buf = String::new();
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

            // Apply SGR parameters
            let mut pi = 0;
            while pi < params.len() {
                match params[pi] {
                    0 => current_style = Style::default(),
                    1 => current_style = current_style.add_modifier(Modifier::BOLD),
                    2 => current_style = current_style.add_modifier(Modifier::DIM),
                    3 => current_style = current_style.add_modifier(Modifier::ITALIC),
                    4 => current_style = current_style.add_modifier(Modifier::UNDERLINED),
                    7 => current_style = current_style.add_modifier(Modifier::REVERSED),
                    9 => current_style = current_style.add_modifier(Modifier::CROSSED_OUT),
                    22 => {
                        current_style =
                            current_style.remove_modifier(Modifier::BOLD | Modifier::DIM)
                    }
                    23 => current_style = current_style.remove_modifier(Modifier::ITALIC),
                    24 => current_style = current_style.remove_modifier(Modifier::UNDERLINED),
                    27 => current_style = current_style.remove_modifier(Modifier::REVERSED),
                    // Basic foreground colors
                    30..=37 => {
                        current_style = current_style.fg(basic_color(params[pi] - 30));
                    }
                    // Basic background colors
                    40..=47 => {
                        current_style = current_style.bg(basic_color(params[pi] - 40));
                    }
                    // Bright foreground
                    90..=97 => {
                        current_style = current_style.fg(bright_color(params[pi] - 90));
                    }
                    // Bright background
                    100..=107 => {
                        current_style = current_style.bg(bright_color(params[pi] - 100));
                    }
                    // 256-color or 24-bit foreground
                    38 => {
                        if pi + 1 < params.len() {
                            if params[pi + 1] == 5 && pi + 2 < params.len() {
                                current_style =
                                    current_style.fg(Color::Indexed(params[pi + 2] as u8));
                                pi += 2;
                            } else if params[pi + 1] == 2 && pi + 4 < params.len() {
                                current_style = current_style.fg(Color::Rgb(
                                    params[pi + 2] as u8,
                                    params[pi + 3] as u8,
                                    params[pi + 4] as u8,
                                ));
                                pi += 4;
                            }
                        }
                    }
                    // 256-color or 24-bit background
                    48 => {
                        if pi + 1 < params.len() {
                            if params[pi + 1] == 5 && pi + 2 < params.len() {
                                current_style =
                                    current_style.bg(Color::Indexed(params[pi + 2] as u8));
                                pi += 2;
                            } else if params[pi + 1] == 2 && pi + 4 < params.len() {
                                current_style = current_style.bg(Color::Rgb(
                                    params[pi + 2] as u8,
                                    params[pi + 3] as u8,
                                    params[pi + 4] as u8,
                                ));
                                pi += 4;
                            }
                        }
                    }
                    39 => current_style = current_style.fg(Color::Reset),
                    49 => current_style = current_style.bg(Color::Reset),
                    _ => {}
                }
                pi += 1;
            }
        } else {
            buf.push(bytes[i] as char);
            i += 1;
        }
    }

    // Flush remaining text
    if !buf.is_empty() {
        spans.push(Span::styled(buf, current_style));
    }

    Line::from(spans)
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

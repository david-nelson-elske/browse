mod app;
mod preview;
mod tree;
mod ui;

use std::io;
use std::path::PathBuf;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
    MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;

fn main() -> io::Result<()> {
    // Parse optional path argument
    let root_path = std::env::args()
        .nth(1)
        .map(|p| {
            let path = PathBuf::from(&p);
            if path.is_absolute() {
                path
            } else {
                std::env::current_dir().unwrap_or_default().join(path)
            }
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let root_path = root_path.canonicalize().unwrap_or(root_path);

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(root_path);

    // Main loop
    loop {
        let area_height = terminal.size()?.height;

        terminal.draw(|f| ui::draw(f, &app))?;

        if app.should_quit {
            break;
        }

        match event::read()? {
            Event::Key(key) => match (key.code, key.modifiers) {
                (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    app.should_quit = true;
                }
                (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
                    app.move_down();
                }
                (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
                    app.move_up();
                }
                (KeyCode::Char('l'), _) | (KeyCode::Right, _) | (KeyCode::Enter, _) => {
                    app.toggle_expand();
                }
                (KeyCode::Char('h'), _) | (KeyCode::Left, _) => {
                    app.collapse_or_parent();
                }
                (KeyCode::Char('g'), _) => {
                    app.jump_top();
                }
                (KeyCode::Char('G'), _) => {
                    app.jump_bottom();
                }
                (KeyCode::Char('.'), _) => {
                    app.toggle_hidden();
                }
                (KeyCode::Char('J'), _) => {
                    app.scroll_preview_down(1);
                }
                (KeyCode::Char('K'), _) => {
                    app.scroll_preview_up(1);
                }
                (KeyCode::Char('d'), _) => {
                    let half = (area_height / 2) as usize;
                    app.scroll_preview_down(half);
                }
                (KeyCode::Char('u'), _) => {
                    let half = (area_height / 2) as usize;
                    app.scroll_preview_up(half);
                }
                _ => {}
            },
            Event::Mouse(mouse) => {
                let tree_width = terminal.size()?.width * 35 / 100;
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        if mouse.column < tree_width {
                            app.click_tree(mouse.row, area_height);
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        if mouse.column >= tree_width {
                            app.scroll_preview_up(3);
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if mouse.column >= tree_width {
                            app.scroll_preview_down(3);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

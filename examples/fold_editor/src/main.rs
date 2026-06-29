use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Position},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_code_editor::{editor::Editor, theme::vesper, utils::get_lang};
use std::io::stdout;

// Run with: cargo run --release -p fold_editor -- path/to/file.rs
const SAMPLE: &str = r#"fn main() {
    let config = Config {
        host: "localhost",
        port: 8080,
    };

    match config.port {
        8080 => println!("default port"),
        _ => println!("custom port"),
    }
}

struct Config {
    host: &'static str,
    port: u16,
}
"#;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (language, content) = if let Some(path) = args.get(1) {
        (get_lang(path), std::fs::read_to_string(path)?)
    } else {
        ("rust".to_string(), SAMPLE.to_string())
    };

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut editor = Editor::new(&language, &content, vesper())?;
    let mut editor_area = ratatui::layout::Rect::default();

    let result = loop {
        terminal.draw(|frame| {
            let [help, area] =
                Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(frame.area());
            frame.render_widget(
                Paragraph::new("Ctrl+F — fold/unfold block at cursor · arrows — move · Esc — exit"),
                help,
            );
            let inner = Block::default().borders(Borders::ALL).inner(area);
            editor_area = inner;
            frame.render_widget(Block::default().borders(Borders::ALL), area);
            frame.render_widget(&editor, inner);
            if let Some((x, y)) = editor.get_visible_cursor(&inner) {
                frame.set_cursor_position(Position::new(x, y));
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.code == KeyCode::Esc {
                        break Ok(());
                    } else if is_fold_toggle_pressed(key) {
                        editor.toggle_fold_at_cursor();
                        editor.focus(&editor_area);
                    } else {
                        editor.input(key, &editor_area)?;
                    }
                }
                Event::Mouse(mouse) => editor.mouse(mouse, &editor_area)?,
                _ => {}
            }
        }
    };

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    result
}

fn is_fold_toggle_pressed(key: crossterm::event::KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f')
}

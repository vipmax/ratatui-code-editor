use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{
        enable_raw_mode, disable_raw_mode, 
        EnterAlternateScreen, LeaveAlternateScreen
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;

mod editor;
use editor::Editor;

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    
    let backend = CrosstermBackend::new(stdout());
    let mut term = Terminal::new(backend)?;
    
    let content = std::fs::read_to_string("test.rs")?;

    let mut editor = Editor::new("main.rs", &content);

    term.draw(|f| {
        f.render_widget(&editor, f.area());
    })?;

    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            let event = event::read()?;

            if let Event::Key(key) = event {
                match key.code {
                    KeyCode::Esc => break,
                    _ => {
                        editor.input(key, term.size()?.height as usize);
                    }
                }

                term.draw(|f| {
                    f.render_widget(&editor, f.area());
                })?;
            }
        }
    }

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

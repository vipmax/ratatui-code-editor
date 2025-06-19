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
    let mut terminal = Terminal::new(backend)?;
    
    let filename = "test.rs";
    
    let content = std::fs::read_to_string(filename)?;

    let mut editor = Editor::new(filename, &content);

    terminal.draw(|f| {
        f.render_widget(&editor, f.area());
    })?;

    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.code == KeyCode::Esc {
                        break;
                    } else {
                        editor.input(key, terminal.size()?.height as usize);
                    }
                }
                Event::Resize(new_width, new_height) => {

                }
                _ => {}
            }

            terminal.draw(|f| {
                f.render_widget(&editor, f.area());
            })?;
        }
    }
    
    // editor.input(key, term.size()?.height as usize);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{
        enable_raw_mode, disable_raw_mode, 
        EnterAlternateScreen, LeaveAlternateScreen
    },
};
use ratatui::{Terminal, backend::CrosstermBackend, layout::{Position}};
use ratatui_code_editor::editor::Editor;
use ratatui_code_editor::theme::vesper;
use std::io::stdout;

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    
    let filename = String::from("test.rs");
    let content = "fn main() {\n    println!(\"Hello, world!\");\n}";
    let mut editor = Editor::new("rust", content, vesper())?;
    let mut editor_area = ratatui::layout::Rect::default();

    editor.set_change_callback(Box::new(
        move |changes| {
            for (start_row, start_col, end_row, end_col, text) in changes {
                println!(
                    "Edit {}: ({}, {}) -> ({}, {}) text: '{}'",
                    filename, start_row, start_col, end_row, end_col, text
                );
            }
        }
    ));
    
    loop {
        terminal.draw(|f| {
            let area = f.area();
            editor_area = area;
            f.render_widget(&editor, editor_area);

            let cursor = editor.get_visible_cursor(&area);
            if let Some((x,y)) = cursor {
                f.set_cursor_position(Position::new(x, y));
            }
        })?;
        
        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Esc {
                break;
            }
            editor.input(key, &editor_area)?;
        }
    }
    
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;    
    Ok(())
}

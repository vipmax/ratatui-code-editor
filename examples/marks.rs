use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{
        enable_raw_mode, disable_raw_mode, 
        EnterAlternateScreen, LeaveAlternateScreen
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};
use ratatui_code_editor::editor::Editor;
use ratatui_code_editor::theme::vesper;
use std::io::stdout;

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    
    let content = "fn main() {\n    println!(\"Hello, world!\");\n}";
    let mut editor = Editor::new("rust", content, vesper());
    let mut editor_area = ratatui::layout::Rect::default();

    let marks = vec![
        (3, 7, "#b1fce5"),
        (16, 24, "#f6c99f"),
    ];

    editor.set_marks(marks);
    
    loop {
        terminal.draw(|f| {
            let area = f.area();
            editor_area = area;
            f.render_widget(&editor, editor_area);
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
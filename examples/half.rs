use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, 
        Event, KeyCode
    },
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, 
        disable_raw_mode, enable_raw_mode
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};
use ratatui::layout::{Layout, Constraint, Direction, Position};
use ratatui::widgets::{Block, Borders};
use std::io::stdout;
use ratatui_code_editor::editor::Editor;
use ratatui_code_editor::theme::vesper;

fn main() -> anyhow::Result<()> {
    let filename = "src/code.rs";
    let language = "rust";
    let content = std::fs::read_to_string(filename)?;    
    
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    execute!(stdout(), EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    
    let theme = vesper();

    let mut editor = Editor::new(&language, &content, theme);

    let mut editor_area = ratatui::layout::Rect::default(); 

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), 
            Constraint::Percentage(50)
        ].as_ref());

    loop {
        terminal.draw(|f| {    
            let block = Block::default()
                .title(" Editor ")
                .borders(Borders::ALL);
            let chunks = layout.split(f.area());
            let inner = block.inner(chunks[0]);
            editor_area = inner;
            f.render_widget(block, chunks[0]);
            f.render_widget(&editor, editor_area);
            
            let cursor = editor.get_visible_cursor(&editor_area);
            if let Some((x,y)) = cursor {
                f.set_cursor_position(Position::new(x, y));
            }
        })?;
        
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.code == KeyCode::Esc {
                        break;
                    } else {
                        editor.input(key, &editor_area)?;
                    }
                }
                Event::Mouse(mouse) => {
                    editor.mouse(mouse, &editor_area)?;
                },
                Event::Resize(_, _) => { }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}
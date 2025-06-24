use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, 
        Event, KeyCode, KeyModifiers, KeyEvent
    },
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, 
        disable_raw_mode, enable_raw_mode
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};
use ratatui::layout::{Layout, Constraint, Direction};
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
        })?;
        
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.code == KeyCode::Esc {
                        break;
                    } else if is_save_pressed(key) {
                        let content = editor.get_content();
                        save_to_file(&content, filename)?;
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

fn save_to_file(content: &str, path: &str) -> anyhow::Result<()> {
    use std::io::Write;
    
    let mut file = std::fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

fn is_save_pressed(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) &&
        key.code == KeyCode::Char('s')
}
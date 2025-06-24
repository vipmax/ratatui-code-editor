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
use std::io::stdout;
use ratatui_code_editor::editor::Editor;
use ratatui_code_editor::theme::vesper;
use ratatui_code_editor::utils::get_lang;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    let filename = if args.len() > 1 {
        &args[1]
    } else {
        eprintln!("Usage: cargo run --release --example main <filename>");
        return Ok(());
    };
    
    let language = get_lang(filename);
    let content = std::fs::read_to_string(filename)?;

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    execute!(stdout(), EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    
    let theme = vesper();

    let mut editor = Editor::new(&language, &content, theme);
    let mut editor_area = ratatui::layout::Rect::default(); 

    loop {

        terminal.draw(|f| {
            let area = f.area();
            editor_area = area;
            f.render_widget(&editor, area);
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
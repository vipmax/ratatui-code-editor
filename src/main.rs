use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::stdout;

mod editor;
use editor::Editor;

mod code;

fn get_theme() -> Vec<(&'static str, &'static str)> {
    vec![
        ("identifier", "#A5FCB6"),
        ("field_identifier", "#A5FCB6"),
        ("property_identifier", "#A5FCB6"),
        ("property", "#A5FCB6"),
        ("string", "#b1fce5"),
        ("keyword", "#a0a0a0"),
        ("constant", "#f6c99f"),
        ("number", "#f6c99f"),
        ("integer", "#f6c99f"),
        ("float", "#f6c99f"),
        ("variable", "#ffffff"),
        ("variable.builtin", "#ffffff"),
        ("function", "#f6c99f"),
        ("function.call", "#f6c99f"),
        ("method", "#f6c99f"),
        ("macro", "#f6c99f"),
        ("comment", "#585858"),
        ("namespace", "#f6c99f"),
        ("type", "#f6c99f"),
        ("type.builtin", "#f6c99f"),
        ("tag.attribute", "#c6a5fc"),
        ("tag", "#c6a5fc"),
        ("error", "#A5FCB6"),
    ]
}

fn get_lang(filename: &str) -> String {

    let extension = std::path::Path::new(filename).extension()
        .and_then(|ext| ext.to_str()).unwrap_or("");

    match extension {
        "rs" => "rust",
        "js" => "javascript",
        "ts" => "typescript",
        "jsx" => "javascript",
        "tsx" => "typescript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "cpp"  => "cpp",
        "c" => "c",
        "html" => "html",
        "css" => "css",
        "json" => "json",
        "toml" => "toml",
        _ => "unknown",
    }
    .to_string()
}


fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filename = args.get(1).unwrap_or_else(|| {
        eprintln!("Usage: editor <filename>");
        std::process::exit(1);
    });
    
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    execute!(stdout(), EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;


    
    let language = get_lang(filename);

    let content = std::fs::read_to_string(filename)?;
    
    let h = terminal.size()?.height as usize;
    let w = terminal.size()?.width as usize;
    
    let theme = get_theme();

    let mut editor = Editor::new(filename, &language, &content, w, h, theme);

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
                        editor.input(key)?;
                    }
                }
                Event::Mouse(mouse) => {
                    editor.mouse(mouse, &mut terminal)?;
                },
                Event::Resize(c, r) => {
                    editor.resize(c, r);
                }
                _ => {}
            }

            terminal.draw(|f| {
                f.render_widget(&editor, f.area());
            })?;
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

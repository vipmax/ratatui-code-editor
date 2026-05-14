use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use git2::Repository;
use ratatui::{backend::CrosstermBackend, layout::Position, Terminal};
use ratatui_code_editor::editor::Editor;
use ratatui_code_editor::theme::vesper;
use ratatui_code_editor::utils::get_lang;
use std::io::stdout;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let filename = if args.len() > 1 {
        &args[1]
    } else {
        eprintln!("Usage: cargo run --release -p diff_editor -- <filename>");
        return Ok(());
    };

    let language = get_lang(filename);
    let content = std::fs::read_to_string(filename)?;

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    execute!(stdout(), EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut editor = Editor::new(&language, &content, vesper())?;
    editor.set_diff_enabled(true);
    if let Some(original) = read_file_from_head(filename)? {
        editor.set_original_code(&original)?;
    } else {
        eprintln!("diff_editor: original from git HEAD not found, diff view disabled for this file");
    }

    let mut editor_area = ratatui::layout::Rect::default();

    loop {
        terminal.draw(|f| {
            let area = f.area();
            editor_area = area;
            f.render_widget(&editor, area);

            if let Some((x, y)) = editor.get_visible_cursor(&area) {
                f.set_cursor_position(Position::new(x, y));
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.code == KeyCode::Esc {
                        break;
                    } else if is_save_pressed(key) {
                        save_to_file(&editor.get_content(), filename)?;
                    } else {
                        editor.input(key, &editor_area)?;
                    }
                }
                Event::Mouse(mouse) => {
                    editor.mouse(mouse, &editor_area)?;
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn read_file_from_head(path: &str) -> Result<Option<String>> {
    let try_read = || -> Result<String> {
        let abs_path = std::fs::canonicalize(path)?;
        let repo = Repository::discover(&abs_path)?;
        let workdir = repo.workdir().ok_or_else(|| anyhow::anyhow!("bare repo"))?;
        let rel_path = abs_path.strip_prefix(workdir)?;
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        let tree = commit.tree()?;
        let entry = tree.get_path(rel_path)?;
        let object = entry.to_object(&repo)?;
        let blob = object.as_blob().ok_or_else(|| anyhow::anyhow!("not a blob"))?;
        let text = std::str::from_utf8(blob.content())?.to_string();
        Ok(text)
    };

    Ok(try_read().ok())
}

fn save_to_file(content: &str, path: &str) -> Result<()> {
    use std::io::Write;

    let mut file = std::fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

fn is_save_pressed(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s')
}

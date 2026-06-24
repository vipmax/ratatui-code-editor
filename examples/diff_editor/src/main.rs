use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use git2::Repository;
use ratatui::{Terminal, backend::CrosstermBackend, layout::Position};
use ratatui_code_editor::editor::Editor;
use ratatui_code_editor::theme::vesper;
use ratatui_code_editor::utils::get_lang;
use std::io::stdout;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EditMode {
    Plain,
    Diff,
    Combine,
}

impl EditMode {
    fn next(self) -> Self {
        match self {
            EditMode::Plain => EditMode::Diff,
            EditMode::Diff => EditMode::Combine,
            EditMode::Combine => EditMode::Plain,
        }
    }
}

// cargo run --release -p diff_editor -- src/editor.rs
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
    let mut edit_mode = EditMode::Combine;
    apply_mode(&mut editor, edit_mode);
    if let Some(original) = read_file_from_head(filename)? {
        editor.set_original_code(&original)?;
    } else {
        eprintln!(
            "diff_editor: original from git HEAD not found, diff view disabled for this file"
        );
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
                    } else if is_cycle_edit_mode_pressed(key) {
                        edit_mode = edit_mode.next();
                        apply_mode(&mut editor, edit_mode);
                        editor.focus(&editor_area);
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
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
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
        let blob = object
            .as_blob()
            .ok_or_else(|| anyhow::anyhow!("not a blob"))?;
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

fn is_cycle_edit_mode_pressed(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f')
}

fn apply_mode(editor: &mut Editor, mode: EditMode) {
    match mode {
        EditMode::Plain => {
            editor.set_diff_enabled(false);
            editor.set_diff_focus_enabled(false);
        }
        EditMode::Diff => {
            editor.set_diff_enabled(true);
            editor.set_diff_focus_context(3);
            editor.set_diff_focus_enabled(true);
        }
        EditMode::Combine => {
            editor.set_diff_enabled(true);
            editor.set_diff_focus_enabled(false);
        }
    }
}

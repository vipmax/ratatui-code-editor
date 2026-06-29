use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend, layout::Position};
use ratatui_code_editor::editor::Editor;
use ratatui_code_editor::theme::vesper;
use std::io::stdout;

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    execute!(stdout(), EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let original = r#"const LANGUAGE_EXTENSIONS = {
  'h': 'c',
  'zig': 'zig',
  'lua': 'lua',
  'md': 'text',
  'markdown': 'text',
  'mdx': 'text'
};

export const getLanguageFromFileName = (fileName: string): string => {
  const ext = fileName.split('.').pop()?.toLowerCase();
  return LANGUAGE_EXTENSIONS[ext || ''] || 'javascript';
};"#;

    let changed = r#"const LANGUAGE_EXTENSIONS = {
  'h': 'c',
  'zig': 'zig',
  'lua': 'lua',
  'md': 'text',
  'markdown': 'text',
  'mdx': 'text'
};

export const getLanguageFromFileName = (fileName: string): string => {
  const ext = fileName.split('.').pop()?.toLowerCase();
  return LANGUAGE_EXTENSIONS[ext || ''] || 'text';
};"#;

    let mut editor = Editor::new("typescript", changed, vesper())?;
    editor.set_diff_enabled(true);
    editor.set_original_code(original)?;
    let mut editor_area = ratatui::layout::Rect::default();

    loop {
        terminal.draw(|f| {
            let area = f.area();
            editor_area = area;
            f.render_widget(&editor, editor_area);

            if let Some((x, y)) = editor.get_visible_cursor(&area) {
                f.set_cursor_position(Position::new(x, y));
            }
        })?;

        match event::read()? {
            Event::Key(key) => {
                if key.code == KeyCode::Esc {
                    break;
                }
                editor.input(key, &editor_area)?;
            }
            Event::Mouse(mouse) => {
                editor.mouse(mouse, &editor_area)?;
            }
            Event::Resize(_, _) => {}
            _ => {}
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

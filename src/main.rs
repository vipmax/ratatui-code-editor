use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::stdout;

mod editor;
use editor::Editor;

mod code;

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    execute!(stdout(), EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let filename = "test/test.ts";
    let language = "typescript";

    let content = std::fs::read_to_string(filename).expect("Failed to read file");

    let mut editor = Editor::new(filename, language, &content);

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
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        editor.scroll_up();
                    }
                    MouseEventKind::ScrollDown => {
                        editor.scroll_down(terminal.size()?.height as usize);
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        let area = terminal.get_frame().area();
                        editor.click(mouse.column, mouse.row, area);
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {}
                _ => {}
            }

            terminal.draw(|f| {
                f.render_widget(&editor, f.area());
            })?;
        }
    }

    // editor.input(key, term.size()?.height as usize);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}

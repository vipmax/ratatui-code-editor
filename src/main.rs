use crossterm::{
    event::{
        self, Event, KeyCode,
        EnableMouseCapture, DisableMouseCapture,
        MouseEventKind, MouseButton
    },
    execute,
    terminal::{
        enable_raw_mode, disable_raw_mode, 
        EnterAlternateScreen, LeaveAlternateScreen
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;

mod editor;
use editor::Editor;

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    execute!(stdout(), EnableMouseCapture)?;
    
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    
    let filename = "test.rs";
    
    let content = std::fs::read_to_string(filename)?;

    let mut editor = Editor::new(filename, &content);

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
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            editor.scroll_up();
                        },
                        MouseEventKind::ScrollDown => {
                            editor.scroll_down(terminal.size()?.height as usize);
                        },
                        MouseEventKind::Down(MouseButton::Left) => {
                            let area = terminal.get_frame().area();
                            editor.click(mouse.column, mouse.row, area);
                        }
                        _ => {}
                    }
                },
                Event::Resize(_, _) => { },
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

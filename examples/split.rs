use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, 
        Event, KeyCode,
    },
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, 
        disable_raw_mode, enable_raw_mode
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};
use ratatui::layout::{Layout, Constraint, Direction, Rect, Position};
use ratatui::widgets::{Block, Borders};
use crossterm::event::MouseEvent;
use std::io::stdout;
use ratatui_code_editor::editor::Editor;
use ratatui_code_editor::theme::vesper;

fn main() -> anyhow::Result<()> {
    let filename1 = "src/code.rs";
    let filename2 = "src/editor.rs";
    let language = "rust";
    let content1 = std::fs::read_to_string(filename1).unwrap_or_default();
    let content2 = std::fs::read_to_string(filename2).unwrap_or_default();

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    execute!(stdout(), EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    
    let theme = vesper();

    let mut editor1 = Editor::new(&language, &content1, theme.clone())?;
    let mut editor2 = Editor::new(&language, &content2, theme)?;

    let mut editor1_area = ratatui::layout::Rect::default(); 
    let mut editor2_area = ratatui::layout::Rect::default(); 

    let mut active_editor = 0;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(50)
                ])
                .split(f.area());

            let block1 = Block::default()
                .title(filename1)
                .borders(Borders::ALL);
            let block2 = Block::default()
                .title(filename2)
                .borders(Borders::ALL);

            editor1_area = block1.inner(chunks[0]);
            editor2_area = block2.inner(chunks[1]);

            f.render_widget(block1, chunks[0]);
            f.render_widget(block2, chunks[1]);
            f.render_widget(&editor1, editor1_area);
            f.render_widget(&editor2, editor2_area);
            
            let cursor = match active_editor {
                0 => editor1.get_visible_cursor(&editor1_area),
                _ => editor2.get_visible_cursor(&editor2_area),
            };
            
            if let Some((x,y)) = cursor {
                f.set_cursor_position(Position::new(x, y));
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.code == KeyCode::Esc {
                        break;
                    } else if key.code == KeyCode::Tab {
                        active_editor = (active_editor + 1) % 2;
                    } else {
                        match active_editor {
                            0 => editor1.input(key, &editor1_area)?,
                            1 => editor2.input(key, &editor2_area)?,
                            _ => {}
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    if let Some(new_active) = detect_active_editor(&mouse, editor1_area, editor2_area) {
                        active_editor = new_active;
                    }

                    match active_editor {
                        0 => editor1.mouse(mouse, &editor1_area)?,
                        1 => editor2.mouse(mouse, &editor2_area)?,
                        _ => {}
                    }
                },

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

fn detect_active_editor(
    mouse: &MouseEvent, 
    editor1_area: Rect, 
    editor2_area: Rect
) -> Option<usize> {
    let x = mouse.column;
    let y = mouse.row;

    if rect_contains(editor1_area, x, y) {
        Some(0)
    } else if rect_contains(editor2_area, x, y) {
        Some(1)
    } else {
        None
    }
}

fn rect_contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x &&
    x < rect.x + rect.width &&
    y >= rect.y &&
    y < rect.y + rect.height
}

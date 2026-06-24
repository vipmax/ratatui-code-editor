use crate::actions::*;
use crate::editor::Editor;
use crate::selection::SelectionSnap;
use anyhow::Result;
use crossterm::event::{
    KeyEvent, KeyModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};
use ratatui_core::layout::Rect;

impl Editor {
    pub fn input(
        &mut self, key: KeyEvent, area: &Rect,
    ) -> Result<()> {
        use crossterm::event::KeyCode;

        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let _alt = key.modifiers.contains(KeyModifiers::ALT);

        match key.code {
            KeyCode::Char('÷') => self.apply(ToggleComment { }),
            KeyCode::Char('z') if ctrl => self.apply(Undo { }),
            KeyCode::Char('y') if ctrl => self.apply(Redo { }),
            KeyCode::Char('c') if ctrl => self.apply(Copy { }),
            KeyCode::Char('v') if ctrl => self.apply(Paste { }),
            KeyCode::Char('x') if ctrl => self.apply(Cut { }),
            KeyCode::Char('k') if ctrl => self.apply(DeleteLine { }),
            KeyCode::Char('d') if ctrl => self.apply(Duplicate { }),
            KeyCode::Char('a') if ctrl => self.apply(SelectAll { }),
            KeyCode::Left      => self.apply(MoveLeft { shift }),
            KeyCode::Right     => self.apply(MoveRight { shift }),
            KeyCode::Up        => self.apply(MoveUp { shift }),
            KeyCode::Down      => self.apply(MoveDown { shift }),
            KeyCode::Backspace => self.apply(Delete { }),
            KeyCode::Enter     => self.apply(InsertNewline { }),
            KeyCode::Char(c)   => self.apply(InsertText { text: c.to_string() }),
            KeyCode::Tab       => self.apply(Indent { }),
            KeyCode::BackTab   => self.apply(UnIndent { }),
            _ => {}
        }
        self.focus(&area);
        Ok(())
    }
    
    pub fn mouse(
        &mut self, mouse: MouseEvent, area: &Rect,
    ) -> Result<()> {

        match mouse.kind {
            MouseEventKind::ScrollUp => self.scroll_up(),
            MouseEventKind::ScrollDown => self.scroll_down(area.height as usize),
            MouseEventKind::Down(MouseButton::Left) => {
                if self.is_diff_focus_active() &&
                    self.expand_hidden_diff_at_mouse(mouse.column, mouse.row, area) {
                    return Ok(());
                }
                let pos = self.cursor_from_mouse(mouse.column, mouse.row, area);
                if let Some(cursor) = pos {
                    self.handle_mouse_down(cursor);
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                // Auto-scroll when dragging on the last or first visible row
                if mouse.row == area.top() {
                    self.scroll_up();
                }
                if mouse.row == area.bottom().saturating_sub(1) {
                    self.scroll_down(area.height as usize);
                }
                let pos = self.cursor_from_mouse(mouse.column, mouse.row, area);
                if let Some(cursor) = pos {
                    self.handle_mouse_drag(cursor);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.selection_snap = SelectionSnap::None;
            }
            _ => {}
        }
        Ok(())
    }
}

use crossterm::event::{
    KeyEvent, KeyModifiers,
    MouseEvent, MouseEventKind, MouseButton,
};
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::prelude::*;
use std::time::Duration;
use crate::click::{ClickKind, ClickTracker};
use crate::code::Code;
use crate::code::{EditKind, EditBatch};
use crate::code::{RopeGraphemes, grapheme_width_and_chars_len, grapheme_width};
use crate::selection::{Selection, SelectionSnap};
use crate::actions::*;
use crate::utils;
use std::collections::HashMap;
use std::cell::RefCell;
use std::cmp::Ordering;
use anyhow::{Result, anyhow};

// keyword and ratatui style
type Theme = HashMap<String, Style>;
// start byte, end byte, style
type Hightlight = (usize, usize, Style);
// start offset, end offset
type HightlightCache = HashMap<(usize, usize), Vec<Hightlight>>;

/// Represents the text editor, which holds the code buffer, cursor, selection,
/// theme, scroll offsets, highlight cache, clipboard, and user mark intervals.
pub struct Editor {
    /// Code buffer and editing/highlighting logic for the current language
    pub(crate) code: Code,
    /// Current cursor position as a character index in the document
    pub(crate) cursor: usize,

    /// Vertical scroll offset: index of the first visible line
    pub(crate) offset_y: usize,

    /// Horizontal scroll offset in characters (visual columns)
    pub(crate) offset_x: usize,

    /// Syntax theme: mapping of token name to ratatui Style
    pub(crate) theme: Theme,

    /// Current text selection, if any
    pub(crate) selection: Option<Selection>,

    /// Click tracker to detect single/double/triple clicks
    pub(crate) clicks: ClickTracker,

    /// Selection snapping mode (to word, to line, or none)
    pub(crate) selection_snap: SelectionSnap,

    /// Fallback clipboard storage when the system clipboard is unavailable
    pub(crate) clipboard: Option<String>,

    /// User marks for intervals: (start, end, color)
    pub(crate) marks: Option<Vec<(usize, usize, Color)>>,

    /// Syntax highlight cache by intervals to speed up rendering
    pub(crate) highlights_cache: RefCell<HightlightCache>,
}

impl Editor {
    pub fn new(lang: &str, text: &str, theme: Vec<(&str, &str)>) -> Result<Self> {
        Self::new_with_highlights(lang, text, theme, None)
    }

    pub fn new_with_highlights(
        lang: &str,
        text: &str,
        theme: Vec<(&str, &str)>,
        custom_highlights: Option<HashMap<String, String>>,
    ) -> Result<Self> {
        let code = Code::new(text, lang, custom_highlights.clone())
            .or_else(|_| Code::new(text, "text", custom_highlights))?;

        let theme = Self::build_theme(&theme);
        let highlights_cache = RefCell::new(HashMap::new());

        Ok(Self {
            code,
            cursor: 0,
            offset_y: 0,
            offset_x: 0,
            theme,
            selection: None,
            clicks: ClickTracker::new(Duration::from_millis(700)),
            selection_snap: SelectionSnap::None,
            clipboard: None,
            marks: None,
            highlights_cache,
        })
    }

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
    
    pub fn focus(&mut self, area: &Rect) {
        let width = area.width as usize;
        let height = area.height as usize;
        let total_lines = self.code.len_lines();
        let max_line_number = total_lines.max(1);
        let line_number_digits = max_line_number.to_string().len().max(5);
        let line_number_width = (line_number_digits + 2) as usize;

        let line = self.code.char_to_line(self.cursor);
        let col = self.cursor - self.code.line_to_char(line);
    
        let visible_width = width.saturating_sub(line_number_width);
        let visible_height = height;
    
        let step_size = 10;
        if col < self.offset_x {
            self.offset_x = col.saturating_sub(step_size);
        } else if col >= self.offset_x + visible_width {
            self.offset_x = col.saturating_sub(visible_width - step_size);
        }
    
        if line < self.offset_y {
            self.offset_y = line;
        } else if line >= self.offset_y + visible_height {
            self.offset_y = line.saturating_sub(visible_height - 1);
        }
    }

    pub fn mouse(
        &mut self, mouse: MouseEvent, area: &Rect,
    ) -> Result<()> {

        match mouse.kind {
            MouseEventKind::ScrollUp => self.scroll_up(),
            MouseEventKind::ScrollDown => self.scroll_down(area.height as usize),
            MouseEventKind::Down(MouseButton::Left) => {
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

    fn handle_mouse_down(&mut self, cursor: usize) {
        let kind = self.clicks.register(cursor);
        let (start, end, snap) = match kind {
            ClickKind::Triple => {
                let (line_start, line_end) = self.code.line_boundaries(cursor);
                (line_start, line_end, SelectionSnap::Line { anchor: cursor })
            }
            ClickKind::Double => {
                let (word_start, word_end) = self.code.word_boundaries(cursor);
                (word_start, word_end, SelectionSnap::Word { anchor: cursor })
            }
            ClickKind::Single => (cursor, cursor, SelectionSnap::None),
        };

        self.selection = Some(Selection::from_anchor_and_cursor(start, end));
        self.cursor = end;
        self.selection_snap = snap;
    }

    fn handle_mouse_drag(&mut self, cursor: usize) {
        match self.selection_snap {
            SelectionSnap::Line { anchor } => {
                let (anchor_start, anchor_end) = self.code.line_boundaries(anchor);
                let (cur_start, cur_end) = self.code.line_boundaries(cursor);
        
                let (sel_start, sel_end, new_cursor) = match cursor.cmp(&anchor) {
                    Ordering::Greater => (anchor_start, cur_end, cur_end),   // forward
                    Ordering::Less => (cur_start, anchor_end, cur_start),    // backward
                    Ordering::Equal => (anchor_start, anchor_end, anchor_end), 
                };
        
                self.selection = Some(Selection::from_anchor_and_cursor(sel_start, sel_end));
                self.cursor = new_cursor;
            }
            SelectionSnap::Word { anchor } => {
                let (anchor_start, anchor_end) = self.code.word_boundaries(anchor);
                let (cur_start, cur_end) = self.code.word_boundaries(cursor);
        
                let (sel_start, sel_end, new_cursor) = match cursor.cmp(&anchor) {
                    Ordering::Greater => (anchor_start, cur_end, cur_end),   // forward
                    Ordering::Less => (cur_start, anchor_end, cur_start),    // backward
                    Ordering::Equal => (anchor_start, anchor_end, anchor_end),
                };
        
                self.selection = Some(Selection::from_anchor_and_cursor(sel_start, sel_end));
                self.cursor = new_cursor;
            }
            SelectionSnap::None => {
                let anchor = self.selection_anchor();
                self.selection = Some(Selection::from_anchor_and_cursor(anchor, cursor));
                self.cursor = cursor;
            }
        }
    }

    fn cursor_from_mouse(
        &self, mouse_x: u16, mouse_y: u16, area: &Rect
    ) -> Option<usize> {
        let total_lines = self.code.len_lines();
        let max_line_number = total_lines.max(1);
        let line_number_digits = max_line_number.to_string().len().max(5);
        let line_number_width = (line_number_digits + 2) as u16;
    
        if mouse_y < area.top()
            || mouse_y >= area.bottom()
            || mouse_x < area.left() + line_number_width
        {
            return None;
        }
    
        let clicked_row = (mouse_y - area.top()) as usize + self.offset_y;
        if clicked_row >= self.code.len_lines() {
            return None;
        }
    
        let clicked_col = (mouse_x - area.left() - line_number_width) as usize;
    
        let line_start_char = self.code.line_to_char(clicked_row);
        let line_len = self.code.line_len(clicked_row);
    
        let start_col = self.offset_x.min(line_len);
        let end_col = line_len;
    
        let char_start = line_start_char + start_col;
        let char_end = line_start_char + end_col;
    
        let mut current_col = 0;
        let mut char_idx = start_col;        
        let visible_chars = self.code.char_slice(char_start, char_end);
        for g in RopeGraphemes::new(&visible_chars) {
            let (g_width, g_chars) = grapheme_width_and_chars_len(g);        
            if current_col + g_width > clicked_col { break; }
            current_col += g_width;
            char_idx += g_chars;
        }
    
        let line = self.code.char_slice(line_start_char, line_start_char + line_len);
        let visual_width: usize = RopeGraphemes::new(&line).map(grapheme_width).sum();
    
        if clicked_col + self.offset_x >= visual_width {
            let mut end_idx = line.len_chars();
            if end_idx > 0 && line.char(end_idx - 1) == '\n' {
                end_idx -= 1;
            }
            char_idx = end_idx;
        }
    
        Some(line_start_char + char_idx)
    }

    /// Clears any active selection.
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Extends or starts a selection from the current cursor to `new_cursor`.
    pub fn extend_selection(&mut self, new_cursor: usize) {
        // If there was already a selection, preserve the anchor (start point)
        // otherwise, use the current cursor as the anchor.
        let anchor = self.selection_anchor();
        self.selection = Some(Selection::from_anchor_and_cursor(anchor, new_cursor));
    }
    
    /// Returns the selection anchor position, or the cursor if no selection exists.
    pub fn selection_anchor(&self) -> usize {
        self.selection
            .as_ref()
            .map(|s| if self.cursor == s.start { s.end } else { s.start })
            .unwrap_or(self.cursor)
    }

    pub fn apply<A: Action>(&mut self, mut action: A) {
        action.apply(self);
    }

    pub fn set_content(&mut self, content: &str) {
        self.code.tx();
        self.code.set_state_before(self.cursor, self.selection);
        self.code.remove(0, self.code.len());
        self.code.insert(0, content);
        self.code.set_state_after(self.cursor, self.selection);
        self.code.commit();
        self.reset_highlight_cache();
    }

    pub fn apply_batch(&mut self, batch: &EditBatch) {
        self.code.tx();

        if let Some(state) = &batch.state_before {
            self.code.set_state_before(state.offset, state.selection);
        }
        if let Some(state) = &batch.state_after {
            self.code.set_state_after(state.offset, state.selection);
        }
        
        for edit in &batch.edits {
            match &edit.kind {
                EditKind::Insert { offset, text } => {
                    self.code.insert(*offset, text);
                }
                EditKind::Remove { offset, text } => {
                    self.code.remove(*offset, *offset + text.chars().count());
                }
            }
        }
        self.code.commit();
        self.reset_highlight_cache();
    }

    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor;
        self.fit_cursor();
    }

    pub fn fit_cursor(&mut self) {
        // make sure cursor is not out of bounds 
        let len = self.code.len_chars();
        self.cursor = self.cursor.min(len);

        // make sure cursor is not out of bounds on the line
        let (row, col) = self.code.point(self.cursor);
        if col > self.code.line_len(row) {
            self.cursor = self.code.line_to_char(row) + self.code.line_len(row);
        }
    }

    pub fn scroll_up(&mut self) {
        if self.offset_y > 0 {
            self.offset_y -= 1;
        }
    }

    pub fn scroll_down(&mut self, area_height: usize) {
        let len_lines = self.code.len_lines();
        if self.offset_y < len_lines.saturating_sub(area_height) {
            self.offset_y += 1;
        }
    }

    fn build_theme(theme: &Vec<(&str, &str)>) -> Theme {
        theme.into_iter()
            .map(|(name, hex)| {
                let (r, g, b) = utils::rgb(hex);
                (name.to_string(), Style::default().fg(Color::Rgb(r, g, b)))
            })
            .collect()
    }

    pub fn get_content(&self) -> String {
        self.code.get_content()
    }

    pub fn get_content_slice(&self, start: usize, end: usize) -> String {
        self.code.slice(start, end)
    }

    pub fn get_cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_clipboard(&mut self, text: &str) -> Result<()> {
        arboard::Clipboard::new()
            .and_then(|mut c| c.set_text(text.to_string()))
            .unwrap_or_else(|_| self.clipboard = Some(text.to_string()));
        Ok(())
    }

    pub fn get_clipboard(&self) -> Result<String> {
        arboard::Clipboard::new()
            .and_then(|mut c| c.get_text())
            .ok()
            .or_else(|| self.clipboard.clone())
            .ok_or_else(|| anyhow!("cant get clipboard"))
    }

    pub fn set_marks(&mut self, marks: Vec<(usize, usize, &str)>) {
        self.marks = Some(
            marks.into_iter()
                .map(|(start, end, color)| {
                    let (r, g, b) = utils::rgb(color);
                    (start, end, Color::Rgb(r, g, b))
                })
                .collect()
        );
    }

    pub fn remove_marks(&mut self) {
        self.marks = None;
    }

    pub fn has_marks(&self) -> bool {
        self.marks.is_some()
    }

    pub fn get_marks(&self) -> Option<&Vec<(usize, usize, Color)>> {
        self.marks.as_ref()
    }

    pub fn get_selection_text(&mut self) -> Option<String> {
        if let Some(selection) = &self.selection && !selection.is_empty() {
            let text = self.code.slice(selection.start, selection.end);
            return Some(text);
        }
        None
    }

    pub fn get_selection(&mut self) -> Option<Selection> {
       return self.selection;
    }

    pub fn set_selection(&mut self, selection: Option<Selection>) {
        self.selection = selection;
    }

    pub fn set_offset_y(&mut self, offset_y: usize) {
        self.offset_y = offset_y;
    }

    pub fn set_offset_x(&mut self, offset_x: usize) {
        self.offset_x = offset_x;
    }
    
    pub fn get_offset_y(&self) -> usize {
        self.offset_y
    }

    pub fn get_offset_x(&self) -> usize {
        self.offset_x
    }

    pub fn code_mut(&mut self) -> &mut Code {
        &mut self.code
    }

    pub fn code_ref(&self) -> &Code {
        &self.code
    }

    /// Set the change callback function for handling document changes
    pub fn set_change_callback(
        &mut self, callback: Box<dyn Fn(Vec<(usize, usize, usize, usize, String)>)>
    ) {
        self.code.set_change_callback(callback);
    }

    pub fn highlight_interval(
        &self, start: usize, end: usize, theme: &Theme
    ) -> Vec<(usize, usize, Style)> {
        let mut cache = self.highlights_cache.borrow_mut();
        let key = (start, end);
        if let Some(v) = cache.get(&key) {
            return v.clone();
        }

        let highlights = self.code.highlight_interval(start, end, theme);
        cache.insert(key, highlights.clone());
        highlights
    }

    pub fn reset_highlight_cache(&self) {
        self.highlights_cache.borrow_mut().clear();
    }
    
    /// calculates visible cursor position 
    pub fn get_visible_cursor(
        &self, area: &Rect
    ) -> Option<(u16, u16)> {
        let total_lines = self.code.len_lines();
        let max_line_number = total_lines.max(1);
        let line_number_digits = max_line_number.to_string().len().max(5);
        let line_number_width = line_number_digits + 2;

        let (cursor_line, cursor_char_col) = self.code.point(self.cursor);
        
        if cursor_line >= self.offset_y && cursor_line < self.offset_y + area.height as usize {
            let line_start_char = self.code.line_to_char(cursor_line);
            let line_len = self.code.line_len(cursor_line);
        
            let max_x = (area.width as usize).saturating_sub(line_number_width);
            let start_col = self.offset_x;
                
            let cursor_visual_col: usize = {
                let slice = self.code.char_slice(line_start_char, line_start_char + cursor_char_col.min(line_len));
                RopeGraphemes::new(&slice).map(grapheme_width).sum()
            };
            
            let offset_visual_col: usize = {
                let slice = self.code.char_slice(line_start_char, line_start_char + start_col.min(line_len));
                RopeGraphemes::new(&slice).map(grapheme_width).sum()
            };
        
            let relative_visual_col = cursor_visual_col.saturating_sub(offset_visual_col);
            let visible_x = relative_visual_col.min(max_x);
        
            let cursor_x = area.left() + (line_number_width + visible_x) as u16;
            let cursor_y = area.top() + (cursor_line - self.offset_y) as u16;
        
            if cursor_x < area.right() && cursor_y < area.bottom() {
                return Some((cursor_x, cursor_y));
            }
        }
        
        return None;
    }
}
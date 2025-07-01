use crossterm::event::{
    KeyEvent, KeyModifiers,
    MouseEvent, MouseEventKind, MouseButton,
};
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::{prelude::*, widgets::Widget};
use std::collections::HashMap;
use unicode_width::UnicodeWidthChar;
use std::time::{Instant, Duration};
use crate::code::Code;
use crate::history::{EditBatch, Edit, EditKind};
use crate::selection::Selection;
use crate::utils;

pub struct Editor {
    code: Code,
    cursor: usize,
    offset_y: usize,
    offset_x: usize,
    theme: HashMap<String, Style>,
    selection: Option<Selection>,
    last_click: Option<(Instant, usize)>,
    last_last_click: Option<(Instant, usize)>,
    marks: Option<Vec<(usize, usize, Color)>>
}

impl Editor {
    /// Create a new editor instance with language, text, and theme
    pub fn new(lang: &str, text: &str, theme: Vec<(&str, &str)>) -> Self {
        let code = Code::new(text, lang)
            .or_else(|_| Code::new(text, "text"))
            .unwrap();

        let theme = Self::build_theme(&theme);

        Self {
            code,
            cursor: 0,
            offset_y: 0,
            offset_x: 0,
            theme,
            selection: None,
            last_click: None,
            last_last_click: None,
            marks: None,
        }
    }

    pub fn input(
        &mut self,
        key: KeyEvent,
        area: &Rect,
    ) -> anyhow::Result<()> {
        use crossterm::event::KeyCode;

        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        // let alt = key.modifiers.contains(KeyModifiers::ALT);

        match key.code {
            KeyCode::Char('z') if ctrl => self.handle_undo(),
            KeyCode::Char('y') if ctrl => self.handle_redo(),
            KeyCode::Char('c') if ctrl => self.handle_copy()?,
            KeyCode::Char('v') if ctrl => self.handle_paste()?,
            KeyCode::Char('x') if ctrl => self.handle_cut()?,
            KeyCode::Char('k') if ctrl => self.handle_delete_line(),
            KeyCode::Char('d') if ctrl => self.handle_duplicate()?,
            KeyCode::Char('a') if ctrl => self.handle_select_all(),

            KeyCode::Char('w') if ctrl => self.offset_x += 1,
            KeyCode::Char('q') if ctrl => self.offset_x = self.offset_x.saturating_sub(1),
            KeyCode::Left      => self.handle_left(shift),
            KeyCode::Right     => self.handle_right(shift),
            KeyCode::Up        => self.handle_up(shift),
            KeyCode::Down      => self.handle_down(shift),
            KeyCode::Backspace => self.handle_delete(),
            KeyCode::Enter     => self.handle_enter(),
            KeyCode::Char(c)   => self.handle_char(c),
            KeyCode::Tab       => self.handle_tab(),
            _ => {}
        }

        self.focus(&area);

        Ok(())
    }
    
    fn focus(&mut self, area: &Rect) {
        let width = area.width as usize;
        let height = area.height as usize;
        let total_lines = self.code.len_lines();
        let max_line_number = total_lines.max(1);
        let line_number_digits = max_line_number.to_string().len();
        let line_number_width = (line_number_digits + 2) as usize;

        let line = self.code.char_to_line(self.cursor);
        let col = self.cursor - self.code.line_to_char(line);
    
        let visible_width = width.saturating_sub(line_number_width);
        let visible_height = height;
    
        if col < self.offset_x {
            self.offset_x = col;
        } else if col >= self.offset_x + visible_width {
            self.offset_x = col.saturating_sub(visible_width - 1);
        }
    
        if line < self.offset_y {
            self.offset_y = line;
        } else if line >= self.offset_y + visible_height {
            self.offset_y = line.saturating_sub(visible_height - 1);
        }
    }

    pub fn mouse(
        &mut self,
        mouse: MouseEvent,
        area: &Rect,
    ) -> anyhow::Result<()> {

        match mouse.kind {
            MouseEventKind::ScrollUp => self.scroll_up(),
            MouseEventKind::ScrollDown => self.scroll_down(area.height as usize),

            MouseEventKind::Down(MouseButton::Left) => {
                let pos = self.cursor_from_mouse(mouse.column, mouse.row, area);

                if let Some(cursor) = pos {
                    let now = Instant::now();
                    let max_dt = Duration::from_millis(700);

                    let click = (now, cursor);
                    let (dbl, tpl) = (
                        self.last_click
                            .map(|(t, p)|{
                                p == cursor && now.duration_since(t) < max_dt
                            }).unwrap_or(false),
                        self.last_click.zip(self.last_last_click)
                            .map(|((t1, p1), (t0, p0))| {
                                p0 == cursor && p1 == cursor &&
                                now.duration_since(t0) < max_dt &&
                                t1.duration_since(t0) < max_dt
                            })
                            .unwrap_or(false),
                    );

                    let (start, end) = if tpl {
                        self.code.line_boundaries(cursor)
                    } else if dbl {
                        self.code.word_boundaries(cursor)
                    } else {
                        (cursor, cursor)
                    };

                    self.selection = Some(Selection::from_anchor_and_cursor(start, end));
                    self.cursor = end;

                    self.last_last_click = self.last_click;
                    self.last_click = Some(click);
                }
            }

            MouseEventKind::Drag(MouseButton::Left) => {
                let pos = self.cursor_from_mouse(mouse.column, mouse.row, area);

                if let Some(cursor) = pos {
                    let anchor = self.selection_anchor();
                    self.selection = Some(Selection::from_anchor_and_cursor(anchor, cursor));
                    self.cursor = cursor;
                }
            }

            _ => {}
        }

        Ok(())
    }

    fn cursor_from_mouse(
        &self, mouse_x: u16, mouse_y: u16, area: &Rect
    ) -> Option<usize> {
        let total_lines = self.code.len_lines();
        let max_line_number = total_lines.max(1);
        let line_number_digits = max_line_number.to_string().len();
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
    
        let visible_chars = self.code.char_slice(char_start, char_end);
    
        let mut current_col = 0;
        let mut char_idx = start_col;
    
        for ch in visible_chars.chars() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1);
            if current_col + ch_width > clicked_col {
                break;
            }
            current_col += ch_width;
            char_idx += 1;
        }
    
        let line = self.code.char_slice(line_start_char, line_start_char + line_len);

        let visual_width: usize = line.chars()
            .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(1))
            .sum();
    
        if clicked_col + self.offset_x >= visual_width {
            let mut end_idx = line.len_chars();
            if end_idx > 0 && line.char(end_idx - 1) == '\n' {
                end_idx -= 1;
            }
            char_idx = end_idx;
        }
    
        Some(line_start_char + char_idx)
    }

    fn handle_left(&mut self, shift: bool) {
        if self.cursor > 0 {
            let new_cursor = self.cursor - 1;
            self.update_selection(new_cursor, shift);
            self.cursor = new_cursor;
        }
    }

    fn handle_right(&mut self, shift: bool) {
        if self.cursor < self.code.len() {
            let new_cursor = self.cursor + 1;
            self.update_selection(new_cursor, shift);
            self.cursor = new_cursor;
        }
    }

    fn handle_up(&mut self, shift: bool) {
        let (row, col) = self.code.point(self.cursor);
        if row > 0 {
            let prev_line_start = self.code.line_to_char(row - 1);
            let prev_line_len = self.code.line_len(row - 1);
            let new_col = if prev_line_len < col {
                prev_line_len.saturating_sub(1)
            } else {
                col
            };
            let new_cursor = prev_line_start + new_col;
            self.update_selection(new_cursor, shift);
            self.cursor = new_cursor;
        }
    }

    fn handle_down(&mut self, shift: bool) {
        let (row, col) = self.code.point(self.cursor);
        if row + 1 < self.code.len_lines() {
            let next_line_start = self.code.line_to_char(row + 1);
            let next_line_len = self.code.line_len(row + 1);
            let new_col = if next_line_len < col {
                next_line_len.saturating_sub(1)
            } else {
                col
            };
            let new_cursor = next_line_start + new_col;
            self.update_selection(new_cursor, shift);
            self.cursor = new_cursor;
        }
    }

    fn update_selection(&mut self, new_cursor: usize, shift: bool) {
        if shift {
            let anchor = self.selection_anchor();
            self.selection = Some(Selection::from_anchor_and_cursor(anchor, new_cursor));
        } else {
            self.selection = None;
        }
    }

    fn selection_anchor(&self) -> usize {
        self.selection
            .as_ref()
            .map(|s| if self.cursor == s.start { s.end } else { s.start })
            .unwrap_or(self.cursor)
    }

    pub fn handle_char(&mut self, text: char) {
        let text = text.to_string();
        self.code.begin_batch();
        self.remove_selection();
        self.code.insert(self.cursor, &text);
        self.code.commit_batch();
        self.cursor += text.chars().count();
    }


    pub fn handle_enter(&mut self) {
        self.code.begin_batch();
        self.remove_selection();

        let (row, _) = self.code.point(self.cursor);
        let indent_level = self.code.indentation_level(row);
        let indent_text = self.code.indent().repeat(indent_level);
        let text = format!("\n{}", indent_text);
        self.code.insert(self.cursor, &text);
        self.code.commit_batch();
        self.cursor += text.chars().count();
    }

    pub fn insert_text(&mut self, pos: usize, text: &str) {
        self.code.begin_batch();
        self.code.insert(pos, text);
        self.code.commit_batch();
    }

    pub fn delete_text(&mut self, from: usize, to: usize) {
        self.code.begin_batch();
        self.code.remove(from, to);
        self.code.commit_batch();
    }

    pub fn remove_selection(&mut self) {
        if let Some(selection) = &self.selection {
            if selection.is_empty() {
                self.selection = None;
                return;
            }
            let (start, end) = selection.sorted();
            self.code.remove(start, end);
            self.cursor = start;
            self.selection = None;
        }
    }

    fn handle_delete(&mut self) {
        if let Some(selection) = &self.selection {
            let (start, end) = selection.sorted();
            self.delete_text(start, end);
            self.cursor = start;
            self.selection = None;
        } else if self.cursor > 0 {
            let (row, col) = self.code.point(self.cursor);
            if self.code.is_only_indentation_before(row, col) {
                let from = self.cursor - col;
                self.delete_text(from, self.cursor);
                self.cursor = from;
            } else {
                self.delete_text(self.cursor - 1, self.cursor);
                self.cursor -= 1;
            }
        }
    }

    pub fn handle_tab(&mut self, ) {
        let text = self.code.indent();
        self.code.begin_batch();
        self.code.insert(self.cursor, &text);
        self.code.commit_batch();
        self.cursor += text.chars().count();
    }

    fn handle_undo(&mut self) {
        let edits = self.code.undo();
        if let Some(edits) = edits {
            for edit in edits.iter().rev()  {
                match &edit.kind {
                    EditKind::Insert { offset, text: _ } => {
                        self.cursor = *offset;
                    }
                    EditKind::Remove { offset, text } => {
                        self.cursor = *offset + text.chars().count();
                    }
                }
            }
        }
    }

    fn handle_redo(&mut self) {
        let edits = self.code.redo();
        if let Some(edits) = edits {
            for edit in edits {
                match &edit.kind {
                    EditKind::Insert { offset, text } => {
                        self.cursor = *offset + text.chars().count();
                    }
                    EditKind::Remove { offset, text: _ } => {
                        self.cursor = *offset;
                    }
                }
            }
        }
    }

    pub fn set_content(&mut self, content: &str) {
        self.code.begin_batch();
        self.code.remove(0, self.code.len());
        self.code.insert(0, content);
        self.code.commit_batch();
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

    pub fn apply_edits(&mut self, edits: &EditBatch) {
        self.code.begin_batch();
        for edit in edits {
            match &edit.kind {
                EditKind::Insert { offset, text } => {
                    self.code.insert(*offset, text);
                }
                EditKind::Remove { offset, text } => {
                    self.code.remove(*offset, *offset + text.chars().count());
                }
            }
        }
        self.code.commit_batch();
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

    fn build_theme(theme: &Vec<(&str, &str)>) -> HashMap<String, Style> {
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

    pub fn get_cursor(&self) -> usize {
        self.cursor
    }

    pub fn handle_copy(&mut self) -> anyhow::Result<()> {
        if let Some(selection) = &self.selection {
            let text = self.code.slice(selection.start, selection.end);
            let mut clipboard = arboard::Clipboard::new()?;
            clipboard.set_text(text)?;
        }
        Ok(())
    }

    pub fn handle_cut(&mut self) -> anyhow::Result<()> {
        if let Some(selection) = self.selection.take() {
            let text = self.code.slice(selection.start, selection.end);
            let mut clipboard = arboard::Clipboard::new()?;
            clipboard.set_text(text)?;

            self.delete_text(selection.start, selection.end);
            self.cursor = selection.start;
            self.selection = None;
        }
        Ok(())
    }

    pub fn handle_paste(&mut self) -> anyhow::Result<()> {
        let mut clipboard = arboard::Clipboard::new()?;
        let text = clipboard.get_text()?;

        self.code.begin_batch();
        self.remove_selection();
        self.code.insert(self.cursor, &text);
        self.code.commit_batch();
        self.cursor += text.chars().count();

        Ok(())
    }

    pub fn handle_delete_line(&mut self) {
        let (start, end) = self.code.line_boundaries(self.cursor);

        if start == end && start == self.code.len() {
            return;
        }

        self.delete_text(start, end);
        self.cursor = start;
        self.selection = None;
    }

    pub fn handle_duplicate(&mut self) -> anyhow::Result<()> {
        if let Some(selection) = &self.selection {
            let text = self.code.slice(selection.start, selection.end);
            let insert_pos = selection.end;
            self.insert_text(insert_pos, &text);
            self.cursor = insert_pos + text.chars().count();
            self.selection = None;
        } else {
            let (line_start, line_end) = self.code.line_boundaries(self.cursor);
            let line_text = self.code.slice(line_start, line_end);
            let column = self.cursor - line_start;

            let insert_pos = line_end;
            let to_insert = if line_text.ends_with('\n') {
                line_text.clone()
            } else {
                format!("{}\n", line_text)
            };
            self.insert_text(insert_pos, &to_insert);

            let new_line_len = to_insert.trim_end_matches('\n').chars().count();
            let new_column = column.min(new_line_len);

            self.cursor = insert_pos + new_column;
        }
        Ok(())
    }

    pub fn handle_select_all(&mut self) {
        let from = 0;
        let to = self.code.len_chars();
        self.selection = Some(Selection::new(from, to));
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
}

impl Widget for &Editor {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let total_lines = self.code.len_lines();
        let total_chars = self.code.len_chars();
        let max_line_number = total_lines.max(1);
        let line_number_digits = max_line_number.to_string().len();
        let line_number_width = line_number_digits + 2;

        let (cursor_line, cursor_char_col) = self.code.point(self.cursor);
        let mut draw_y = area.top();
        
        let line_number_style = Style::default().fg(Color::DarkGray);
        let default_text_style = Style::default().fg(Color::White);

        // draw line numbers and text
        for line_idx in self.offset_y..total_lines {
            if draw_y >= area.bottom() { break }
        
            let line_number = format!("{:>width$}  ", line_idx + 1, width = line_number_digits);
            buf.set_string(area.left(), draw_y, &line_number, line_number_style);
        
            let line_len = self.code.line_len(line_idx);
            let max_x = (area.width as usize).saturating_sub(line_number_width);
        
            let start_col = self.offset_x.min(line_len);
            let end_col = (start_col + max_x).min(line_len);
        
            let line_start_char = self.code.line_to_char(line_idx);
            let char_start = line_start_char + start_col;
            let char_end = line_start_char + end_col;
        
            let visible_chars = self.code.char_slice(char_start, char_end);

            let displayed_line = visible_chars.to_string().replace("\t", &" ");
        
            let text_x = area.left() + line_number_width as u16;
            if text_x < area.left() + area.width && draw_y < area.top() + area.height {
                buf.set_string(text_x, draw_y, &displayed_line, default_text_style);
            }
        
            draw_y += 1;
        }

        // draw syntax highlighting
        if self.code.is_highlight() {
        
            // Render syntax highlighting for the visible portion of the text buffer.
            // For each visible line within the viewport, limit the highlighting to the
            // visible columns to avoid expensive processing of long lines outside the view.
            // This improves performance by only querying Tree-sitter for the visible slice,
            // then applying styles per character based on byte ranges returned by the syntax query.
            
            for screen_y in 0..(area.height as usize) {
                let line_idx = self.offset_y + screen_y;
                if line_idx >= total_lines { break }
            
                let line_len = self.code.line_len(line_idx);
                let max_x = (area.width as usize).saturating_sub(line_number_width);
            
                let line_start_char = self.code.line_to_char(line_idx);
                let start_char = line_start_char + self.offset_x;
                let visible_len = line_len.saturating_sub(self.offset_x);
                let end = max_x.min(visible_len);
                let end_char = start_char + end;

                if start_char > total_chars || end_char > total_chars {
                    continue; // last line offset case 
                }

                let chars = self.code.char_slice(start_char, end_char);

                let start_byte = self.code.char_to_byte(start_char);
            
                let highlights = self.code.highlight_interval(
                    start_char, end_char, &self.theme
                );
            
                let mut x = 0;
                let mut byte_idx_in_rope = start_byte;
            
                for ch in chars.chars().take(max_x) {
                    if x >= max_x { break }
            
                    let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1);
                    let ch_len = ch.len_utf8();
            
                    let draw_x = area.left() + line_number_width as u16 + x as u16;
                    let draw_y = area.top() + screen_y as u16;
            
                    let mut style = Style::default();
                    for &(start, end, s) in &highlights {
                        if start <= byte_idx_in_rope && byte_idx_in_rope < end {
                            style = s;
                            break;
                        }
                    }
            
                    buf[(draw_x, draw_y)].set_style(style);
            
                    x += ch_width;
                    byte_idx_in_rope += ch_len;
                }
            }
        }

        // draw selection
        if let Some(selection) = self.selection {
            let start = selection.start.min(selection.end);
            let end = selection.start.max(selection.end);
        
            let start_line = self.code.char_to_line(start);
            let end_line = self.code.char_to_line(end);
        
            for line_idx in start_line..=end_line {
                if line_idx < self.offset_y { continue }
                if line_idx >= self.offset_y + area.height as usize { break }
        
                let line_start_char = self.code.line_to_char(line_idx);
                let line_len = self.code.line_len(line_idx);
                let line_end_char = line_start_char + line_len;
        
                let sel_start = start.max(line_start_char);
                let sel_end = end.min(line_end_char);
        
                let rel_start = sel_start - line_start_char;
                let rel_end = sel_end - line_start_char;
        
                let start_col = self.offset_x.min(line_len);
                let max_text_width = (area.width as usize).saturating_sub(line_number_width);
                let end_col = (start_col + max_text_width).min(line_len);
        
                let char_slice_start = line_start_char + start_col;
                let char_slice_end = line_start_char + end_col;
        
                let visible_chars = self.code.char_slice(char_slice_start, char_slice_end);

                let draw_y = area.top() + (line_idx - self.offset_y) as u16;
                let mut visual_x = 0;
                let mut char_col = start_col;
        
                for ch in visible_chars.chars() {
                    if char_col >= rel_start && char_col < rel_end {
                        let draw_x = area.left() + line_number_width as u16 + visual_x;
                        if draw_x < area.right() && draw_y < area.bottom() {
                            buf[(draw_x, draw_y)].set_style(Style::default().bg(Color::DarkGray));
                        }
                    }
        
                    visual_x += UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
                    char_col += 1;
                }
            }
        }

        // draw highlights
        if let Some(ref marks) = self.marks {
            for &(start, end, color) in marks {
                if start >= end || end > total_chars { continue }

                let start_line = self.code.char_to_line(start);
                let end_line = self.code.char_to_line(end);

                for line_idx in start_line..=end_line {
                    if line_idx < self.offset_y || line_idx >= self.offset_y + area.height as usize {
                        continue;
                    }

                    let line_start_char = self.code.line_to_char(line_idx);
                    let line_len = self.code.line_len(line_idx);
                    let line_end_char = line_start_char + line_len;

                    let highlight_start = start.max(line_start_char);
                    let highlight_end = end.min(line_end_char);

                    let rel_start = highlight_start - line_start_char;
                    let rel_end = highlight_end - line_start_char;

                    let start_col = self.offset_x.min(line_len);
                    let max_text_width = (area.width as usize).saturating_sub(line_number_width);
                    let end_col = (start_col + max_text_width).min(line_len);

                    let char_slice_start = line_start_char + start_col;
                    let char_slice_end = line_start_char + end_col;

                    let visible_chars = self.code.char_slice(char_slice_start, char_slice_end);

                    let draw_y = area.top() + (line_idx - self.offset_y) as u16;
                    let mut visual_x = 0;
                    let mut char_col = start_col;

                    for ch in visible_chars.chars() {
                        if char_col >= rel_start && char_col < rel_end {
                            let draw_x = area.left() + line_number_width as u16 + visual_x;
                            if draw_x < area.right() && draw_y < area.bottom() {
                                buf[(draw_x, draw_y)].set_bg(color);
                            }
                        }

                        visual_x += UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
                        char_col += 1;
                    }
                }
            }
        }


        // draw cursor
        if cursor_line >= self.offset_y && cursor_line < self.offset_y + area.height as usize {
            let line_start_char = self.code.line_to_char(cursor_line);
            let line_len = self.code.line_len(cursor_line);
        
            let max_x = (area.width as usize).saturating_sub(line_number_width);
            let start_col = self.offset_x;
                
            let cursor_visual_col: usize = self.code
                .char_slice(line_start_char, line_start_char + cursor_char_col.min(line_len))
                .chars().map(|ch| UnicodeWidthChar::width(ch).unwrap_or(1)).sum();
        
            let offset_visual_col: usize = self.code
                .char_slice(line_start_char, line_start_char + start_col.min(line_len))
                .chars().map(|ch| UnicodeWidthChar::width(ch).unwrap_or(1)).sum();
        
            let relative_visual_col = cursor_visual_col.saturating_sub(offset_visual_col);
            let visible_x = relative_visual_col.min(max_x);
        
            let cursor_x = area.left() + line_number_width as u16 + visible_x as u16;
            let cursor_y = area.top() + (cursor_line - self.offset_y) as u16;
        
            if cursor_x < area.right() && cursor_y < area.bottom() {
                buf[(cursor_x, cursor_y)].set_style(
                    Style::default().bg(Color::White).fg(Color::Black)
                );
            }
        }

    }
}
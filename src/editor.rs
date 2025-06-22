use crossterm::event::{
    KeyEvent, MouseEvent, MouseEventKind, MouseButton, KeyModifiers
};
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::{prelude::*, widgets::Widget};
use std::collections::HashMap;
use unicode_width::UnicodeWidthChar;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::Stdout;
use std::time::{Instant, Duration};

use crate::code::Code;
use crate::history::{Edit, EditKind};
use crate::selection::Selection;

type Theme = HashMap<String, Style>;

pub struct Editor {
    name: String,
    code: Code,
    cursor: usize,
    offset_y: usize,
    width: usize,
    height: usize,
    theme: Theme,
    selection: Option<Selection>,
    last_click: Option<(Instant, usize)>,
    last_last_click: Option<(Instant, usize)>,
}

impl Editor {
    /// Create a new editor instance, 
    /// with the given name, language, text, width, and height.
    pub fn new(
        name: &str, lang: &str, text: &str, w: usize, h: usize,
        theme: Vec<(&str, &str)>,
    ) -> Self {
        let code = Code::new(text, lang)
            .or_else(|_| Code::new(text, "text"))
            .unwrap();
        
        let theme = Self::build_theme(&theme);

        Self {
            name: name.to_string(),
            code,
            cursor: 0,
            offset_y: 0,
            width: w,
            height: h,
            theme,
            selection: None,
            last_click: None,
            last_last_click: None,
        }
    }
    
    pub fn input(
        &mut self, key: KeyEvent
    ) -> anyhow::Result<()> {
        use crossterm::event::KeyCode::*;

        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        if ctrl {
            match key.code {
                Char('z') => self.undo(),
                Char('y') => self.redo(),
                _ => {}
            }
        } else {
            match key.code {
                Left => self.move_left(shift),
                Right => self.move_right(shift),
                Up => self.move_up(shift),
                Down => self.move_down(shift),
                Backspace => self.delete_char(),
                Enter => self.insert_text("\n"),
                Char(c) => self.insert_text(&c.to_string()),
                _ => {}
            }
        }
        self.scroll_to_cursor();
        
        
        Ok(())
    }

    pub fn mouse(
        &mut self,
        mouse: MouseEvent,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> anyhow::Result<()> {
        let area = terminal.get_frame().area();
        let pos = self.cursor_from_mouse(mouse.column, mouse.row, area);

        match mouse.kind {
            MouseEventKind::ScrollUp => self.scroll_up(),
            MouseEventKind::ScrollDown => self.scroll_down(area.height as usize),

            MouseEventKind::Down(MouseButton::Left) => {
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
    
    fn cursor_from_mouse(&self, mouse_x: u16, mouse_y: u16, area: Rect) -> Option<usize> {
        let total_lines = self.code.content.len_lines();
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
        if clicked_row >= self.code.content.len_lines() {
            return None;
        }

        let clicked_col = (mouse_x - area.left() - line_number_width) as usize;
        let line = self.code.content.line(clicked_row);

        let mut current_col = 0;
        let mut char_idx = 0;
        for ch in line.chars() {
            let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if current_col + ch_width > clicked_col {
                break;
            }
            current_col += ch_width;
            char_idx += 1;
        }

        let line_visual_width: usize = line
            .chars()
            .map(|ch| unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0))
            .sum();

        if clicked_col >= line_visual_width {
            let mut end_idx = line.len_chars();
            if end_idx > 0 && line.char(end_idx - 1) == '\n' {
                end_idx -= 1;
            }
            char_idx = end_idx;
        }

        let line_start = self.code.content.line_to_char(clicked_row);
        Some(line_start + char_idx)
    }


    pub(crate) fn resize(&mut self, w: u16, h: u16) {
        self.width = w as usize;
        self.height = h as usize;
    }
    
    fn position(&self) -> (usize, usize) {
        let row = self.code.content.char_to_line(self.cursor);
        let line_start = self.code.content.line_to_char(row);
        let col = self.cursor - line_start;
        (row, col)
    }

    fn scroll_to_cursor(&mut self) {
        let (cursor_row, _) = self.position();

        if cursor_row >= self.offset_y + self.height {
            self.offset_y = cursor_row - self.height + 1;
        } else if cursor_row < self.offset_y {
            self.offset_y = cursor_row;
        }
    }

    fn move_left(&mut self, shift: bool) {
        if self.cursor > 0 {
            let new_cursor = self.cursor - 1;
            self.update_selection(new_cursor, shift);
            self.cursor = new_cursor;
        }
    }

    fn move_right(&mut self, shift: bool) {
        if self.cursor < self.code.content.len_chars() {
            let new_cursor = self.cursor + 1;
            self.update_selection(new_cursor, shift);
            self.cursor = new_cursor;
        }
    }

    fn move_up(&mut self, shift: bool) {
        let (row, col) = self.position();
        if row > 0 {
            let prev_line_start = self.code.content.line_to_char(row - 1);
            let prev_line_len = self.code.content.line(row - 1).len_chars();
            let new_col = col.min(prev_line_len);
            let new_cursor = prev_line_start + new_col;
            self.update_selection(new_cursor, shift);
            self.cursor = new_cursor;
        }
    }

    fn move_down(&mut self, shift: bool) {
        let (row, col) = self.position();
        if row + 1 < self.code.content.len_lines() {
            let next_line_start = self.code.content.line_to_char(row + 1);
            let next_line_len = self.code.content.line(row + 1).len_chars();
            let new_col = col.min(next_line_len);
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


    fn insert_text(&mut self, text: &str) {
        self.code.begin_batch();
        self.code.insert(self.cursor, text);
        self.code.commit_batch();
        self.cursor += text.chars().count();
    }

    fn delete_char(&mut self) {
        if self.cursor > 0 {
            self.delete_text(self.cursor - 1, self.cursor);
        }
    }

    fn delete_text(&mut self, from: usize, to: usize) {
        self.code.begin_batch();
        self.code.remove(from, to);
        self.code.commit_batch();
        self.cursor = from;
    }
    
    fn undo(&mut self) {
        let edits = self.code.undo();
        if let Some(edits) = edits {
            for edit in edits.iter().rev()  {
                match &edit.kind {
                    EditKind::Insert { offset, text } => {
                        self.cursor = *offset;
                    }
                    EditKind::Remove { offset, text } => {
                        self.cursor = *offset + text.chars().count();
                    }
                }
            }
        }
    }
    
    fn redo(&mut self) {
        let edits = self.code.redo();
        if let Some(edits) = edits {
            for edit in edits {
                match &edit.kind {
                    EditKind::Insert { offset, text } => {
                        self.cursor = *offset + text.chars().count();
                    }
                    EditKind::Remove { offset, text } => {
                        self.cursor = *offset;
                    }
                }
            }
        }
    }

    pub fn scroll_up(&mut self) {
        if self.offset_y > 0 {
            self.offset_y -= 1;
        }
    }

    pub fn scroll_down(&mut self, area_height: usize) {
        let max_offset = self.code.content.len_lines().saturating_sub(area_height);
        if self.offset_y < max_offset {
            self.offset_y += 1;
        }
    }

    fn build_theme(theme: &Vec<(&str, &str)>) -> HashMap<String, Style> {
        theme.into_iter()
            .map(|(name, hex)| {
                let hex = hex.trim_start_matches('#');
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                
                (name.to_string(), Style::default().fg(Color::Rgb(r, g, b)))
            })
            .collect()
    }
    
    pub fn get_content(&self) -> String {
        self.code.content.to_string()
    }

}

impl Widget for &Editor {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let total_lines = self.code.content.len_lines();
        let max_line_number = total_lines.max(1);
        let line_number_digits = max_line_number.to_string().len();
        let line_number_width = line_number_digits + 2; 

        let (cursor_line, cursor_char_col) = self.position();
        let mut draw_y = area.top();

        let start_line = self.offset_y;

        let mut max_line_number = 0;

        // draw line numbers and text
        for line_idx in self.offset_y..total_lines {
            if draw_y >= area.bottom() {
                break;
            }

            let line = self.code.content.line(line_idx);

            let line_number = format!("{:>width$}  ", line_idx + 1, width = line_number_digits);

            if area.left() < area.left() + area.width && draw_y < area.top() + area.height {
                buf.set_string(
                    area.left(),
                    draw_y,
                    &line_number,
                    Style::default().fg(Color::DarkGray),
                );
            }

            let max_text_width = (area.width as usize).saturating_sub(line_number_width);
            let displayed_line: String = line.chars().take(max_text_width).collect();

            let text_x = area.left() + line_number_width as u16;
            if text_x < area.left() + area.width && draw_y < area.top() + area.height {
                buf.set_string(
                    text_x,
                    draw_y,
                    &displayed_line,
                    Style::default().fg(Color::White),
                );
            }

            draw_y += 1;
            max_line_number = line_idx;
        }

        /*
            This code block is responsible for rendering syntax highlighting within the visible area of the editor.
            The editor retrieves all relevant syntax captures (query_matches) within the visible line range. 
            For each capture, it:
                - gets the corresponding style from the theme,
                - converts byte positions to screen coordinates,
                - iterates over each character in the captured range,
                - and applies the style to each visible character cell in the buffer.
        */
        if self.code.is_highlightable() {
            let end_line = max_line_number + 1;

            let allowed = self.theme.keys().collect::<Vec<_>>();
            let matches = self.code.query_matches(start_line, end_line, &allowed);

            for (start_byte, end_byte, capture_name) in matches {
                let style = match self.theme.get(&capture_name) {
                    Some(s) => *s, None => continue,
                };
                let start_offset = self.code.content.byte_to_char(start_byte);
                let end_offset = self.code.content.byte_to_char(end_byte);

                let start_line_idx = self.code.content.byte_to_line(start_byte);
                let end_line_idx = self.code.content.byte_to_line(end_byte);

                let start_line_offset = self.code.content.line_to_char(start_line_idx);
                let end_line_offset = self.code.content.line_to_char(end_line_idx);

                let start_col = start_offset - start_line_offset;
                let end_col = end_offset - start_line_offset;

                let content = self.code.content.byte_slice(start_byte..end_byte).to_string();

                let mut x = start_col;
                let mut y = start_line_idx;

                for ch in content.chars() {
                    if ch == '\n' { y += 1; x = 0; continue; }

                    let not_visible = y < self.offset_y || y >= self.offset_y + area.height as usize;
                    if not_visible {
                        x += UnicodeWidthChar::width(ch).unwrap_or(0);
                        continue;
                    }

                    let draw_y = area.top() + (y - self.offset_y) as u16;
                    let draw_x = area.left() + line_number_width as u16 + x as u16;

                    if draw_x < area.left() + area.width && draw_y < area.top() + area.height {
                        buf[(draw_x, draw_y)].set_style(style);
                    }

                    x += UnicodeWidthChar::width(ch).unwrap_or(0);
                }
            }
        }
        
        if let Some(selection) = self.selection {
            
            let start = selection.start.min(selection.end);
            let end = selection.start.max(selection.end);
        
            let start_line = self.code.content.char_to_line(start);
            let end_line = self.code.content.char_to_line(end);
        
            for line_idx in start_line..=end_line {
                if line_idx < self.offset_y {
                    continue; // not visible
                }
                if line_idx >= self.offset_y + area.height as usize {
                    break; // not visible
                }
        
                let line = self.code.content.line(line_idx);
                let line_start = self.code.content.line_to_char(line_idx);
                let line_end = line_start + line.len_chars();
        
                let sel_start = start.max(line_start);
                let sel_end = end.min(line_end);
        
                let rel_start = sel_start - line_start;
                let rel_end = sel_end - line_start;
        
                let draw_y = area.top() + (line_idx - self.offset_y) as u16;
                let mut x = 0;
                let mut char_idx = 0;
        
                for ch in line.chars() {
                    if char_idx >= rel_start && char_idx < rel_end {
                        let draw_x = area.left() + line_number_width as u16 + x;
                        if draw_x < area.right() && draw_y < area.bottom() {
                            buf[(draw_x, draw_y)]
                                .set_style(Style::default().bg(Color::DarkGray));
                        }
                    }
        
                    x += UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
                    char_idx += 1;
                }
            }
        }


        // draw the cursor like a block
        if cursor_line >= self.offset_y && cursor_line < self.offset_y + area.height as usize {
            let line = self.code.content.line(cursor_line);
            let max_text_width = (area.width as usize).saturating_sub(line_number_width);
            let cursor_visual_col = line
                .chars()
                .take(cursor_char_col)
                .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0))
                .sum::<usize>()
                .min(max_text_width);

            let cursor_x = area.left() + line_number_width as u16 + cursor_visual_col as u16;
            let cursor_y = area.top() + (cursor_line - self.offset_y) as u16;

            if cursor_x < area.left() + area.width && cursor_y < area.top() + area.height {
                buf[(cursor_x, cursor_y)].set_style(Style::default().bg(Color::White).fg(Color::Black));
            }
        }
    }
}


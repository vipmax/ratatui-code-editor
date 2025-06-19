use ropey::Rope;
use ratatui::{prelude::*, widgets::Widget};
use crossterm::event::KeyEvent;
use unicode_width::UnicodeWidthChar;

pub struct Editor {
    pub name: String,
    pub content: Rope,
    pub cursor: usize,
    pub offset_y: usize,
}

impl Editor {
    pub fn new(name: &str, text: &str) -> Self {
        Self {
            name: name.to_string(),
            content: Rope::from_str(text),
            cursor: 0,
            offset_y: 0,
        }
    }

    fn cursor_pos(&self) -> (usize, usize) {
        let row = self.content.char_to_line(self.cursor);
        let line_start = self.content.line_to_char(row);
        let col = self.cursor - line_start;
        (row, col)
    }

    pub fn input(&mut self, key: KeyEvent, area_height: usize) {
        use crossterm::event::KeyCode::*;

        match key.code {
            Left => self.move_left(),
            Right => self.move_right(),
            Up => self.move_up(),
            Down => self.move_down(),
            Backspace => self.delete_char(),
            Enter => self.insert_char('\n'),
            Char(c) => self.insert_char(c),
            _ => {}
        }

        self.scroll_to_cursor(area_height);
    }

    fn scroll_to_cursor(&mut self, area_height: usize) {
        let (cursor_row, _) = self.cursor_pos();

        if cursor_row >= self.offset_y + area_height {
            self.offset_y = cursor_row - area_height + 1;
        }
        else if cursor_row < self.offset_y {
            self.offset_y = cursor_row;
        }
    }

    fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn move_right(&mut self) {
        if self.cursor < self.content.len_chars() {
            self.cursor += 1;
        }
    }

    fn move_up(&mut self) {
        let (row, col) = self.cursor_pos();
        if row > 0 {
            let prev_line_start = self.content.line_to_char(row - 1);
            let prev_line_len = self.content.line(row - 1).len_chars();
            let new_col = col.min(prev_line_len);
            self.cursor = prev_line_start + new_col;
        }
    }

    fn move_down(&mut self) {
        let (row, col) = self.cursor_pos();
        if row + 1 < self.content.len_lines() {
            let next_line_start = self.content.line_to_char(row + 1);
            let next_line_len = self.content.line(row + 1).len_chars();
            let new_col = col.min(next_line_len);
            self.cursor = next_line_start + new_col;
        }
    }

    fn insert_char(&mut self, c: char) {
        self.content.insert_char(self.cursor, c);
        self.cursor += 1;
    }

    fn delete_char(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.content.remove(self.cursor..self.cursor + 1);
        }
    }
    
    pub fn scroll_up(&mut self) {
        if self.offset_y > 0 {
            self.offset_y -= 1;
        }
    }
    
    pub fn scroll_down(&mut self, area_height: usize) {
        let max_offset = self.content.len_lines().saturating_sub(area_height);
        if self.offset_y < max_offset {
            self.offset_y += 1;
        }
    }
    
    pub fn click(&mut self, mouse_x: u16, mouse_y: u16, area: Rect) {
        let total_lines = self.content.len_lines();
        let max_line_number = total_lines.max(1);
        let line_number_digits = max_line_number.to_string().len();
        let line_number_width = (line_number_digits + 2) as u16;
    
        if mouse_y < area.top() || mouse_y >= area.bottom() || mouse_x < area.left() + line_number_width {
            return;
        }
    
        let clicked_row = (mouse_y - area.top()) as usize + self.offset_y;
        if clicked_row >= self.content.len_lines() {
            return;
        }
    
        let clicked_col = (mouse_x - area.left() - line_number_width) as usize;
        let line = self.content.line(clicked_row);
    
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
    
        let line_visual_width: usize = line.chars()
            .map(|ch| unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0))
            .sum();
    
        if clicked_col >= line_visual_width {
            // Fix to prevent cursor jumping to next line
            let mut end_idx = line.len_chars();
            if end_idx > 0 && line.char(end_idx - 1) == '\n' {
                end_idx -= 1;
            }
            char_idx = end_idx;
        }
    
        let line_start = self.content.line_to_char(clicked_row);
        self.cursor = line_start + char_idx;
    }



}
impl Widget for &Editor {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let total_lines = self.content.len_lines();
        let max_line_number = total_lines.max(1);
        let line_number_digits = max_line_number.to_string().len();
        let line_number_width = line_number_digits + 2; // digits + 2 spaces (1 for padding, 1 extra)

        let (cursor_line, cursor_char_col) = self.cursor_pos();
        let mut draw_y = area.top();

        for line_idx in self.offset_y..total_lines {
            if draw_y >= area.bottom() {
                break;
            }

            let line = self.content.line(line_idx);

            // Format line number with extra space after it
            let line_number = format!("{:>width$}  ", line_idx + 1, width = line_number_digits);

            // Draw the line number in dark gray
            buf.set_string(
                area.left(),
                draw_y,
                &line_number,
                Style::default().fg(Color::DarkGray),
            );

            // Calculate width before cursor on this line
            let cursor_visual_col = if line_idx == cursor_line {
                line.chars()
                    .take(cursor_char_col)
                    .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0))
                    .sum::<usize>()
                    .min((area.width as usize).saturating_sub(line_number_width))
            } else {
                0
            };

            // Truncate line content to fit
            let max_text_width = (area.width as usize).saturating_sub(line_number_width);
            let displayed_line: String = line.chars().take(max_text_width).collect();

            // Draw the code line
            buf.set_string(
                area.left() + line_number_width as u16,
                draw_y,
                &displayed_line,
                Style::default().fg(Color::White),
            );

            // Draw cursor
            if line_idx == cursor_line {
                let cursor_x = area.left() + line_number_width as u16 + cursor_visual_col as u16;
                buf[(cursor_x, draw_y)].set_style(Style::default().bg(Color::White).fg(Color::Black));
            }

            draw_y += 1;
        }
    }
}


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
}

impl Widget for &Editor {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Determine the cursor's line and column indices relative to the text buffer
        let (cursor_line, cursor_char_col) = self.cursor_pos();
        let mut draw_y = area.top();

        // Iterate over lines starting from the vertical scroll offset up to the last line
        for line_idx in self.offset_y..self.content.len_lines() {
            // Stop drawing if we reached the bottom of the allocated area
            if draw_y >= area.bottom() {
                break;
            }

            let line = self.content.line(line_idx);

            // Compute the cursor's visual column (width) on the current line, accounting for wide chars
            let cursor_visual_col = if line_idx == cursor_line {
                line.chars()
                    .take(cursor_char_col)
                    .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0))
                    .sum::<usize>()
                    .min(area.width as usize)
            } else {
                0
            };

            // Collect line's characters truncated to the terminal width to avoid overflow
            let displayed_line: String = line.chars().take(area.width as usize).collect();

            // Render the line with default white foreground color
            buf.set_string(area.left(), draw_y, &displayed_line, Style::default().fg(Color::White));

            // Highlight cursor position by inverting fg/bg colors if on this line
            if line_idx == cursor_line {
                let cursor_x = area.left() + cursor_visual_col as u16;
                buf[(cursor_x, draw_y)].set_style(Style::default().bg(Color::White).fg(Color::Black));
            }

            draw_y += 1;
        }
    }
}


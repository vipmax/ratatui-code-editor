use crossterm::event::KeyEvent;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::{prelude::*, widgets::Widget};
use std::collections::HashMap;
use unicode_width::UnicodeWidthChar;

use crate::code::Code;

pub struct Editor {
    pub name: String,
    pub code: Code,
    pub cursor: usize,
    pub offset_y: usize,
}

impl Editor {
    pub fn new(name: &str, lang: &str, text: &str) -> Self {
        let code = Code::new(text, lang)
            .or_else(|_| Code::new(text, "text"))
            .unwrap();

        Self {
            name: name.to_string(),
            code,
            cursor: 0,
            offset_y: 0,
        }
    }

    fn cursor_pos(&self) -> (usize, usize) {
        let row = self.code.content.char_to_line(self.cursor);
        let line_start = self.code.content.line_to_char(row);
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
            Enter => self.insert_text("\n"),
            Char(c) => self.insert_text(&c.to_string()),
            _ => {}
        }

        self.scroll_to_cursor(area_height);
    }

    fn scroll_to_cursor(&mut self, area_height: usize) {
        let (cursor_row, _) = self.cursor_pos();

        if cursor_row >= self.offset_y + area_height {
            self.offset_y = cursor_row - area_height + 1;
        } else if cursor_row < self.offset_y {
            self.offset_y = cursor_row;
        }
    }

    fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn move_right(&mut self) {
        if self.cursor < self.code.content.len_chars() {
            self.cursor += 1;
        }
    }

    fn move_up(&mut self) {
        let (row, col) = self.cursor_pos();
        if row > 0 {
            let prev_line_start = self.code.content.line_to_char(row - 1);
            let prev_line_len = self.code.content.line(row - 1).len_chars();
            let new_col = col.min(prev_line_len);
            self.cursor = prev_line_start + new_col;
        }
    }

    fn move_down(&mut self) {
        let (row, col) = self.cursor_pos();
        if row + 1 < self.code.content.len_lines() {
            let next_line_start = self.code.content.line_to_char(row + 1);
            let next_line_len = self.code.content.line(row + 1).len_chars();
            let new_col = col.min(next_line_len);
            self.cursor = next_line_start + new_col;
        }
    }

    fn insert_text(&mut self, text: &str) {
        self.code.insert(self.cursor, text);
        self.cursor += text.chars().count();
    }

    fn delete_char(&mut self) {
        if self.cursor > 0 {
            self.delete_text(self.cursor - 1, self.cursor);
        }
    }

    fn delete_text(&mut self, from: usize, to: usize) {
        self.code.remove(from, to);
        self.cursor = from;
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

    pub fn click(&mut self, mouse_x: u16, mouse_y: u16, area: Rect) {
        let total_lines = self.code.content.len_lines();
        let max_line_number = total_lines.max(1);
        let line_number_digits = max_line_number.to_string().len();
        let line_number_width = (line_number_digits + 2) as u16;

        if mouse_y < area.top()
            || mouse_y >= area.bottom()
            || mouse_x < area.left() + line_number_width
        {
            return;
        }

        let clicked_row = (mouse_y - area.top()) as usize + self.offset_y;
        if clicked_row >= self.code.content.len_lines() {
            return;
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
            // Fix to prevent cursor jumping to next line
            let mut end_idx = line.len_chars();
            if end_idx > 0 && line.char(end_idx - 1) == '\n' {
                end_idx -= 1;
            }
            char_idx = end_idx;
        }

        let line_start = self.code.content.line_to_char(clicked_row);
        self.cursor = line_start + char_idx;
    }

    pub fn hex_to_rgb(hex_color: &str) -> (u8, u8, u8) {
        let hex = hex_color.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        (r, g, b)
    }

    pub fn build_theme(&self) -> HashMap<String, Style> {
        [
            ("identifier", "#A5FCB6"),
            ("field_identifier", "#A5FCB6"),
            ("property_identifier", "#A5FCB6"),
            ("property", "#A5FCB6"),
            ("string", "#b1fce5"),
            ("keyword", "#a0a0a0"),
            ("constant", "#f6c99f"),
            ("number", "#f6c99f"),
            ("integer", "#f6c99f"),
            ("float", "#f6c99f"),
            ("variable", "#ffffff"),
            ("variable.builtin", "#ffffff"),
            ("function", "#f6c99f"),
            ("function.call", "#f6c99f"),
            ("method", "#f6c99f"),
            ("comment", "#585858"),
            ("namespace", "#f6c99f"),
            ("type", "#f6c99f"),
            ("type.builtin", "#f6c99f"),
            ("tag.attribute", "#c6a5fc"),
            ("tag", "#c6a5fc"),
            ("error", "#A5FCB6"),
        ]
        .into_iter()
        .map(|(name, hex)| {
            let (r, g, b) = Self::hex_to_rgb(hex);
            (name.to_string(), Style::default().fg(Color::Rgb(r, g, b)))
        })
        .collect()
    }
}

impl Widget for &Editor {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let total_lines = self.code.content.len_lines();
        let max_line_number = total_lines.max(1);
        let line_number_digits = max_line_number.to_string().len();
        let line_number_width = line_number_digits + 2; 

        let (cursor_line, cursor_char_col) = self.cursor_pos();
        let mut draw_y = area.top();

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

        if self.code.is_highlighted() {
            // code highlighting over the text
            let start_line = self.offset_y;
            let end_line = max_line_number + 1;

            let theme = self.build_theme();
            let allowed = theme.keys().collect::<Vec<_>>();

            let matches = self.code.query_matches(start_line, end_line, &allowed);

            for (_, _, start_pos, end_pos, capture_name) in matches {
                // check if the capture is visible in the area
                if end_pos.row < start_line || start_pos.row >= end_line {
                    continue;
                }

                let style = match theme.get(&capture_name) {
                    Some(s) => *s,
                    None => continue,
                };

                for line_idx in start_pos.row..=end_pos.row {
                    if line_idx < start_line {
                        continue;
                    }
                    let draw_y = area.top() + (line_idx - start_line) as u16;
                    if draw_y >= area.bottom() {
                        break;
                    }

                    let line = self.code.content.line(line_idx);
                    let line_len = line.len_chars();
                    let max_text_width = (area.width as usize).saturating_sub(line_number_width);
                    let line_str: String = line.chars().take(max_text_width).collect();

                    let start_col = if line_idx == start_pos.row {
                        start_pos.column
                    } else {
                        0
                    };

                    let end_col = if line_idx == end_pos.row {
                        end_pos.column
                    } else {
                        line_len.min(max_text_width)
                    };

                    // calculate the width of each character
                    let mut current_width = 0;
                    let mut start_x = None;
                    let mut end_x = None;
                    for (i, ch) in line_str.chars().enumerate() {
                        let w = UnicodeWidthChar::width(ch).unwrap_or(0);
                        if i == start_col {
                            start_x = Some(current_width);
                        }
                        if i == end_col {
                            end_x = Some(current_width);
                            break;
                        }
                        current_width += w;
                    }

                    if end_x.is_none() {
                        end_x = Some(current_width);
                    }

                    if let (Some(start_x), Some(end_x)) = (start_x, end_x) {
                        let base_x = area.left() + line_number_width as u16;
                        for x in start_x..end_x {
                            let pos_x = base_x + x as u16;
                            if pos_x < area.left() + area.width && draw_y < area.top() + area.height {
                                buf[(pos_x, draw_y)].set_style(style);
                            }
                        }
                    }
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


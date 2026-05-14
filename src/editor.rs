use crate::actions::*;
use crate::click::{ClickKind, ClickTracker};
use crate::code::Code;
use crate::code::{EditBatch, Operation};
use crate::code::{RopeGraphemes, grapheme_width, grapheme_width_and_chars_len};
use crate::selection::{Selection, SelectionSnap};
use crate::types::{HightlightCache, Theme, VisualRow};
use crate::utils;
use anyhow::{Result, anyhow};
use ratatui_core::layout::Rect;
use ratatui_core::style::{Color, Style};
use similar::{ChangeTag, TextDiff};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Duration;

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

    /// Controls when to show the line numbers
    pub(crate) show_line_numbers: bool,

    /// Controls the left padding before writing the code
    pub(crate) left_code_padding: usize,

    /// Runtime toggle for diff rendering and visual rows.
    pub(crate) diff_enabled: bool,

    /// Runtime toggle for showing only changed diff rows with surrounding context.
    pub(crate) diff_focus_enabled: bool,

    /// Number of unchanged visual rows shown around focused diff rows.
    pub(crate) diff_focus_context: usize,

    /// Original code snapshot used for diff and ghost-line highlighting.
    pub(crate) original_code: Option<Code>,

    /// Visual rows model (real + ghost deleted rows) used for stable scrolling.
    pub(crate) visual_rows: RefCell<Vec<VisualRow>>,
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
            show_line_numbers: true,
            left_code_padding: 2,
            diff_enabled: false,
            diff_focus_enabled: false,
            diff_focus_context: 3,
            original_code: None,
            visual_rows: RefCell::new(Vec::new()),
        })
    }

    pub(crate) fn get_line_number_width(&self) -> usize {
        if self.show_line_numbers {
            let total_lines = self.code.len_lines();
            let max_line_number = total_lines.max(1);
            let line_number_digits = max_line_number.to_string().len().max(5);
            (line_number_digits + self.left_code_padding) as usize
        } else {
            self.left_code_padding
        }
    }

    pub fn focus(&mut self, area: &Rect) {
        self.fit_cursor();
        self.clamp_cursor_to_focus_rows();
        self.clamp_offset_y();

        let width = area.width as usize;
        let height = area.height as usize;
        let line_number_width = self.get_line_number_width();

        let line = self.code.char_to_line(self.cursor);
        let col = self.cursor - self.code.line_to_char(line);
        let visual_line = self.visual_line_idx(line);

        let visible_width = width.saturating_sub(line_number_width);
        let visible_height = height;

        let step_size = 10;
        if col < self.offset_x {
            self.offset_x = col.saturating_sub(step_size);
        } else if col >= self.offset_x + visible_width {
            self.offset_x = col.saturating_sub(visible_width.saturating_sub(step_size));
        }

        if visual_line < self.offset_y {
            self.offset_y = visual_line;
        } else if visual_line >= self.offset_y + visible_height {
            self.offset_y = visual_line.saturating_sub(visible_height - 1);
        }
    }

    /// Handles a mouse button press at the given cursor position, updating selection and click state.
    pub fn handle_mouse_down(&mut self, cursor: usize) {
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

    /// Handles a mouse drag event at the given cursor position, extending the selection.
    pub fn handle_mouse_drag(&mut self, cursor: usize) {
        match self.selection_snap {
            SelectionSnap::Line { anchor } => {
                let (anchor_start, anchor_end) = self.code.line_boundaries(anchor);
                let (cur_start, cur_end) = self.code.line_boundaries(cursor);

                let (sel_start, sel_end, new_cursor) = match cursor.cmp(&anchor) {
                    Ordering::Greater => (anchor_start, cur_end, cur_end), // forward
                    Ordering::Less => (cur_start, anchor_end, cur_start),  // backward
                    Ordering::Equal => (anchor_start, anchor_end, anchor_end),
                };

                self.selection = Some(Selection::from_anchor_and_cursor(sel_start, sel_end));
                self.cursor = new_cursor;
            }
            SelectionSnap::Word { anchor } => {
                let (anchor_start, anchor_end) = self.code.word_boundaries(anchor);
                let (cur_start, cur_end) = self.code.word_boundaries(cursor);

                let (sel_start, sel_end, new_cursor) = match cursor.cmp(&anchor) {
                    Ordering::Greater => (anchor_start, cur_end, cur_end), // forward
                    Ordering::Less => (cur_start, anchor_end, cur_start),  // backward
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

    /// Converts mouse coordinates to a cursor position within the editor area, returning `None` if outside.
    pub fn cursor_from_mouse(&self, mouse_x: u16, mouse_y: u16, area: &Rect) -> Option<usize> {
        let line_number_width = self.get_line_number_width() as u16;

        if mouse_y < area.top()
            || mouse_y >= area.bottom()
            || mouse_x < area.left() + line_number_width
        {
            return None;
        }

        let clicked_visual_row = (mouse_y - area.top()) as usize + self.offset_y;
        let clicked_row = self.real_line_for_visual_row(clicked_visual_row);
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
            if current_col + g_width > clicked_col {
                break;
            }
            current_col += g_width;
            char_idx += g_chars;
        }

        let line = self
            .code
            .char_slice(line_start_char, line_start_char + line_len);
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
            .map(|s| {
                if self.cursor == s.start {
                    s.end
                } else {
                    s.start
                }
            })
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

    pub fn set_original_code(&mut self, content: &str) -> Result<()> {
        let original = Code::new(content, self.code_ref().lang(), None)
            .or_else(|_| Code::new(content, "text", None))?;
        self.highlights_cache.borrow_mut().clear();
        self.original_code = Some(original);
        self.rebuild_visual_rows();
        Ok(())
    }

    pub fn clear_original_code(&mut self) {
        self.highlights_cache.borrow_mut().clear();
        self.original_code = None;
        self.visual_rows.borrow_mut().clear();
        self.offset_y = 0;
    }

    pub fn has_diff(&self) -> bool {
        self.diff_enabled && self.original_code.is_some()
    }

    pub fn set_diff_enabled(&mut self, enabled: bool) {
        self.diff_enabled = enabled;
        self.rebuild_visual_rows();
        self.clamp_offset_y();
    }

    pub fn is_diff_enabled(&self) -> bool {
        self.diff_enabled
    }

    pub fn set_diff_focus_enabled(&mut self, enabled: bool) {
        self.diff_focus_enabled = enabled;
        self.rebuild_visual_rows();
        self.clamp_cursor_to_focus_rows();
        self.clamp_offset_y();
    }

    pub fn toggle_diff_focus(&mut self) {
        self.set_diff_focus_enabled(!self.diff_focus_enabled);
    }

    pub fn is_diff_focus_enabled(&self) -> bool {
        self.diff_focus_enabled
    }

    pub fn set_diff_focus_context(&mut self, context_lines: usize) {
        self.diff_focus_context = context_lines;
        self.rebuild_visual_rows();
        self.clamp_cursor_to_focus_rows();
        self.clamp_offset_y();
    }

    pub fn diff_focus_context(&self) -> usize {
        self.diff_focus_context
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
            match edit.operation {
                Operation::Insert => {
                    self.code.insert(edit.start, &edit.text);
                }
                Operation::Remove => {
                    self.code
                        .remove(edit.start, edit.start + edit.text.chars().count());
                }
            }
        }
        self.code.commit();
        self.reset_highlight_cache();
    }

    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor;
        self.fit_cursor();
        self.clamp_cursor_to_focus_rows();
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
        let len_lines = self.visual_len_lines();
        if self.offset_y < len_lines.saturating_sub(area_height) {
            self.offset_y += 1;
        }
    }

    fn build_theme(theme: &Vec<(&str, &str)>) -> Theme {
        theme
            .into_iter()
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
            marks
                .into_iter()
                .map(|(start, end, color)| {
                    let (r, g, b) = utils::rgb(color);
                    (start, end, Color::Rgb(r, g, b))
                })
                .collect(),
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
        if let Some(selection) = &self.selection
            && !selection.is_empty()
        {
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
        self.offset_y = offset_y.min(self.visual_len_lines().saturating_sub(1));
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

    pub(crate) fn visual_len_lines(&self) -> usize {
        if self.has_diff() {
            return self.visual_rows.borrow().len().max(1);
        }
        self.code.len_lines().max(1)
    }

    pub(crate) fn real_line_for_visual_row(&self, visual_row: usize) -> usize {
        let last = self.code.len_lines().saturating_sub(1);
        if !self.has_diff() {
            return visual_row.min(last);
        }

        self.visual_rows
            .borrow()
            .get(visual_row)
            .map(|row| match row {
                VisualRow::Real { line_idx, .. } => *line_idx,
                VisualRow::FoldSeparator { .. } => last,
                VisualRow::GhostDeleted { anchor_line, .. } => {
                    anchor_line.saturating_sub(1).min(last)
                }
            })
            .unwrap_or(last)
    }

    pub(crate) fn visual_line_idx(&self, line_idx: usize) -> usize {
        if self.has_diff() {
            let rows = self.visual_rows.borrow();
            return rows
                .iter()
                .position(
                    |row| matches!(row, VisualRow::Real { line_idx: idx, .. } if *idx == line_idx),
                )
                .unwrap_or(line_idx);
        }
        line_idx
    }

    pub fn code_mut(&mut self) -> &mut Code {
        &mut self.code
    }

    pub fn code_ref(&self) -> &Code {
        &self.code
    }

    /// Set the change callback function for handling document changes
    pub fn set_change_callback(
        &mut self,
        callback: Box<dyn Fn(Vec<(usize, usize, usize, usize, String)>)>,
    ) {
        self.code.set_change_callback(callback);
    }

    pub fn highlight_interval(
        &self,
        start: usize,
        end: usize,
        theme: &Theme,
    ) -> Vec<(usize, usize, Style)> {
        let mut cache = self.highlights_cache.borrow_mut();
        let key = (0, start, end);
        if let Some(v) = cache.get(&key) {
            return v.clone();
        }

        let highlights = self.code.highlight_interval(start, end, theme);
        cache.insert(key, highlights.clone());
        highlights
    }

    pub fn highlight_interval_original(
        &self,
        start: usize,
        end: usize,
        theme: &Theme,
    ) -> Vec<(usize, usize, Style)> {
        let Some(original) = &self.original_code else {
            return Vec::new();
        };
        let mut cache = self.highlights_cache.borrow_mut();
        let key = (1, start, end);
        if let Some(v) = cache.get(&key) {
            return v.clone();
        }

        let highlights = original.highlight_interval(start, end, theme);
        cache.insert(key, highlights.clone());
        highlights
    }

    pub fn reset_highlight_cache(&self) {
        self.highlights_cache.borrow_mut().clear();
        self.rebuild_visual_rows();
    }

    fn clamp_offset_y(&mut self) {
        self.offset_y = self.offset_y.min(self.visual_len_lines().saturating_sub(1));
    }

    pub(crate) fn previous_focus_real_line(&self, line_idx: usize) -> Option<usize> {
        if !self.is_diff_focus_active() {
            return None;
        }

        self.visual_rows.borrow().iter().rev().find_map(|row| {
            if let VisualRow::Real { line_idx: idx, .. } = row
                && *idx < line_idx
            {
                return Some(*idx);
            }
            None
        })
    }

    pub(crate) fn next_focus_real_line(&self, line_idx: usize) -> Option<usize> {
        if !self.is_diff_focus_active() {
            return None;
        }

        self.visual_rows.borrow().iter().find_map(|row| {
            if let VisualRow::Real { line_idx: idx, .. } = row
                && *idx > line_idx
            {
                return Some(*idx);
            }
            None
        })
    }

    pub(crate) fn is_diff_focus_active(&self) -> bool {
        self.has_diff() && self.diff_focus_enabled
    }

    fn clamp_cursor_to_focus_rows(&mut self) {
        if !self.is_diff_focus_active() {
            return;
        }

        let (cursor_line, cursor_char_col) = self.code.point(self.cursor);
        if self.focus_real_line_visible(cursor_line) {
            return;
        }

        let current_visual_col = self.code.char_col_to_visual(cursor_line, cursor_char_col);
        let Some(target_line) = self.nearest_focus_real_line(cursor_line) else {
            return;
        };
        let target_start = self.code.line_to_char(target_line);
        let target_len = self.code.line_len(target_line);
        let target_col = self
            .code
            .visual_to_char_col(target_line, current_visual_col)
            .min(target_len);

        self.cursor = target_start + target_col;
        self.clear_selection();
    }

    fn focus_real_line_visible(&self, line_idx: usize) -> bool {
        self.visual_rows
            .borrow()
            .iter()
            .any(|row| matches!(row, VisualRow::Real { line_idx: idx, .. } if *idx == line_idx))
    }

    fn nearest_focus_real_line(&self, line_idx: usize) -> Option<usize> {
        let prev = self.previous_focus_real_line(line_idx);
        let next = self.next_focus_real_line(line_idx);

        match (prev, next) {
            (Some(prev), Some(next)) => {
                if line_idx - prev <= next - line_idx {
                    Some(prev)
                } else {
                    Some(next)
                }
            }
            (Some(prev), None) => Some(prev),
            (None, Some(next)) => Some(next),
            (None, None) => None,
        }
    }

    /// calculates visible cursor position
    pub fn get_visible_cursor(&self, area: &Rect) -> Option<(u16, u16)> {
        let line_number_width = self.get_line_number_width();

        let (cursor_line, cursor_char_col) = self.code.point(self.cursor);
        let cursor_visual_line = self.visual_line_idx(cursor_line);

        if cursor_visual_line >= self.offset_y
            && cursor_visual_line < self.offset_y + area.height as usize
        {
            let line_start_char = self.code.line_to_char(cursor_line);
            let line_len = self.code.line_len(cursor_line);

            let max_x = (area.width as usize).saturating_sub(line_number_width);
            let start_col = self.offset_x;

            let cursor_visual_col: usize = {
                let slice = self.code.char_slice(
                    line_start_char,
                    line_start_char + cursor_char_col.min(line_len),
                );
                RopeGraphemes::new(&slice).map(grapheme_width).sum()
            };

            let offset_visual_col: usize = {
                let slice = self
                    .code
                    .char_slice(line_start_char, line_start_char + start_col.min(line_len));
                RopeGraphemes::new(&slice).map(grapheme_width).sum()
            };

            let relative_visual_col = cursor_visual_col.saturating_sub(offset_visual_col);
            let visible_x = relative_visual_col.min(max_x);

            let cursor_x = area.left() + (line_number_width + visible_x) as u16;
            let cursor_y = area.top() + (cursor_visual_line - self.offset_y) as u16;

            if cursor_x < area.right() && cursor_y < area.bottom() {
                return Some((cursor_x, cursor_y));
            }
        }

        return None;
    }

    pub fn show_line_numbers(&mut self, show: bool) {
        self.show_line_numbers = show
    }

    pub fn set_left_code_padding(&mut self, char_count: usize) {
        self.left_code_padding = char_count
    }

    pub(crate) fn rebuild_visual_rows(&self) {
        if !self.diff_enabled {
            self.visual_rows.borrow_mut().clear();
            return;
        }
        let Some(original) = &self.original_code else {
            self.visual_rows.borrow_mut().clear();
            return;
        };

        let full_rows = self.build_diff_visual_rows(original);
        let rows = if self.diff_focus_enabled {
            Self::focused_diff_rows(&full_rows, self.diff_focus_context)
        } else {
            full_rows
        };

        *self.visual_rows.borrow_mut() = rows;
    }

    fn build_diff_visual_rows(&self, original: &Code) -> Vec<VisualRow> {
        let current = self.code.get_content();
        let original_text = original.get_content();
        let diff = TextDiff::from_lines(&original_text, &current);

        let mut rows = Vec::new();
        let mut current_line_idx = 0usize;
        let mut original_line_idx = 0usize;
        let mut pending_deletes: Vec<(String, usize)> = Vec::new();

        for op in diff.ops() {
            for change in diff.iter_changes(op) {
                match change.tag() {
                    ChangeTag::Delete => {
                        pending_deletes.push((
                            change.to_string().trim_end_matches('\n').to_string(),
                            original_line_idx,
                        ));
                        original_line_idx += 1;
                    }
                    ChangeTag::Insert => {
                        let anchor = current_line_idx + 1;
                        for (text, orig_idx) in pending_deletes.drain(..) {
                            rows.push(VisualRow::GhostDeleted {
                                anchor_line: anchor,
                                text,
                                original_line_idx: orig_idx,
                            });
                        }
                        rows.push(VisualRow::Real {
                            line_idx: current_line_idx,
                            is_added: true,
                        });
                        current_line_idx += 1;
                    }
                    ChangeTag::Equal => {
                        let anchor = current_line_idx + 1;
                        for (text, orig_idx) in pending_deletes.drain(..) {
                            rows.push(VisualRow::GhostDeleted {
                                anchor_line: anchor,
                                text,
                                original_line_idx: orig_idx,
                            });
                        }
                        rows.push(VisualRow::Real {
                            line_idx: current_line_idx,
                            is_added: false,
                        });
                        current_line_idx += 1;
                        original_line_idx += 1;
                    }
                }
            }
        }

        if !pending_deletes.is_empty() {
            let anchor = current_line_idx + 1;
            for (text, orig_idx) in pending_deletes.drain(..) {
                rows.push(VisualRow::GhostDeleted {
                    anchor_line: anchor,
                    text,
                    original_line_idx: orig_idx,
                });
            }
        }

        // Keep a safe fallback mapping for unusual trailing newline cases.
        while current_line_idx < self.code.len_lines() {
            rows.push(VisualRow::Real {
                line_idx: current_line_idx,
                is_added: false,
            });
            current_line_idx += 1;
        }

        rows
    }

    fn focused_diff_rows(rows: &[VisualRow], context_lines: usize) -> Vec<VisualRow> {
        let mut include = vec![false; rows.len()];

        for (idx, row) in rows.iter().enumerate() {
            if row.is_changed() {
                let start = idx.saturating_sub(context_lines);
                let end = (idx + context_lines + 1).min(rows.len());
                for should_include in include.iter_mut().take(end).skip(start) {
                    *should_include = true;
                }
            }
        }

        rows.iter()
            .enumerate()
            .filter_map(|(idx, row)| include[idx].then_some((idx, row)))
            .fold(Vec::new(), |mut focused, (idx, row)| {
                if let Some((prev_idx, _)) = focused.last()
                    && idx > prev_idx + 1
                {
                    focused.push((
                        idx,
                        VisualRow::FoldSeparator {
                            hidden_lines: idx - prev_idx - 1,
                        },
                    ));
                }
                focused.push((idx, row.clone()));
                focused
            })
            .into_iter()
            .map(|(_, row)| row)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real(line_idx: usize, is_added: bool) -> VisualRow {
        VisualRow::Real { line_idx, is_added }
    }

    #[test]
    fn focus_diff_focus_clamps_cursor_before_moving_viewport() {
        let mut editor = Editor::new("text", "0\n1\n2\n3\n4\n5\n", vec![]).unwrap();
        editor.set_diff_enabled(true);
        editor.set_original_code("0\n1\n2\n3\n4\n5\n").unwrap();
        editor.set_diff_focus_enabled(true);
        *editor.visual_rows.borrow_mut() = vec![real(4, true)];
        editor.cursor = editor.code.line_to_char(1);
        editor.offset_y = 99;

        editor.focus(&Rect::new(0, 0, 80, 10));

        assert_eq!(editor.code.point(editor.get_cursor()).0, 4);
        assert_eq!(editor.get_offset_y(), 0);
    }
}

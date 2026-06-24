use crate::code::Code;
use crate::types::VisualRow;
use ropey::RopeSlice;
use similar::{Algorithm, DiffOp};

#[derive(Clone, Copy)]
pub(crate) enum FoldExpandDirection {
    Up,
    Down,
    All,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ViewMode {
    Plain,
    Diff,
    DiffFocus { context_lines: usize },
}

impl ViewMode {
    pub(crate) fn has_diff(self) -> bool {
        !matches!(self, ViewMode::Plain)
    }

    pub(crate) fn is_diff_focus(self) -> bool {
        matches!(self, ViewMode::DiffFocus { .. })
    }
}

#[derive(Default)]
pub(crate) struct View {
    rows: Vec<VisualRow>,
    expanded_hidden_ranges: Vec<(usize, usize)>,
}

impl View {
    pub(crate) fn fold_separator_text(hidden_lines: usize, amount: usize) -> String {
        format!(
            "[+{}up] [+{}down] [show unchanged {} lines]",
            amount, amount, hidden_lines
        )
    }

    pub(crate) fn rows(&self) -> &[VisualRow] {
        &self.rows
    }

    pub(crate) fn clear(&mut self) {
        self.rows.clear();
        self.expanded_hidden_ranges.clear();
    }

    pub(crate) fn rebuild(&mut self, code: &Code, original: Option<&Code>, mode: ViewMode) {
        if !mode.has_diff() {
            self.clear();
            return;
        }

        let Some(original) = original else {
            self.clear();
            return;
        };

        self.rows = match mode {
            ViewMode::Plain => Vec::new(),
            ViewMode::Diff => {
                self.expanded_hidden_ranges.clear();
                Self::build_diff_rows(code, original)
            }
            ViewMode::DiffFocus { context_lines } => {
                let full_rows = Self::build_diff_rows(code, original);
                Self::focused_diff_rows(
                    &full_rows,
                    context_lines,
                    &self.expanded_hidden_ranges,
                )
            }
        };
    }

    pub(crate) fn expand_hidden_at_visual_row(
        &mut self,
        code: &Code,
        original: Option<&Code>,
        mode: ViewMode,
        visual_row: usize,
        clicked_col: usize,
        visible_width: usize,
        amount: usize,
    ) -> bool {
        if !matches!(mode, ViewMode::DiffFocus { .. }) {
            return false;
        }

        let row = match self.rows.get(visual_row) {
            Some(row) => row,
            None => return false,
        };

        let &VisualRow::FoldSeparator {
            hidden_lines,
            hidden_start,
            hidden_end,
        } = row else {
            return false;
        };

        if hidden_start > hidden_end || amount == 0 {
            return false;
        }

        let Some(direction) = Self::fold_expand_direction_for_click(
            hidden_lines, clicked_col, visible_width, amount
        ) else {
            return false;
        };

        let (start, end) = match direction {
            FoldExpandDirection::Up => {
                let end = hidden_start + amount - 1;
                (hidden_start, end.min(hidden_end))
            }
            FoldExpandDirection::Down => {
                let start = hidden_end.saturating_sub(amount.saturating_sub(1));
                (hidden_start.max(start), hidden_end)
            }
            FoldExpandDirection::All => (hidden_start, hidden_end),
        };

        self.add_expanded_hidden_range(start, end);
        self.rebuild(code, original, mode);
        true
    }

    fn fold_expand_direction_for_click(
        hidden_lines: usize,
        clicked_col: usize,
        visible_width: usize,
        amount: usize,
    ) -> Option<FoldExpandDirection> {
        if clicked_col >= visible_width {
            return None;
        }

        let up_label = format!("[+{}up]", amount);
        let down_label = format!("[+{}down]", amount);
        let full_text = Self::fold_separator_text(hidden_lines, amount);

        let up_len = up_label.chars().count();
        let down_len = down_label.chars().count();
        let full_text_len = full_text.chars().count();
        let visible_text_len = visible_width.min(full_text_len);

        let up_end = up_len;
        let down_start = up_end + 1;
        let down_end = down_start + down_len;
        let show_start = down_end + 1;

        if clicked_col < up_end.min(visible_text_len) {
            return Some(FoldExpandDirection::Up);
        }
        if clicked_col >= down_start && clicked_col < down_end.min(visible_text_len) {
            return Some(FoldExpandDirection::Down);
        }
        if clicked_col >= show_start && clicked_col < visible_text_len {
            return Some(FoldExpandDirection::All);
        }

        None
    }

    pub(crate) fn visual_len_lines(&self, code: &Code, mode: ViewMode) -> usize {
        if mode.has_diff() {
            return self.rows.len().max(1);
        }
        code.len_lines().max(1)
    }

    pub(crate) fn line_for_visual_row(
        &self,
        code: &Code,
        mode: ViewMode,
        visual_row: usize,
    ) -> Option<usize> {
        let last = code.len_lines().saturating_sub(1);
        if !mode.has_diff() {
            return Some(visual_row.min(last));
        }

        self.rows.get(visual_row).and_then(|row| match row {
            VisualRow::Real { line_idx, .. } => Some(*line_idx),
            VisualRow::GhostDeleted { anchor_line, .. } => {
                Some(anchor_line.saturating_sub(1).min(last))
            }
            VisualRow::FoldSeparator { .. } => None,
        })
    }

    pub(crate) fn visual_row_for_line(&self, mode: ViewMode, line_idx: usize) -> Option<usize> {
        if !mode.has_diff() {
            return Some(line_idx);
        }

        self.rows.iter().position(
            |row| matches!(row, VisualRow::Real { line_idx: idx, .. } if *idx == line_idx),
        )
    }

    pub(crate) fn line_visible(&self, mode: ViewMode, line_idx: usize) -> bool {
        self.visual_row_for_line(mode, line_idx).is_some()
    }

    pub(crate) fn prev_line(&self, mode: ViewMode, line_idx: usize) -> Option<usize> {
        if !mode.has_diff() {
            return line_idx.checked_sub(1);
        }

        self.rows.iter().rev().find_map(|row| {
            if let VisualRow::Real { line_idx: idx, .. } = row
                && *idx < line_idx
            {
                return Some(*idx);
            }
            None
        })
    }

    pub(crate) fn next_line(&self, code: &Code, mode: ViewMode, line_idx: usize) -> Option<usize> {
        if !mode.has_diff() {
            let next = line_idx + 1;
            return (next < code.len_lines()).then_some(next);
        }

        self.rows.iter().find_map(|row| {
            if let VisualRow::Real { line_idx: idx, .. } = row
                && *idx > line_idx
            {
                return Some(*idx);
            }
            None
        })
    }

    pub(crate) fn nearest_line(
        &self,
        code: &Code,
        mode: ViewMode,
        line_idx: usize,
    ) -> Option<usize> {
        let prev = self.prev_line(mode, line_idx);
        let next = self.next_line(code, mode, line_idx);

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

    fn build_diff_rows(code: &Code, original: &Code) -> Vec<VisualRow> {
        let current_lines: Vec<RopeSlice<'_>> = (0..code.len_lines()).map(|i| code.line(i)).collect();
        let original_lines: Vec<RopeSlice<'_>> = (0..original.len_lines()).map(|i| original.line(i)).collect();
        let diff = similar::capture_diff_slices(Algorithm::Myers, &original_lines, &current_lines);

        let mut rows = Vec::new();
        let mut current_line_idx = 0usize;
        let mut original_line_idx = 0usize;
        let mut pending_deletes: Vec<usize> = Vec::new();

        for op in diff {
            match op {
                DiffOp::Equal { len, .. } => {
                    for _ in 0..len {
                        let anchor_line = current_line_idx + 1;
                        for orig_idx in pending_deletes.drain(..) {
                            rows.push(VisualRow::GhostDeleted {
                                anchor_line,
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
                DiffOp::Delete { old_len, .. } => {
                    for _ in 0..old_len {
                        pending_deletes.push(original_line_idx);
                        original_line_idx += 1;
                    }
                }
                DiffOp::Insert { new_len, .. } => {
                    for _ in 0..new_len {
                        let anchor = current_line_idx + 1;
                        for orig_idx in pending_deletes.drain(..) {
                            rows.push(VisualRow::GhostDeleted {
                                anchor_line: anchor,
                                original_line_idx: orig_idx,
                            });
                        }
                        rows.push(VisualRow::Real {
                            line_idx: current_line_idx,
                            is_added: true,
                        });
                        current_line_idx += 1;
                    }
                }
                DiffOp::Replace { old_len, new_len, .. } => {
                    for _ in 0..old_len {
                        pending_deletes.push(original_line_idx);
                        original_line_idx += 1;
                    }
                    for _ in 0..new_len {
                        let anchor = current_line_idx + 1;
                        for orig_idx in pending_deletes.drain(..) {
                            rows.push(VisualRow::GhostDeleted {
                                anchor_line: anchor,
                                original_line_idx: orig_idx,
                            });
                        }
                        rows.push(VisualRow::Real {
                            line_idx: current_line_idx,
                            is_added: true,
                        });
                        current_line_idx += 1;
                    }
                }
            }
        }

        if !pending_deletes.is_empty() {
            let anchor_line = current_line_idx + 1;
            for original_line_idx in pending_deletes.drain(..) {
                rows.push(VisualRow::GhostDeleted {
                    anchor_line,
                    original_line_idx,
                });
            }
        }

        while current_line_idx < code.len_lines() {
            rows.push(VisualRow::Real {
                line_idx: current_line_idx,
                is_added: false,
            });
            current_line_idx += 1;
        }

        rows
    }

    fn focused_diff_rows(
        rows: &[VisualRow],
        context_lines: usize,
        expanded_hidden_ranges: &[(usize, usize)],
    ) -> Vec<VisualRow> {

        // Build a visibility mask: changed rows plus configured context around them.
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

        // Keep user-expanded hidden regions visible even if they are outside default context.
        for &(start, end) in expanded_hidden_ranges {
            let start = start.min(rows.len().saturating_sub(1));
            let end = end.min(rows.len().saturating_sub(1));
            if start > end {
                continue;
            }
            for should_include in include.iter_mut().take(end + 1).skip(start) {
                *should_include = true;
            }
        }

        let mut result = Vec::new();
        let mut last_included_idx = None;

        for (idx, &is_included) in include.iter().enumerate() {
            if is_included {
                if let Some(last) = last_included_idx {
                    if idx > last + 1 {
                        result.push(VisualRow::FoldSeparator {
                            hidden_lines: idx - last - 1,
                            hidden_start: last + 1,
                            hidden_end: idx - 1,
                        });
                    }
                } else if idx > 0 {
                    result.push(VisualRow::FoldSeparator {
                        hidden_lines: idx,
                        hidden_start: 0,
                        hidden_end: idx - 1,
                    });
                }
                result.push(rows[idx].clone());
                last_included_idx = Some(idx);
            }
        }

        if let Some(last) = last_included_idx {
            if last + 1 < rows.len() {
                result.push(VisualRow::FoldSeparator {
                    hidden_lines: rows.len() - last - 1,
                    hidden_start: last + 1,
                    hidden_end: rows.len() - 1,
                });
            }
        } else {
            return rows.to_vec();
        }

        result
    }

    fn add_expanded_hidden_range(&mut self, start: usize, end: usize) {
        self.expanded_hidden_ranges.push((start.min(end), start.max(end)));
        self.expanded_hidden_ranges.sort_by_key(|(s, _)| *s);

        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (s, e) in self.expanded_hidden_ranges.drain(..) {
            if let Some((_, last_end)) = merged.last_mut() && s <= *last_end + 1 {
                *last_end = (*last_end).max(e);
            } else {
                merged.push((s, e));
            }
        }
        self.expanded_hidden_ranges = merged;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_diff_rows() {
        let code = Code::new("hello\nthere\nworld", "unknown", None).unwrap();
        let original = Code::new("hello\nworld", "unknown", None).unwrap();
        let rows = View::build_diff_rows(&code, &original);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], VisualRow::Real { line_idx: 0, is_added: false });
        assert_eq!(rows[1], VisualRow::Real { line_idx: 1, is_added: true });
        assert_eq!(rows[2], VisualRow::Real { line_idx: 2, is_added: false });
    }
}

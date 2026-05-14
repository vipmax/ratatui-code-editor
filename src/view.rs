use crate::code::Code;
use crate::types::VisualRow;
use similar::{ChangeTag, TextDiff};

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
}

impl View {
    pub(crate) fn rows(&self) -> &[VisualRow] {
        &self.rows
    }

    pub(crate) fn clear(&mut self) {
        self.rows.clear();
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

        let full_rows = Self::build_diff_rows(code, original);
        self.rows = match mode {
            ViewMode::Plain => Vec::new(),
            ViewMode::Diff => full_rows,
            ViewMode::DiffFocus { context_lines } => {
                Self::focused_diff_rows(&full_rows, context_lines)
            }
        };
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
        let current = code.get_content();
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

        while current_line_idx < code.len_lines() {
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

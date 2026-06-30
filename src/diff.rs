use crate::code::Code;
use crate::types::VisualRow;
use ropey::RopeSlice;
use similar::{Algorithm, DiffOp};

/// Builds visual rows for a full diff between the current code and its original snapshot.
///
/// Unchanged and inserted current lines are emitted as [`VisualRow::Real`], while deleted
/// original lines are emitted as [`VisualRow::GhostDeleted`] anchored to the next current line.
///
/// ```ignore
/// let code = Code::new("hello\nthere\nworld", "unknown", None)?;
/// let original = Code::new("hello\nworld", "unknown", None)?;
///
/// let rows = build_diff_rows(&code, &original);
/// assert_eq!(
///     rows,
///     vec![
///         VisualRow::Real { line_idx: 0, is_added: false },
///         VisualRow::Real { line_idx: 1, is_added: true },
///         VisualRow::Real { line_idx: 2, is_added: false },
///     ],
/// );
/// ```
pub(crate) fn build_diff_rows(code: &Code, original: &Code) -> Vec<VisualRow> {
    build_diff_rows_with_algorithm(Algorithm::Myers, code, original)
}

fn build_diff_rows_with_algorithm(
    algorithm: Algorithm,
    code: &Code,
    original: &Code,
) -> Vec<VisualRow> {
    let diff = {
        let original_lines = lines(original);
        let current_lines = lines(code);
        similar::capture_diff_slices(algorithm, &original_lines, &current_lines)
    };

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
            DiffOp::Replace {
                old_len, new_len, ..
            } => {
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

// RopeSlice keeps references into the rope, so this does not copy line text.
fn lines(code: &Code) -> Vec<RopeSlice<'_>> {
    (0..code.len_lines())
        .map(|line_idx| code.line(line_idx))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_diff_rows_for_common_changes() {
        let cases = [
            (
                "unchanged",
                "a\nb\nc",
                "a\nb\nc",
                vec![real(0, false), real(1, false), real(2, false)],
            ),
            (
                "insert in middle",
                "a\nx\nb",
                "a\nb",
                vec![real(0, false), real(1, true), real(2, false)],
            ),
            (
                "delete in middle",
                "a\nc",
                "a\nb\nc",
                vec![real(0, false), ghost(2, 1), real(1, false)],
            ),
            (
                "replace in middle",
                "a\nx\nc",
                "a\nb\nc",
                vec![real(0, false), ghost(2, 1), real(1, true), real(2, false)],
            ),
            (
                "insert at start",
                "x\na\nb",
                "a\nb",
                vec![real(0, true), real(1, false), real(2, false)],
            ),
            (
                "delete at start",
                "b\nc",
                "a\nb\nc",
                vec![ghost(1, 0), real(0, false), real(1, false)],
            ),
            (
                "multiple deletes before insert",
                "a\nx\nd",
                "a\nb\nc\nd",
                vec![
                    real(0, false),
                    ghost(2, 1),
                    ghost(2, 2),
                    real(1, true),
                    real(2, false),
                ],
            ),
            (
                "replace one line with two",
                "a\nx\ny\nc",
                "a\nb\nc",
                vec![
                    real(0, false),
                    ghost(2, 1),
                    real(1, true),
                    real(2, true),
                    real(3, false),
                ],
            ),
            (
                "replace two lines with one",
                "a\nx\nd",
                "a\nb\nc\nd",
                vec![
                    real(0, false),
                    ghost(2, 1),
                    ghost(2, 2),
                    real(1, true),
                    real(2, false),
                ],
            ),
            (
                "separate change hunks",
                "a\nx\nc\ny\ne",
                "a\nb\nc\nd\ne",
                vec![
                    real(0, false),
                    ghost(2, 1),
                    real(1, true),
                    real(2, false),
                    ghost(4, 3),
                    real(3, true),
                    real(4, false),
                ],
            ),
            (
                "delete at end with trailing newline",
                "a\nb\n",
                "a\nb\nc\n",
                vec![real(0, false), real(1, false), ghost(3, 2), real(2, false)],
            ),
        ];

        for (name, current, original, expected) in cases {
            assert_eq!(diff_rows(current, original), expected, "{name}");
        }
    }

    fn diff_rows(current: &str, original: &str) -> Vec<VisualRow> {
        let code = Code::new(current, "unknown", None).unwrap();
        let original = Code::new(original, "unknown", None).unwrap();
        build_diff_rows(&code, &original)
    }

    fn real(line_idx: usize, is_added: bool) -> VisualRow {
        VisualRow::Real { line_idx, is_added }
    }

    fn ghost(anchor_line: usize, original_line_idx: usize) -> VisualRow {
        VisualRow::GhostDeleted {
            anchor_line,
            original_line_idx,
        }
    }
}

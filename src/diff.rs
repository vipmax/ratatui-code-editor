use crate::code::Code;
use crate::types::VisualRow;
use ropey::RopeSlice;
use similar::{Algorithm, DiffOp};

pub(crate) fn build_diff_rows(code: &Code, original: &Code) -> Vec<VisualRow> {
    build_diff_rows_with_algorithm(Algorithm::Myers, code, original)
}

fn build_diff_rows_with_algorithm(
    algorithm: Algorithm,
    code: &Code,
    original: &Code,
) -> Vec<VisualRow> {
    // Keep the temporary RopeSlice vectors scoped to diff calculation, so they
    // are dropped before we start converting DiffOps into VisualRows.
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
    fn test_build_diff_rows() {
        let code = Code::new("hello\nthere\nworld", "unknown", None).unwrap();
        let original = Code::new("hello\nworld", "unknown", None).unwrap();
        let rows = build_diff_rows(&code, &original);
        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows[0],
            VisualRow::Real {
                line_idx: 0,
                is_added: false
            }
        );
        assert_eq!(
            rows[1],
            VisualRow::Real {
                line_idx: 1,
                is_added: true
            }
        );
        assert_eq!(
            rows[2],
            VisualRow::Real {
                line_idx: 2,
                is_added: false
            }
        );
    }
}

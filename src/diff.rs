use crate::code::Code;
use crate::types::{VisualRow, LineDiff};
use ropey::RopeSlice;
use similar::{Algorithm, DiffOp};

pub(crate) fn compute_diff(code: &Code, original: &Code) -> Vec<VisualRow> {
    compute_diff_with_algorithm(Algorithm::Myers, code, original)
}

fn compute_diff_with_algorithm(
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
                            curr_line_idx: None,
                        });
                    }
                    rows.push(VisualRow::Real {
                        line_idx: current_line_idx,
                        is_added: false,
                        orig_line_idx: None,
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
                            curr_line_idx: None,
                        });
                    }
                    rows.push(VisualRow::Real {
                        line_idx: current_line_idx,
                        is_added: true,
                        orig_line_idx: None,
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
                
                let anchor = current_line_idx + 1;
                let total_deletes = pending_deletes.len();
                let drained_deletes: Vec<usize> = pending_deletes.drain(..).collect();
                
                for (i, orig_idx) in drained_deletes.iter().copied().enumerate() {
                    let matched_curr = if i < new_len {
                        Some(current_line_idx + i)
                    } else {
                        None
                    };
                    rows.push(VisualRow::GhostDeleted {
                        anchor_line: anchor,
                        original_line_idx: orig_idx,
                        curr_line_idx: matched_curr,
                    });
                }
                
                for i in 0..new_len {
                    let matched_orig = if i < total_deletes {
                        Some(drained_deletes[i])
                    } else {
                        None
                    };
                    rows.push(VisualRow::Real {
                        line_idx: current_line_idx,
                        is_added: true,
                        orig_line_idx: matched_orig,
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
                curr_line_idx: None,
            });
        }
    }

    while current_line_idx < code.len_lines() {
        rows.push(VisualRow::Real {
            line_idx: current_line_idx,
            is_added: false,
            orig_line_idx: None,
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

pub(crate) fn merge_ranges(ranges: Vec<(usize, usize)>, max_gap: usize) -> Vec<(usize, usize)> {
    if ranges.is_empty() {
        return ranges;
    }

    let mut merged = Vec::new();
    let mut current = ranges[0];

    for next in ranges.into_iter().skip(1) {
        if next.0 <= current.1 + max_gap {
            current.1 = current.1.max(next.1);
        } else {
            merged.push(current);
            current = next;
        }
    }
    merged.push(current);
    merged
}

pub(crate) fn compute_line_diff(
    original: &Code,
    orig_idx: usize,
    code: &Code,
    curr_idx: usize,
) -> LineDiff {
    let tokens_a = original.tokenize_line(orig_idx);
    let tokens_b = code.tokenize_line(curr_idx);

    let slices_a: Vec<RopeSlice<'_>> = tokens_a.iter().map(|(t, _, _)| *t).collect();
    let slices_b: Vec<RopeSlice<'_>> = tokens_b.iter().map(|(t, _, _)| *t).collect();

    let mut a_ranges = Vec::new();
    let mut b_ranges = Vec::new();

    let diff = similar::capture_diff_slices(
        similar::Algorithm::Myers, &slices_a, &slices_b
    );

    for op in diff {
        match op {
            similar::DiffOp::Equal { .. } => {}
            similar::DiffOp::Delete { old_index, old_len, .. } => {
                if old_len > 0 && old_index < tokens_a.len() {
                    let start = tokens_a[old_index].1;
                    let end = tokens_a[(old_index + old_len - 1).min(tokens_a.len() - 1)].2;
                    a_ranges.push((start, end));
                }
            }
            similar::DiffOp::Insert { new_index, new_len, .. } => {
                if new_len > 0 && new_index < tokens_b.len() {
                    let start = tokens_b[new_index].1;
                    let end = tokens_b[(new_index + new_len - 1).min(tokens_b.len() - 1)].2;
                    b_ranges.push((start, end));
                }
            }
            similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                if old_len > 0 && old_index < tokens_a.len() {
                    let start = tokens_a[old_index].1;
                    let end = tokens_a[(old_index + old_len - 1).min(tokens_a.len() - 1)].2;
                    a_ranges.push((start, end));
                }
                if new_len > 0 && new_index < tokens_b.len() {
                    let start = tokens_b[new_index].1;
                    let end = tokens_b[(new_index + new_len - 1).min(tokens_b.len() - 1)].2;
                    b_ranges.push((start, end));
                }
            }
        }
    }

    // Merge closely located ranges (max gap of 2 characters) to avoid fragmentation
    LineDiff {
        deletions: merge_ranges(a_ranges, 2),
        additions: merge_ranges(b_ranges, 2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_diff() {
        let code = Code::new("hello\nthere\nworld", "unknown", None).unwrap();
        let original = Code::new("hello\nworld", "unknown", None).unwrap();
        let rows = compute_diff(&code, &original);
        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows[0],
            VisualRow::Real {
                line_idx: 0,
                is_added: false,
                orig_line_idx: None,
            }
        );
        assert_eq!(
            rows[1],
            VisualRow::Real {
                line_idx: 1,
                is_added: true,
                orig_line_idx: None,
            }
        );
        assert_eq!(
            rows[2],
            VisualRow::Real {
                line_idx: 2,
                is_added: false,
                orig_line_idx: None,
            }
        );
    }

    #[test]
    fn test_compute_diff_replace() {
        let code = Code::new("hello\nworld2", "unknown", None).unwrap();
        let original = Code::new("hello\nworld", "unknown", None).unwrap();
        let rows = compute_diff(&code, &original);
        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows[0],
            VisualRow::Real {
                line_idx: 0,
                is_added: false,
                orig_line_idx: None,
            }
        );
        assert_eq!(
            rows[1],
            VisualRow::GhostDeleted {
                anchor_line: 2,
                original_line_idx: 1,
                curr_line_idx: Some(1),
            }
        );
        assert_eq!(
            rows[2],
            VisualRow::Real {
                line_idx: 1,
                is_added: true,
                orig_line_idx: Some(1),
            }
        );
    }

    #[test]
    fn test_tokenize_line() {
        let code = Code::new("let a = 42;", "unknown", None).unwrap();
        let tokens = code.tokenize_line(0);
        assert_eq!(tokens.len(), 8);
        assert_eq!(tokens[0], (code.line(0).slice(0..3), 0, 3));
        assert_eq!(tokens[1], (code.line(0).slice(3..4), 3, 4));
        assert_eq!(tokens[2], (code.line(0).slice(4..5), 4, 5));
        assert_eq!(tokens[3], (code.line(0).slice(5..6), 5, 6));
        assert_eq!(tokens[4], (code.line(0).slice(6..7), 6, 7));
        assert_eq!(tokens[5], (code.line(0).slice(7..8), 7, 8));
        assert_eq!(tokens[6], (code.line(0).slice(8..10), 8, 10));
        assert_eq!(tokens[7], (code.line(0).slice(10..11), 10, 11));
    }

    #[test]
    fn test_merge_ranges() {
        let ranges = vec![(0, 4), (5, 10), (15, 20)];
        assert_eq!(merge_ranges(ranges, 2), vec![(0, 10), (15, 20)]);
    }

    #[test]
    fn test_compute_line_diff() {
        let code_a = Code::new("self.width", "unknown", None).unwrap();
        let code_b = Code::new("other.height", "unknown", None).unwrap();
        let diff = compute_line_diff(&code_a, 0, &code_b, 0);
        assert_eq!(diff.deletions, vec![(0, 10)]);
        assert_eq!(diff.additions, vec![(0, 12)]);
    }
}

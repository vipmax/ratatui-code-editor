use crate::editor::{Editor};
use crate::selection::{Selection};
use crate::code::{EditKind};

pub trait Action {
    fn apply(&mut self, editor: &mut Editor);
}


/// Moves the cursor one character to the right.
///
/// If `shift` is true, the selection is extended to the new cursor position.
/// If `shift` is false and there is an active selection, the cursor jumps
/// to the end of the selection and the selection is cleared.
/// Otherwise, the cursor moves one position to the right.
pub struct MoveRight {
    pub shift: bool,
}

impl Action for MoveRight {
    fn apply(&mut self, editor: &mut Editor) {
        let cursor = editor.get_cursor();

        if !self.shift {
            if let Some(sel) = editor.get_selection() {
                if !sel.is_empty() {
                    let (_, end) = sel.sorted();
                    editor.set_cursor(end);
                    editor.clear_selection();
                    return;
                }
            }
        }

        if cursor < editor.code_mut().len() {
            let new_cursor = cursor.saturating_add(1);
            if self.shift {
                editor.extend_selection(new_cursor);
            } else {
                editor.clear_selection();
            }
            editor.set_cursor(new_cursor);
        }
    }
}

/// Moves the cursor one character to the left.
///
/// If `shift` is true, the selection is extended to the new cursor position.
/// If `shift` is false and there is an active selection, the cursor jumps
/// to the start of the selection and the selection is cleared.
/// Otherwise, the cursor moves one position to the left.
pub struct MoveLeft {
    pub shift: bool,
}

impl Action for MoveLeft {
    fn apply(&mut self, editor: &mut Editor) {
        let cursor = editor.get_cursor();

        if !self.shift {
            if let Some(sel) = editor.get_selection() {
                if !sel.is_empty() {
                    let (start, _) = sel.sorted();
                    editor.set_cursor(start);
                    editor.clear_selection();
                    return;
                }
            }
        }

        if cursor > 0 {
            let new_cursor = cursor.saturating_sub(1);
            if self.shift {
                editor.extend_selection(new_cursor);
            } else {
                editor.clear_selection();
            }
            editor.set_cursor(new_cursor);
        }
    }
}

/// Moves the cursor one line up.
///
/// If the previous line is shorter, the cursor is placed at the end of that line.
/// If `shift` is true, the selection is extended to the new cursor position.
/// If `shift` is false, the selection is cleared.
pub struct MoveUp {
    pub shift: bool,
}

impl Action for MoveUp {
    fn apply(&mut self, editor: &mut Editor) {
        let cursor = editor.get_cursor();
        let code = editor.code_mut();
        let (row, col) = code.point(cursor);

        if row == 0 { return }

        let prev_start = code.line_to_char(row - 1);
        let prev_len = code.line_len(row - 1);
        let new_col = col.min(prev_len);
        let new_cursor = prev_start + new_col;

        // Update selection or clear it
        if self.shift {
            editor.extend_selection(new_cursor);
        } else {
            editor.clear_selection();
        }

        // Set the new cursor position
        editor.set_cursor(new_cursor);
    }
}

/// Moves the cursor one line down.
/// 
/// If the next line is shorter, the cursor is placed at the end of that line.
/// If `shift` is true, the selection is extended to the new cursor position.
/// If `shift` is false, the selection is cleared.
/// 
pub struct MoveDown {
    pub shift: bool,
}

impl Action for MoveDown {
    fn apply(&mut self, editor: &mut Editor) {
        let cursor = editor.get_cursor();
        let code = editor.code_mut();
        let (row, col) = code.point(cursor);
        let is_last_line = row + 1 >= code.len_lines();
        if is_last_line { return }

        let next_start = code.line_to_char(row + 1);
        let next_len = code.line_len(row + 1);
        let new_col = col.min(next_len);
        let new_cursor = next_start + new_col;

        // Update selection or clear it
        if self.shift {
            editor.extend_selection(new_cursor);
        } else {
            editor.clear_selection();
        }

        // Set the new cursor position
        editor.set_cursor(new_cursor);
    }
}

/// Inserts arbitrary text at the cursor, replacing the selection if any.
pub struct InsertText {
    pub text: String,
}

impl Action for InsertText {
    fn apply(&mut self, editor: &mut Editor) {
        // 1. Extract current cursor and selection
        let mut cursor = editor.get_cursor();
        let mut selection = editor.get_selection();

        // 2. Work with code
        let code = editor.code_mut();
        code.tx();
        code.set_state_before(cursor, selection);

        // 3. Remove selection if present
        if let Some(sel) = &selection {
            if !sel.is_empty() {
                let (start, end) = sel.sorted();
                code.remove(start, end);
                cursor = start;
            }
        }
        selection = None;

        // 4. Insert the text at the cursor
        code.insert(cursor, &self.text);
        cursor += self.text.chars().count();

        // 5. Update editor state
        code.set_state_after(cursor, selection);
        code.commit();

        editor.set_cursor(cursor);
        editor.set_selection(selection);
        editor.reset_highlight_cache();
    }
}

/// Inserts a newline at the cursor with automatic indentation.
/// 
/// The indentation is computed based on the current line and column.
/// Delegates the actual insertion to `InsertText`.
pub struct InsertNewline;

impl Action for InsertNewline {
    fn apply(&mut self, editor: &mut Editor) {
        // 1. Get current cursor position
        let cursor = editor.get_cursor();
        let code = editor.code_mut();
        let (row, col) = code.point(cursor);

        // 2. Compute indentation for the new line
        let indent_level = code.indentation_level(row, col);
        let indent_text = code.indent().repeat(indent_level);

        // 3. Prepare the text to insert
        let text_to_insert = format!("\n{}", indent_text);

        // 4. Use InsertText action to insert the text
        let mut insert_action = InsertText { text: text_to_insert };
        insert_action.apply(editor);
    }
}

/// Deletes the selected text or the character before the cursor.
/// 
/// - If there is a non-empty selection, deletes the selection.
/// - If there is no selection, deletes the previous character.
/// - If the cursor is after indentation only, deletes the entire indentation.
pub struct Delete;

impl Action for Delete {
    fn apply(&mut self, editor: &mut Editor) {
        // 1. Extract current cursor and selection
        let mut cursor = editor.get_cursor();
        let mut selection = editor.get_selection();

        // 2. Work with code
        let code = editor.code_mut();
        code.tx();
        code.set_state_before(cursor, selection);

        if let Some(sel) = &selection && !sel.is_empty() {
            // Delete selection
            let (start, end) = sel.sorted();
            code.remove(start, end);
            cursor = start;
            selection = None;
        } else if cursor > 0 {
            // Delete single char or indentation
            let (row, col) = code.point(cursor);
            if code.is_only_indentation_before(row, col) {
                let from = cursor - col;
                code.remove(from, cursor);
                cursor = from;
            } else {
                code.remove(cursor - 1, cursor);
                cursor -= 1;
            }
        }

        // 3. Commit changes and update editor
        code.set_state_after(cursor, selection);
        code.commit();

        editor.set_cursor(cursor);
        editor.set_selection(selection);
        editor.reset_highlight_cache();
    }
}

pub struct ToggleComment;

impl Action for ToggleComment {
    /// The `ToggleComment` action toggles line comments at the start of the selected lines.
    ///
    /// If all lines in the selection already start with the language's comment string
    /// (e.g., "//" for Rust), this action removes the comment string from each affected line.
    /// Otherwise, it prepends the comment string to the beginning of each line in the selection.
    ///
    /// If there is no selection, the action is applied to the line under the cursor.
    fn apply(&mut self, editor: &mut Editor) {
        // 1. Extract current cursor and selection
        let mut cursor = editor.get_cursor();
        let mut selection = editor.get_selection();
        let selection_anchor = editor.selection_anchor();

        // 2. Work with code
        let code = editor.code_mut();

        code.tx();
        code.set_state_before(cursor, selection);

        let comment_text = code.comment();
        let comment_len = comment_text.chars().count();

        // 3. Determine lines to modify
        let lines_to_handle = if let Some(sel) = &selection && !sel.is_empty() {
            let (start, end) = sel.sorted();
            let (start_row, _) = code.point(start);
            let (end_row, _) = code.point(end);
            (start_row..=end_row).collect::<Vec<_>>()
        } else {
            let (row, _) = code.point(cursor);
            vec![row]
        };

        // 4. Check if all lines already have the comment
        let all_have_comment = lines_to_handle.iter().all(|&line_idx| {
            let line_start = code.line_to_char(line_idx);
            let line_len = code.line_len(line_idx);
            line_start + comment_len <= line_start + line_len &&
                code.slice(line_start, line_start + comment_len) == comment_text
        });

        // 5. Apply changes (add or remove comment)
        let mut comments_added = 0usize;
        let mut comments_removed = 0usize;

        for &line_idx in lines_to_handle.iter().rev() {
            let start = code.line_to_char(line_idx);
            if all_have_comment {
                // Remove comment if present at start
                let slice = code.slice(start, start + comment_len);
                if slice == comment_text {
                    code.remove(start, start + comment_len);
                    comments_removed += 1;
                }
            } else {
                // Add comment at start
                code.insert(start, &comment_text);
                comments_added += 1;
            }
        }

        // 6. Update cursor and selection
        if let Some(sel) = &selection && !sel.is_empty() {
            let (smin, _) = sel.sorted();
            let mut anchor = selection_anchor;
            let is_forward = anchor == smin;

            if is_forward {
                if !all_have_comment {
                    cursor += comment_len * comments_added;
                    anchor += comment_len;
                } else {
                    cursor = cursor.saturating_sub(comment_len * comments_removed);
                    anchor = anchor.saturating_sub(comment_len);
                }
            } else {
                if !all_have_comment {
                    cursor += comment_len;
                    anchor += comment_len * comments_added;
                } else {
                    cursor = cursor.saturating_sub(comment_len);
                    anchor = anchor.saturating_sub(comment_len * comments_removed);
                }
            }

            selection = Some(Selection::from_anchor_and_cursor(anchor, cursor));
        } else {
            if !all_have_comment {
                cursor += comment_len;
            } else {
                cursor = cursor.saturating_sub(comment_len);
            }
        }

        // 7. Commit changes
        code.set_state_after(cursor, selection);
        code.commit();

        // 8. Return changed values to the editor
        editor.set_cursor(cursor);
        editor.set_selection(selection);
        editor.reset_highlight_cache();
    }
}

/// Inserts indentation at the beginning of the current line or selected lines.
/// 
/// - If there is a selection, inserts indentation at the start of each selected line.
/// - If there is no selection, inserts indentation at the current line.
/// - Updates cursor and selection accordingly.
pub struct Indent;

impl Action for Indent {
    fn apply(&mut self, editor: &mut Editor) {
        // 1. Extract current cursor and selection
        let mut cursor = editor.get_cursor();
        let mut selection = editor.get_selection();
        let selection_anchor = editor.selection_anchor();

        // 2. Work with code
        let code = editor.code_mut();
        code.tx();
        code.set_state_before(cursor, selection);

        let indent_text = code.indent();

        // 3. Determine lines to handle
        let lines_to_handle = if let Some(sel) = &selection && !sel.is_empty() {
            let (start, end) = sel.sorted();
            let (start_row, _) = code.point(start);
            let (end_row, _) = code.point(end);
            (start_row..=end_row).collect::<Vec<_>>()
        } else {
            let (row, _) = code.point(cursor);
            vec![row]
        };

        // 4. Insert indentation for each line (reverse to not shift indices)
        let mut indents_added = 0;
        for &line_idx in lines_to_handle.iter().rev() {
            let line_start = code.line_to_char(line_idx);
            code.insert(line_start, &indent_text);
            indents_added += 1;
        }

        // 5. Update cursor and selection
        if let Some(sel) = &selection && !sel.is_empty() {
            let (smin, _) = sel.sorted();
            let mut anchor = selection_anchor;
            let is_forward = anchor == smin;

            if is_forward {
                cursor += indent_text.len() * indents_added;
                anchor += indent_text.len();
            } else {
                cursor += indent_text.len();
                anchor += indent_text.len() * indents_added;
            }

            selection = Some(Selection::from_anchor_and_cursor(anchor, cursor));
        } else {
            cursor += indent_text.len();
        }

        // 6. Commit changes
        code.set_state_after(cursor, selection);
        code.commit();

        editor.set_cursor(cursor);
        editor.set_selection(selection);
        editor.reset_highlight_cache();
    }
}


/// Removes one indentation level from the start of the current line or selected lines.
/// 
/// - If there is a selection, removes indentation from each selected line.
/// - If there is no selection, removes indentation from the current line.
/// - Updates cursor and selection accordingly.
pub struct UnIndent;

impl Action for UnIndent {
    fn apply(&mut self, editor: &mut Editor) {
        // 1. Extract current cursor and selection
        let mut cursor = editor.get_cursor();
        let mut selection = editor.get_selection();
        let selection_anchor = editor.selection_anchor();

        // 2. Work with code
        let code = editor.code_mut();
        code.tx();
        code.set_state_before(cursor, selection);

        let indent_text = code.indent();
        let indent_len = indent_text.chars().count();

        // 3. Determine lines to handle
        let lines_to_handle = if let Some(sel) = &selection && !sel.is_empty() {
            let (start, end) = sel.sorted();
            let (start_row, _) = code.point(start);
            let (end_row, _) = code.point(end);
            (start_row..=end_row).collect::<Vec<_>>()
        } else {
            let (row, _) = code.point(cursor);
            vec![row]
        };

        // 4. Remove indentation from each line
        let mut lines_untabbed = 0;
        for &line_idx in lines_to_handle.iter().rev() {
            if let Some(indent_cols) = code.find_indent_at_line_start(line_idx) {
                let remove_count = indent_cols.min(indent_len);
                if remove_count > 0 {
                    let line_start = code.line_to_char(line_idx);
                    code.remove(line_start, line_start + remove_count);
                    lines_untabbed += 1;
                }
            }
        }

        // 5. Update cursor and selection
        if let Some(sel) = &selection && !sel.is_empty() {
            let (smin, _) = sel.sorted();
            let mut anchor = selection_anchor;
            let is_forward = anchor == smin;

            if is_forward {
                cursor = cursor.saturating_sub(indent_len * lines_untabbed);
                anchor = anchor.saturating_sub(indent_len);
            } else {
                cursor = cursor.saturating_sub(indent_len);
                anchor = anchor.saturating_sub(indent_len * lines_untabbed);
            }

            selection = Some(Selection::from_anchor_and_cursor(anchor, cursor));
        } else {
            cursor = cursor.saturating_sub(indent_len * lines_untabbed);
        }

        // 6. Commit changes
        code.set_state_after(cursor, selection);
        code.commit();

        editor.set_cursor(cursor);
        editor.set_selection(selection);
        editor.reset_highlight_cache();
    }
}

/// Selects the entire text in the editor.
pub struct SelectAll;

impl Action for SelectAll {
    fn apply(&mut self, editor: &mut Editor) {
        // Set selection from start to end of the document
        let from = 0;
        let code = editor.code_mut();
        let to = code.len_chars();
        let sel = Selection::new(from, to);
        editor.set_selection(Some(sel));
    }
}

/// Duplicates the selected text or the current line if no selection exists.
///
/// If there is a selection, it duplicates the selected text immediately after it.
/// If there is no selection, it duplicates the entire line under the cursor,
/// preserving the cursor's relative column position.
pub struct Duplicate;

impl Action for Duplicate {
    fn apply(&mut self, editor: &mut Editor) {
        // 1. Extract current cursor and selection
        let mut cursor = editor.get_cursor();
        let mut selection = editor.get_selection();
        let code = editor.code_mut();

        code.tx();
        code.set_state_before(cursor, selection);

        if let Some(sel) = &selection {
            // Duplicate selected text
            let text = code.slice(sel.start, sel.end);
            let insert_pos = sel.end;
            code.insert(insert_pos, &text);
            cursor = insert_pos + text.chars().count();
            selection = None;
        } else {
            // Duplicate the current line
            let (line_start, line_end) = code.line_boundaries(cursor);
            let line_text = code.slice(line_start, line_end);
            let column = cursor - line_start;

            let insert_pos = line_end;
            let to_insert = if line_text.ends_with('\n') {
                line_text.clone()
            } else {
                format!("{}\n", line_text)
            };
            code.insert(insert_pos, &to_insert);

            // Keep cursor on the same relative column in the new line
            let new_line_len = to_insert.trim_end_matches('\n').chars().count();
            let new_column = column.min(new_line_len);
            cursor = insert_pos + new_column;
        }

        code.set_state_after(cursor, selection);
        code.commit();

        // Update editor state
        editor.set_cursor(cursor);
        editor.set_selection(selection);
        editor.reset_highlight_cache();
    }
}

/// Deletes the entire line under the cursor.
pub struct DeleteLine;

impl Action for DeleteLine {
    fn apply(&mut self, editor: &mut Editor) {
        // 1. Extract current cursor and selection
        let mut cursor = editor.get_cursor();
        let mut selection = editor.get_selection();
        let code = editor.code_mut();

        // 2. Compute line boundaries
        let (start, end) = code.line_boundaries(cursor);

        // Do nothing if the line is empty and at the end of file
        if start == end && start == code.len() {
            return;
        }

        // 3. Remove the line
        code.tx();
        code.set_state_before(cursor, selection);
        code.remove(start, end);
        code.set_state_after(start, None);
        code.commit();

        // 4. Update editor state
        cursor = start;
        selection = None;
        editor.set_cursor(cursor);
        editor.set_selection(selection);
        editor.reset_highlight_cache();
    }
}

/// Cuts the current selection: copies it to the clipboard and removes it from the editor.
pub struct Cut;

impl Action for Cut {
    fn apply(&mut self, editor: &mut Editor) {
        // 1. Extract current cursor and selection
        let mut cursor = editor.get_cursor();
        let mut selection = editor.get_selection();

        let sel = match &selection {
            Some(sel) if !sel.is_empty() => sel.clone(),
            _ => return, // nothing to cut
        };

        // 2. Copy to clipboard first, before borrowing code mutably
        let text = editor.code_ref().slice(sel.start, sel.end);
        let _ = editor.set_clipboard(&text);

        // 3. Now borrow code mutably
        let code = editor.code_mut();
        code.tx();
        code.set_state_before(cursor, selection);
        code.remove(sel.start, sel.end);
        code.set_state_after(sel.start, None);
        code.commit();

        // 4. Update editor state
        cursor = sel.start;
        selection = None;
        editor.set_cursor(cursor);
        editor.set_selection(selection);
        editor.reset_highlight_cache();
    }
}

/// Copies the selected text to the clipboard.
/// 
/// Does nothing if there is no active selection.
pub struct Copy;

impl Action for Copy {
    fn apply(&mut self, editor: &mut Editor) {
        // 1. Extract current selection
        let selection = editor.get_selection();

        // 2. Return early if no selection
        let Some(sel) = selection else { return };
        if sel.is_empty() { return }

        // 3. Get text and copy to clipboard
        let text = editor.code_ref().slice(sel.start, sel.end);
        let _ = editor.set_clipboard(&text);
    }
}

/// Pastes text from the clipboard at the current cursor position.
/// 
/// If a selection exists, it will be replaced by the pasted text.
/// The pasted text is adjusted using language-specific indentation rules.
pub struct Paste;

impl Action for Paste {
    fn apply(&mut self, editor: &mut Editor) {
        // 1. Get clipboard contents
        let Ok(text) = editor.get_clipboard() else { return };
        if text.is_empty() { return }

        // 2. Extract current cursor and selection
        let mut cursor = editor.get_cursor();
        let mut selection = editor.get_selection();
        let code = editor.code_mut();

        // 3. Prepare transaction
        code.tx();
        code.set_state_before(cursor, selection);

        // 4. Remove selection if present
        if let Some(sel) = &selection {
            if !sel.is_empty() {
                let (start, end) = sel.sorted();
                code.remove(start, end);
                cursor = start;
                selection = None;
            }
        }

        // 5. Perform paste with smart indentation
        let inserted = code.smart_paste(cursor, &text);
        cursor += inserted;

        // 6. Finalize transaction
        code.set_state_after(cursor, selection);
        code.commit();

        // 7. Update editor state
        editor.set_cursor(cursor);
        editor.set_selection(selection);
        editor.reset_highlight_cache();
    }
}

/// Undoes the last edit in the code buffer.
/// 
/// Restores both the cursor position and selection state
/// from the saved editor snapshot if available.
pub struct Undo;

impl Action for Undo {
    fn apply(&mut self, editor: &mut Editor) {
        // 1. Get mutable access to code
        let code = editor.code_mut();

        // 2. Try to undo
        let edits = code.undo();
        editor.reset_highlight_cache();

        // 3. If nothing to undo, return
        let Some(batch) = edits else { return };

        // 4. Restore cursor and selection from saved state if possible
        if let Some(before) = batch.state_before {
            editor.set_cursor(before.offset);
            editor.set_selection(before.selection);
            return;
        }

        // 5. Otherwise infer cursor position from edits
        for edit in batch.edits.iter().rev() {
            match &edit.kind {
                EditKind::Insert { offset, .. } => {
                    editor.set_cursor(*offset);
                }
                EditKind::Remove { offset, text } => {
                    editor.set_cursor(*offset + text.chars().count());
                }
            }
        }
    }
}

/// Redoes the last undone edit in the code buffer.
/// 
/// Restores both the cursor position and selection state
/// from the saved editor snapshot if available.
pub struct Redo;

impl Action for Redo {
    fn apply(&mut self, editor: &mut Editor) {
        // 1. Get mutable access to code
        let code = editor.code_mut();

        // 2. Try to redo
        let edits = code.redo();
        editor.reset_highlight_cache();

        // 3. If nothing to redo, return
        let Some(batch) = edits else { return };

        // 4. Restore cursor and selection from saved state if possible
        if let Some(after) = batch.state_after {
            editor.set_cursor(after.offset);
            editor.set_selection(after.selection);
            return;
        }

        // 5. Otherwise infer cursor position from edits
        for edit in batch.edits {
            match &edit.kind {
                EditKind::Insert { offset, text } => {
                    editor.set_cursor(*offset + text.chars().count());
                }
                EditKind::Remove { offset, .. } => {
                    editor.set_cursor(*offset);
                }
            }
        }
    }
}
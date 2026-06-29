use ratatui_code_editor::editor::Editor;
use ratatui_core::layout::Rect;

fn focused_diff_editor() -> Editor {
    let mut editor = Editor::new("text", "0\n1\n2\nchanged\n4\n5\n", vec![]).unwrap();
    editor.set_diff_enabled(true);
    editor.set_original_code("0\n1\n2\n3\n4\n5\n").unwrap();
    editor.set_diff_focus_context(0);
    editor.set_diff_focus_enabled(true);
    editor
}

#[test]
fn focus_moves_cursor_to_nearest_changed_line() {
    let mut editor = focused_diff_editor();
    editor.set_cursor(2);

    editor.focus(&Rect::new(0, 0, 80, 10));

    assert_eq!(editor.get_cursor(), 6);
}

#[test]
fn focus_handles_a_zero_height_viewport() {
    let mut editor = Editor::new("text", "first\nsecond", vec![]).unwrap();
    editor.set_cursor(6);

    editor.focus(&Rect::new(0, 0, 80, 0));
}

#[test]
fn clicking_fold_control_expands_hidden_diff() {
    let mut editor = focused_diff_editor();
    let area = Rect::new(0, 0, 80, 10);

    assert!(editor.expand_hidden_diff_at_mouse(14, 0, &area));
    // column 9 = line_number_digits(5) + left_code_padding(2) + fold_gutter_width(2)
    assert_eq!(editor.cursor_from_mouse(9, 0, &area), Some(0));
}

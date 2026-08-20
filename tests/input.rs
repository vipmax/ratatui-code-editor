use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_code_editor::editor::Editor;
use ratatui_core::layout::Rect;

#[test]
fn ctrl_u_unindents_current_line() {
    let mut editor = Editor::new("rust", "    let value = 1;\n", vec![]).unwrap();
    let area = Rect::new(0, 0, 80, 10);

    editor
        .input(
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
            &area,
        )
        .unwrap();

    assert_eq!(editor.get_content(), "let value = 1;\n");
}

#[test]
fn ctrl_f_is_plain_editor_input() {
    let mut editor = Editor::new("rust", "", vec![]).unwrap();
    let area = Rect::new(0, 0, 80, 10);

    editor
        .input(
            KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
            &area,
        )
        .unwrap();

    assert_eq!(editor.get_content(), "f");
}

#[test]
fn toggle_fold_at_cursor_toggles_cursor_line_fold() {
    let source = "fn main() {\n    let value = 1;\n}\nafter();\n";
    let mut editor = Editor::new("rust", source, vec![]).unwrap();
    let area = Rect::new(0, 0, 80, 10);

    editor.set_cursor(source.find("fn main").unwrap());
    assert!(editor.toggle_fold_at_cursor());

    assert!(editor.get_visible_cursor(&area).is_some());
    editor.set_cursor(source.find("value").unwrap());
    assert!(editor.get_visible_cursor(&area).is_none());
}

#[test]
fn focus_scrolls_horizontally_for_wide_glyph_lines() {
    let line: String = std::iter::repeat('界').take(20).collect();
    let mut editor = Editor::new("text", &line, vec![]).unwrap();
    let area = Rect::new(0, 0, 40, 10);

    editor.set_cursor(line.chars().count());
    editor.focus(&area);

    assert!(editor.get_offset_x() > 0);
    assert!(editor.get_visible_cursor(&area).is_some());
}

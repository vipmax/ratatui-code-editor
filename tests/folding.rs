use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui_code_editor::code::Code;
use ratatui_code_editor::{
    actions::{InsertText, MoveRight},
    editor::Editor,
    selection::Selection,
    types::{CodeFoldingOptions, FoldIndicators},
};
use ratatui_core::layout::Rect;

#[test]
fn rust_reports_and_toggles_a_multiline_fold() {
    let source = "fn main() {\n    let value = 1;\n}\n";
    let code = Code::new(source, "rust", None).unwrap();
    assert!(
        code.fold_ranges()
            .iter()
            .any(|range| range.start_line == 0 && range.end_line == 2)
    );

    let mut editor = Editor::new("rust", source, vec![]).unwrap();
    editor.set_cursor(source.find("value").unwrap());
    assert!(editor.toggle_fold_at_line(0));
    assert_eq!(editor.code_ref().point(editor.get_cursor()).0, 1);
    assert!(!editor.toggle_fold_at_line(9));
    assert!(editor.toggle_fold_at_line(0));

    editor.set_code_folding_options(CodeFoldingOptions {
        enabled: false,
        indicators: FoldIndicators::ascii(),
    });
    assert!(!editor.is_code_folding_enabled());
    assert_eq!(
        editor.code_folding_options().indicators,
        FoldIndicators::ascii()
    );
}

#[test]
fn folded_ranges_are_pruned_after_content_changes() {
    let source = "fn main() {\n    let value = 1;\n}\n";
    let mut editor = Editor::new("rust", source, vec![]).unwrap();

    assert!(editor.toggle_fold_at_line(0));

    let updated = "const A: i32 = 1;\nconst B: i32 = 2;\nconst C: i32 = 3;\n";
    editor.set_content(updated);
    editor.set_cursor(updated.find("C").unwrap());

    assert!(
        editor
            .get_visible_cursor(&Rect::new(0, 0, 80, 10))
            .is_some()
    );
}

#[test]
fn cursor_is_pulled_out_when_selection_endpoint_is_inside_fold() {
    let source = "fn main() {\n    let value = 1;\n}\n";
    let mut editor = Editor::new("rust", source, vec![]).unwrap();
    let value = source.find("value").unwrap();

    editor.set_cursor(value);
    editor.set_selection(Some(Selection::from_anchor_and_cursor(0, value)));
    assert!(editor.toggle_fold_at_line(0));
    assert_eq!(editor.code_ref().point(editor.get_cursor()).0, 1);

    editor.apply(MoveRight { shift: false });

    assert_eq!(editor.code_ref().point(editor.get_cursor()).0, 0);
    assert!(
        editor
            .get_visible_cursor(&Rect::new(0, 0, 80, 10))
            .is_some()
    );
}

#[test]
fn move_right_skips_hidden_fold_body() {
    let source = "fn main() {\n    let value = 1;\n}\nafter();\n";
    let mut editor = Editor::new("rust", source, vec![]).unwrap();
    let value = source.find("value").unwrap();

    editor.set_cursor(value);
    assert!(editor.toggle_fold_at_line(0));
    assert_eq!(editor.code_ref().point(editor.get_cursor()).0, 1);

    editor.apply(MoveRight { shift: false });

    assert_eq!(editor.code_ref().point(editor.get_cursor()).0, 3);
    assert!(
        editor
            .get_visible_cursor(&Rect::new(0, 0, 80, 10))
            .is_some()
    );
}

#[test]
fn typing_inside_hidden_fold_keeps_cursor_inside_until_navigation() {
    let source = "fn main() {\n    let value = 1;\n}\nafter();\n";
    let mut editor = Editor::new("rust", source, vec![]).unwrap();
    let value = source.find("value").unwrap();

    editor.set_cursor(value);
    assert!(editor.toggle_fold_at_line(0));

    editor.apply(InsertText {
        text: "new_".into(),
    });

    assert!(editor.get_content().contains("new_value"));
    assert_eq!(editor.code_ref().point(editor.get_cursor()).0, 1);

    editor.apply(MoveRight { shift: false });

    assert_eq!(editor.code_ref().point(editor.get_cursor()).0, 3);
}

#[test]
fn terminal_input_inside_hidden_fold_does_not_focus_cursor_out() {
    let source = "fn main() {\n    let value = 1;\n}\nafter();\n";
    let mut editor = Editor::new("rust", source, vec![]).unwrap();
    let value = source.find("value").unwrap();
    let area = Rect::new(0, 0, 80, 10);

    editor.set_cursor(value);
    assert!(editor.toggle_fold_at_line(0));
    assert_eq!(editor.get_offset_y(), 0);

    editor
        .input(
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty()),
            &area,
        )
        .unwrap();

    assert_eq!(editor.code_ref().point(editor.get_cursor()).0, 1);
    assert_eq!(editor.get_offset_y(), 0);
}

#[test]
fn hidden_line_numbers_keep_fold_gutter_clickable() {
    let source = "fn main() {\n    let value = 1;\n}\nafter();\n";
    let mut editor = Editor::new("rust", source, vec![]).unwrap();
    let value = source.find("value").unwrap();
    let area = Rect::new(0, 0, 80, 10);

    editor.show_line_numbers(false);
    editor.set_cursor(value);
    editor
        .mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 2,
                row: 0,
                modifiers: KeyModifiers::empty(),
            },
            &area,
        )
        .unwrap();

    assert!(editor.get_visible_cursor(&area).is_none());
}

use ratatui_code_editor::editor::Editor;
use ratatui_core::style::Color;

#[test]
fn test_editor_get_line_diff() {
    let mut editor = Editor::new("rust", "fn main() {\n    let a = 10;\n}", vec![]).unwrap();
    editor.set_original_code("fn main() {\n    let b = 10;\n}").unwrap();

    let add_highlights = editor.get_line_diff(1, 1, false);
    assert_eq!(add_highlights, vec![(8, 9)]);

    let del_highlights = editor.get_line_diff(1, 1, true);
    assert_eq!(del_highlights, vec![(8, 9)]);
}

#[test]
fn test_editor_word_highlight() {
    let mut editor = Editor::new("rust", "let abc = 123;\nlet abc_def = abc;\nlet abc = 456;", vec![]).unwrap();
    // Cursor is at index 4 ('a' of the first 'abc')
    editor.set_cursor(4);
    let ranges = editor.word_highlight_ranges();
    // Expected matches at characters:
    // Line 0: "let abc = 123;\n" (15 chars) -> "abc" at 4..7
    // Line 1: "let abc_def = abc;\n" (19 chars) -> "abc" at 14..17 of line 1 (15 + 14 = 29..32)
    // Line 2: "let abc = 456;" -> "abc" at 4..7 of line 2 (15 + 19 = 34 + 4 = 38..41)
    assert_eq!(ranges, vec![(4, 7), (29, 32), (38, 41)]);
}

#[test]
fn test_build_theme_bg_fg() {
    let theme = vec![
        ("diff_added", "#017d4e"),
        ("identifier", "#A5FCB6"),
    ];
    let built = Editor::build_theme(&theme);
    
    let diff_added_style = built.get("diff_added").unwrap();
    assert_eq!(diff_added_style.bg, Some(Color::Rgb(1, 125, 78)));
    assert_eq!(diff_added_style.fg, None);
    
    let identifier_style = built.get("identifier").unwrap();
    assert_eq!(identifier_style.fg, Some(Color::Rgb(165, 252, 182)));
    assert_eq!(identifier_style.bg, None);
}

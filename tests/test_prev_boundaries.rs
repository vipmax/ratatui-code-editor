use ratatui_code_editor::code::Code;

#[test]
fn test_prev_boundaries() {
    let s = "नमस्ते दुनिया!";
    let mut code = Code::new(s, "text", None).unwrap();
    let chars = code.len_chars();
    let mut cursor = chars;
    println!("Char len: {}", chars);
    while cursor > 0 {
        let prev = code.prev_grapheme_boundary(cursor);
        let text = code.char_slice(prev, cursor);
        println!(
            "cursor {} -> prev {} (text: {:?})",
            cursor,
            prev,
            text.to_string()
        );
        cursor = prev;
    }
}

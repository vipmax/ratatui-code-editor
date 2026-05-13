use ratatui_code_editor::code::Code;

#[test]
fn test_boundaries() {
    let s = "नमस्ते दुनिया!";
    let mut code = Code::new(s, "text", None).unwrap();
    let mut cursor = 0;
    let len = code.len_chars();
    println!("Char len: {}", len);
    while cursor < len {
        let next = code.next_grapheme_boundary(cursor);
        let text = code.char_slice(cursor, next);
        println!(
            "cursor {} -> next {} (text: {:?})",
            cursor,
            next,
            text.to_string()
        );
        cursor = next;
    }
}

use ratatui_code_editor::code::Code;

#[test]
fn test_prev_with_next() {
    let s = "नमस्ते दुनिया!";
    let mut code = Code::new(s, "text", None).unwrap();
    let chars = code.len_chars();

    let mut cursor = chars;
    while cursor > 0 {
        // compute prev using next from line start
        let line = code.char_to_line(cursor);
        let mut line_start = code.line_to_char(line);

        let mut prev = line_start;
        let mut cur = line_start;

        if cursor == line_start && line > 0 {
            line_start = code.line_to_char(line - 1);
            prev = line_start;
            cur = line_start;
        }

        while cur < cursor {
            prev = cur;
            cur = code.next_grapheme_boundary(cur);
            if cur >= cursor {
                break;
            }
        }

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

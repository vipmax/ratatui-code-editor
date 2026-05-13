use ratatui_code_editor::code::{Code, RopeGraphemes, grapheme_width_and_chars_len};

fn char_col_to_visual(code: &Code, line_idx: usize, char_col: usize) -> usize {
    let line_start = code.line_to_char(line_idx);
    let line_len = code.line_len(line_idx);
    let limit = char_col.min(line_len);
    let slice = code.char_slice(line_start, line_start + limit);
    RopeGraphemes::new(&slice)
        .map(|g| grapheme_width_and_chars_len(g).0)
        .sum()
}

fn visual_to_char_col(code: &Code, line_idx: usize, visual_col: usize) -> usize {
    let line_start = code.line_to_char(line_idx);
    let line_len = code.line_len(line_idx);
    let slice = code.char_slice(line_start, line_start + line_len);

    let mut current_visual = 0;
    let mut char_col = 0;
    for g in RopeGraphemes::new(&slice) {
        let (g_width, g_chars) = grapheme_width_and_chars_len(g);
        if current_visual + g_width > visual_col {
            break;
        }
        current_visual += g_width;
        char_col += g_chars;
    }
    char_col
}

#[test]
fn test_coord() {
    let s = "नमस्ते दुनिया!";
    let code = Code::new(s, "text", None).unwrap();
    for i in 0..=code.len_chars() {
        let vis = char_col_to_visual(&code, 0, i);
        let back = visual_to_char_col(&code, 0, vis);
        println!("char {} -> vis {} -> back char {}", i, vis, back);
    }
}

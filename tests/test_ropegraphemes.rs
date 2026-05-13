use ratatui_code_editor::code::{Code, RopeGraphemes, grapheme_width_and_chars_len};

#[test]
fn test_ropegrapheme() {
    let s = "नमस्ते दुनिया!";
    let code = Code::new(s, "text", None).unwrap();
    let text = code.char_slice(0, code.len_chars());
    let mut total_width = 0;
    println!("Total chars = {}", code.len_chars());
    for g in RopeGraphemes::new(&text) {
        let (width, chars) = grapheme_width_and_chars_len(g);
        println!("{:?} - width: {}, chars: {}", g.to_string(), width, chars);
        total_width += width;
    }
    println!("Total visual width: {}", total_width);
}

use unicode_segmentation::UnicodeSegmentation;

#[test]
fn test_grapheme_cursor_bug2() {
    let s = "नमस्ते दुनिया!";
    let mut chars = s.char_indices();
    for _ in 0..6 {
        chars.next();
    }
    let byte_idx = chars.next().unwrap().0; // byte index of " "

    println!("String slice: {:?}", &s[..byte_idx]);
    let graphemes: Vec<&str> = (&s[..byte_idx]).graphemes(true).collect();
    println!("Graphemes in slice: {:?}", graphemes);
}

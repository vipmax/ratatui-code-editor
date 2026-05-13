use unicode_segmentation::GraphemeCursor;

#[test]
fn test_grapheme_cursor_bug() {
    let s = "नमस्ते दुनिया!";
    let mut chars = s.char_indices();
    for _ in 0..6 {
        chars.next();
    }
    let byte_idx = chars.next().unwrap().0; // byte index of " "

    let mut cursor2 = GraphemeCursor::new(byte_idx, s.len(), true);
    let mut prev = byte_idx;
    loop {
        match cursor2.prev_boundary(s, 0) {
            Ok(Some(p)) => {
                println!("prev step: {}", p);
                if p == 0 {
                    break;
                }
                prev = p;
            }
            Ok(None) => break,
            Err(e) => {
                println!("err2: {:?}", e);
                break;
            }
        }
    }
}

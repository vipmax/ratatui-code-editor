use unicode_segmentation::GraphemeCursor;

#[test]
fn test_grapheme_cursor_bug3() {
    let s = "नमस्ते दुनिया!";
    println!("String: {:?}", s);

    let mut cursor = GraphemeCursor::new(18, s.len(), true);
    match cursor.prev_boundary(s, 0) {
        Ok(Some(p)) => println!("Cursor from 18 prev: {}", p),
        _ => (),
    }

    let mut cursor2 = GraphemeCursor::new(18, 18, true);
    match cursor2.prev_boundary(&s[..18], 0) {
        Ok(Some(p)) => println!("Cursor from 18 (len 18) prev: {}", p),
        _ => (),
    }
}

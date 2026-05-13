use unicode_segmentation::UnicodeSegmentation;

#[test]
fn test_grapheme_cursor_bug4() {
    let s = "नमस्ते दुनिया!";
    println!("String: {:?}", s);

    let mut iter = (&s[..18]).graphemes(true);
    let mut backward = Vec::new();
    while let Some(g) = iter.next_back() {
        backward.push(g);
    }
    println!("Backward: {:?}", backward);

    let forward: Vec<_> = (&s[..18]).graphemes(true).collect();
    println!("Forward: {:?}", forward);
}

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[test]
fn test_hindi() {
    let s = "नमस्ते दुनिया!";
    println!("String: {}", s);
    for g in s.graphemes(true) {
        println!(
            "grapheme: {:?} (len chars: {}, width: {})",
            g,
            g.chars().count(),
            UnicodeWidthStr::width(g)
        );
    }
}

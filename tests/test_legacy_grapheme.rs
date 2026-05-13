use unicode_segmentation::UnicodeSegmentation;

#[test]
fn test_legacy() {
    let s = "नमस्ते दुनिया!";
    println!("Extended:");
    for g in s.graphemes(true) {
        println!("{:?}", g);
    }
    println!("Legacy:");
    for g in s.graphemes(false) {
        println!("{:?}", g);
    }
}

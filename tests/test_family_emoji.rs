use unicode_width::UnicodeWidthStr;

#[test]
fn test_family_emoji() {
    let s = "👨‍👩‍👧‍👦";
    println!("Family width: {}", UnicodeWidthStr::width(s));

    // Also test ninja
    let s2 = "🥷🏿";
    println!("Ninja width: {}", UnicodeWidthStr::width(s2));

    // Also handshake
    let s3 = "🤝🏽";
    println!("Handshake width: {}", UnicodeWidthStr::width(s3));
}

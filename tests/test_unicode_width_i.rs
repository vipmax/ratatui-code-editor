use unicode_width::UnicodeWidthStr;

#[test]
fn test_unicode_width_i() {
    println!("न width: {}", UnicodeWidthStr::width("न"));
    println!("ि width: {}", UnicodeWidthStr::width("ि"));
    println!("नि width: {}", UnicodeWidthStr::width("नि"));
    println!("स width: {}", UnicodeWidthStr::width("स"));
    println!("् width: {}", UnicodeWidthStr::width("्"));
    println!("त width: {}", UnicodeWidthStr::width("त"));
    println!("े width: {}", UnicodeWidthStr::width("े"));
}

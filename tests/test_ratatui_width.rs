use ratatui::text::Line;

#[test]
fn test_ratatui_width() {
    println!("Ratatui line width: {}", Line::from("नमस्ते दुनिया!").width());
}

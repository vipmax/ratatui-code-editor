use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

#[test]
fn test_ratatui_buffer() {
    let s = "नमस्ते दुनिया!";
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
    buf.set_string(0, 0, s, Style::default());
    for x in 0..15 {
        let cell = buf.cell((x, 0)).unwrap().clone();
        println!(
            "x: {}, symbol: {:?} (skip: {})",
            x,
            cell.symbol(),
            cell.skip
        );
    }
}

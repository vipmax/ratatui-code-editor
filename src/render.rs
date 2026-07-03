use crate::code::{RopeGraphemes, grapheme_width_and_bytes_len, grapheme_width_and_chars_len};
use crate::editor::Editor;
use crate::types::VisualRow;
use crate::view::View;
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Color, Style};
use ratatui_core::widgets::Widget;

/// Draws the main editor view in the provided area using the ratatui rendering buffer.
///
/// Renders visible [`VisualRow`]s, including fold separators and deleted diff rows.
/// Added and deleted rows receive a diff background before syntax highlighting is
/// applied. Selections and user marks are then drawn over real editor rows.
///
/// # Arguments
///
/// * `self` - The `Editor` instance (as reference) to render.
/// * `area` - The rectangular area on the terminal to draw within.
/// * `buf` - The ratatui `Buffer` that represents the screen cells to draw to.
///
impl Widget for &Editor {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let code = self.code_ref();
        let total_lines = code.len_lines();
        let max_line_number = total_lines.max(1);
        let line_number_digits = max_line_number.to_string().len().max(5);
        let line_number_width = self.get_line_number_width();
        let fold_gutter_width = self.fold_gutter_width();
        let total_visual_lines = self.visual_len_lines();
        let mut draw_y = area.top();

        let line_number_style = Style::default().fg(Color::DarkGray);
        let default_text_style = Style::default().fg(Color::White);

        let diff_added_bg = self.theme_style("diff_added").fg.unwrap_or(Color::Rgb(24, 60, 37));
        let diff_added_word_bg = self.theme_style("diff_added_word").fg.unwrap_or(Color::Rgb(38, 91, 53));
        let diff_deleted_bg = self.theme_style("diff_deleted").fg.unwrap_or(Color::Rgb(76, 35, 35));
        let diff_deleted_word_bg = self.theme_style("diff_deleted_word").fg.unwrap_or(Color::Rgb(118, 53, 53));

        let fold_separator_style = Style::default().fg(Color::DarkGray);

        // draw lines, syntax highlighting, selection and marks in a single unified loop
        for visual_row_idx in self.offset_y..total_visual_lines {
            if draw_y >= area.bottom() {
                break;
            }

            let row = match self.visual_row(visual_row_idx) {
                Some(row) => row,
                None => break,
            };

            if let VisualRow::FoldSeparator { hidden_lines, .. } = &row {
                if self.show_line_numbers {
                    buf.set_string(
                        area.left(),
                        draw_y,
                        &format!("{:>width$}", "...", width = line_number_digits),
                        line_number_style,
                    );
                }
                let text_x = area.left() + line_number_width as u16;
                let text =
                    View::fold_separator_text(*hidden_lines, self.diff_options.expand_amount);
                let width = (area.width as usize).saturating_sub(line_number_width);
                let visible_text = text.chars().take(width).collect::<String>();
                if text_x < area.right() {
                    buf.set_string(text_x, draw_y, &visible_text, fold_separator_style);
                }
            } else {
                let (line_idx, is_added, is_ghost, partner_line_idx) = match &row {
                    VisualRow::Real { line_idx, is_added, orig_line_idx } => (*line_idx, *is_added, false, *orig_line_idx),
                    VisualRow::GhostDeleted {
                        original_line_idx, curr_line_idx, ..
                    } => (*original_line_idx, false, true, *curr_line_idx),
                    _ => unreachable!(),
                };
                let source_code = if is_ghost {
                    self.original_code.as_ref().unwrap_or(code)
                } else {
                    code
                };

                // 1. Draw line numbers
                if self.show_line_numbers {
                    let line_number = if is_ghost {
                        format!("{:>width$}", " ", width = line_number_digits)
                    } else {
                        format!("{:>width$}", line_idx + 1, width = line_number_digits)
                    };
                    buf.set_string(area.left(), draw_y, &line_number, line_number_style);
                }
                if !is_ghost {
                    if let Some(collapsed) = self.code_fold_indicator(line_idx) {
                        let indicator = if collapsed {
                            &self.code_folding_options.indicators.collapsed
                        } else {
                            &self.code_folding_options.indicators.expanded
                        };
                        buf.set_string(
                            area.left() + (line_number_width - fold_gutter_width) as u16,
                            draw_y,
                            indicator,
                            line_number_style,
                        );
                    }
                }

                let text_x = area.left() + line_number_width as u16;
                let width = (area.width as usize).saturating_sub(line_number_width);

                let line_len = source_code.line_len(line_idx);
                let start_col = self.offset_x.min(line_len);
                let end_col = (start_col + width).min(line_len);

                let line_start_char = source_code.line_to_char(line_idx);
                let char_slice_start = line_start_char + start_col;
                let char_slice_end = line_start_char + end_col;
                let visible_chars = source_code.char_slice(char_slice_start, char_slice_end);

                let start_byte = source_code.char_to_byte(char_slice_start);
                let end_byte = source_code.char_to_byte(char_slice_end);

                // Fetch highlights
                let highlights = if code.is_highlight() {
                    if is_ghost {
                        self.highlight_interval_original(start_byte, end_byte, &self.theme)
                    } else {
                        self.highlight_interval(start_byte, end_byte, &self.theme)
                    }
                } else {
                    Vec::new()
                };

                // Fetch intra-line diff highlights on the fly from cache
                let intra_highlights = partner_line_idx.map(|partner_idx| {
                    if is_ghost {
                        self.get_line_diff(line_idx, partner_idx, true)
                    } else {
                        self.get_line_diff(partner_idx, line_idx, false)
                    }
                });

                // Base style background color
                let base_bg = match is_ghost {
                    true => Some(diff_deleted_bg),
                    false if is_added => Some(diff_added_bg),
                    false => None,
                };

                let mut x = 0;
                let mut byte_idx_in_rope = start_byte;
                let mut char_col = start_col;

                // 3. Single loop over the graphemes of the line
                for g in RopeGraphemes::new(&visible_chars) {
                    let (g_width, g_bytes) = grapheme_width_and_bytes_len(g);
                    let (_, g_chars) = grapheme_width_and_chars_len(g);

                    if x >= width {
                        break;
                    }

                    let start_x = text_x + x as u16;

                    // Check if current character falls within an intra-line highlight range
                    let is_word_highlight = intra_highlights.as_ref().map_or(false, |ranges| {
                        ranges.iter().any(|&(start, end)| char_col >= start && char_col < end)
                    });

                    let active_bg = if is_word_highlight {
                        if is_ghost { Some(diff_deleted_word_bg) } else { Some(diff_added_word_bg) }
                    } else {
                        base_bg
                    };

                    // Compose style
                    let mut style = if let Some(bg) = active_bg {
                        Style::default().bg(bg)
                    } else {
                        default_text_style
                    };

                    // Layer A: Syntax highlights
                    for &(start, end, s) in &highlights {
                        if start <= byte_idx_in_rope && byte_idx_in_rope < end {
                            style = style.patch(s);
                            if let Some(bg) = active_bg {
                                style = style.bg(bg); // Keep active diff background
                            }
                            break;
                        }
                    }

                    let global_char_idx = line_start_char + char_col;

                    if !is_ghost {
                        // Layer B: Selection
                        if let Some(selection) = self.selection
                            && !selection.is_empty()
                        {
                            let start = selection.start.min(selection.end);
                            let end = selection.start.max(selection.end);
                            if global_char_idx >= start && global_char_idx < end {
                                style = style.bg(Color::DarkGray);
                            }
                        }

                        // Layer C: Marks
                        if let Some(ref marks) = self.marks {
                            for &(m_start, m_end, m_color) in marks {
                                if global_char_idx >= m_start && global_char_idx < m_end {
                                    style = style.bg(m_color);
                                }
                            }
                        }
                    }

                    // Draw character
                    let display_g = g.to_string().replace('\t', " ");
                    if start_x < area.right() {
                        buf.set_string(start_x, draw_y, &display_g, style);
                    }

                    x = x.saturating_add(g_width);
                    byte_idx_in_rope += g_bytes;
                    char_col += g_chars;
                }

                // 4. Fill remaining width with background if needed
                if let Some(bg) = base_bg
                    && x < width
                    && text_x + (x as u16) < area.right()
                {
                    let fill_x = text_x + (x as u16);
                    let fill_width = width - x;
                    buf.set_string(
                        fill_x,
                        draw_y,
                        &" ".repeat(fill_width),
                        Style::default().bg(bg),
                    );
                }
            }
            draw_y += 1;
        }
    }
}

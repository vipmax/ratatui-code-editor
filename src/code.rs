use anyhow::{Result, anyhow};
use ropey::{Rope, RopeSlice};
use streaming_iterator::StreamingIterator;
use tree_sitter::{InputEdit, Point, QueryCursor};
use tree_sitter::{Language, Parser, Query, Tree};
use crate::history::{History, EditBatch, Edit, EditKind};
use rust_embed::RustEmbed;
use std::collections::HashMap;
use crate::utils::indent;

#[derive(RustEmbed)]
#[folder = ""]
#[include = "langs/*/*"]
struct LangAssets;

pub struct Code {
    content: ropey::Rope,
    lang: String,
    tree: Option<Tree>,
    parser: Option<Parser>,
    query: Option<Query>,
    applying_history: bool,
    history: History,
    current_batch: EditBatch,
}

impl Code {
    /// Create a new `Code` instance with the given text and language.
    pub fn new(text: &str, lang: &str) -> Result<Self> {
        let (tree, parser, query) = match Self::get_language(lang) {
            Some(language) => {
                let highlights = Self::get_highlights(lang)?;
                let mut parser = Parser::new();
                parser.set_language(&language)?;
                let tree = parser.parse(text, None);
                let query = Query::new(&language, &highlights)?;
                (tree, Some(parser), Some(query))
            }
            None => (None, None, None),
        };
        
        Ok(Self {
            content: Rope::from_str(text),
            lang: lang.to_string(),
            tree, parser, query,
            applying_history: true,
            history: History::new(1000),
            current_batch: Vec::new(),
        })
    }
    
    fn get_language(lang: &str) -> Option<Language> {
        match lang {
            "rust" => Some(tree_sitter_rust::LANGUAGE.into()),
            "javascript" => Some(tree_sitter_javascript::LANGUAGE.into()),
            "typescript" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            "python" => Some(tree_sitter_python::LANGUAGE.into()),
            "go" => Some(tree_sitter_go::LANGUAGE.into()),
            "java" => Some(tree_sitter_java::LANGUAGE.into()),
            "c_sharp" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
            "c" => Some(tree_sitter_c::LANGUAGE.into()),
            "cpp" => Some(tree_sitter_cpp::LANGUAGE.into()),
            "html" => Some(tree_sitter_html::LANGUAGE.into()),
            "css" => Some(tree_sitter_css::LANGUAGE.into()),
            "yaml" => Some(tree_sitter_yaml::LANGUAGE.into()),
            "json" => Some(tree_sitter_json::LANGUAGE.into()),
            "toml" => Some(tree_sitter_toml_ng::LANGUAGE.into()),
            "shell" => Some(tree_sitter_bash::LANGUAGE.into()),
            _ => None,
        }
    }
    
    fn get_highlights(lang: &str) -> anyhow::Result<String> {
        let p = format!("langs/{}/highlights.scm", lang);
        let highlights_bytes =
            LangAssets::get(&p).ok_or_else(|| anyhow!("No highlights found for {}", lang))?;
        let highlights_bytes = highlights_bytes.data.as_ref();
        let highlights = std::str::from_utf8(highlights_bytes)?;
        Ok(highlights.to_string())
    }

    pub fn point(&self, offset: usize) -> (usize, usize) {
        let row = self.content.char_to_line(offset);
        let line_start = self.content.line_to_char(row);
        let col = offset - line_start;
        (row, col)
    }

    pub fn offset(&self, row: usize, col: usize) -> usize {
        let line_start = self.content.line_to_char(row);
        line_start + col
    }
    
    pub fn get_content(&self) -> String {
        self.content.to_string()
    }
    
    pub fn slice(&self, start: usize, end: usize) -> String {
        self.content.slice(start..end).to_string()
    }

    pub fn len(&self) -> usize {
        self.content.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.content.len_lines()
    }

    pub fn len_chars(&self) -> usize {
        self.content.len_chars()
    }

    pub fn line_to_char(&self, line_idx: usize) -> usize {
        self.content.line_to_char(line_idx)
    }
    pub fn char_to_byte(&self, char_idx: usize) -> usize {
        self.content.char_to_byte(char_idx)
    }

    pub fn line_len(&self, line_idx: usize) -> usize {
        self.content.line(line_idx).len_chars()
    }
    
    pub fn line(&self, line_idx: usize) -> RopeSlice{
        self.content.line(line_idx)
    }
    
    pub fn char_to_line(&self, char_idx: usize) -> usize {
        self.content.char_to_line(char_idx)
    }
    
    pub fn char_slice(&self, start: usize, end: usize) -> RopeSlice {
        self.content.slice(start..end)
    }
    
    pub fn byte_slice(&self, start: usize, end: usize) -> RopeSlice {
        self.content.byte_slice(start..end)
    }
    
    pub fn byte_to_line(&self, byte_idx: usize) -> usize {
        self.content.byte_to_line(byte_idx)
    }
    
    pub fn byte_to_char(&self, byte_idx: usize) -> usize {
        self.content.byte_to_char(byte_idx)
    }
    
    pub fn begin_batch(&mut self) {
        self.current_batch.clear();
    }

    pub fn commit_batch(&mut self) {
        if !self.current_batch.is_empty() {
            self.history.push(self.current_batch.clone());
            self.current_batch.clear();
        }
    }
    
    pub fn insert(&mut self, from: usize, text: &str) {
        let byte_idx = self.content.char_to_byte(from);
        let byte_len: usize = text.chars().map(|ch| ch.len_utf8()).sum();
        
        self.content.insert(from, text);
        
        if self.applying_history {
            self.current_batch.push(Edit {
                kind: EditKind::Insert {
                    offset: from,
                    text: text.to_string(),
                },
            });
        }
        
        if self.tree.is_some() {
            self.edit_tree(InputEdit {
                start_byte: byte_idx,
                old_end_byte: byte_idx,
                new_end_byte: byte_idx + byte_len,
                start_position: Point { row: 0, column: 0 },
                old_end_position: Point { row: 0, column: 0 },
                new_end_position: Point { row: 0, column: 0 },
            });
        }
    }

    pub fn remove(&mut self, from: usize, to: usize) {
        let from_byte = self.content.char_to_byte(from);
        let to_byte = self.content.char_to_byte(to);
        let removed_text = self.content.slice(from..to).to_string();
        
        self.content.remove(from..to);
        
        if self.applying_history {
            self.current_batch.push(Edit {
                kind: EditKind::Remove {
                    offset: from,
                    text: removed_text,
                },
            });
        }
        
        if self.tree.is_some() {
            self.edit_tree(InputEdit {
                start_byte: from_byte,
                old_end_byte: to_byte,
                new_end_byte: from_byte,
                start_position: Point { row: 0, column: 0 },
                old_end_position: Point { row: 0, column: 0 },
                new_end_position: Point { row: 0, column: 0 },
            });
        }
       
    }

    fn edit_tree(&mut self, edit: InputEdit) {
        if let Some(tree) = self.tree.as_mut() {
            tree.edit(&edit);
            self.reparse();
        }
    }

    fn reparse(&mut self) {
        if let Some(parser) = self.parser.as_mut() {
            let rope = &self.content;
            self.tree = parser.parse_with_options(
                &mut |byte, _| {
                    if byte <= rope.len_bytes() {
                        let (chunk, start, _, _) = rope.chunk_at_byte(byte);
                        &chunk.as_bytes()[byte - start..]
                    } else {
                        &[]
                    }
                },
                self.tree.as_ref(),
                None,
            );
        }
    }

    pub fn is_highlight(&self) -> bool {
        self.query.is_some()
    }
    
    /// Highlights the interval between `start` and `end` char indices.
    /// Returns a list of (start byte, end byte, token_name) for highlighting.
    pub fn highlight_interval<T: Copy>(
        &self, start: usize, end: usize, theme: &HashMap<String, T>,
    ) -> Vec<(usize, usize, T)> {
        if start > start { panic!("invalid range")}
        let Some(query) = &self.query else { return vec![]; };
        let Some(tree) = &self.tree else { return vec![]; };
    
        let mut query_cursor = QueryCursor::new();
        query_cursor.set_byte_range(start..end);
    
        let root_node = tree.root_node();
        let capture_names = query.capture_names();
        
        let mut query_matches = query_cursor.matches(
            query, root_node, RopeProvider(self.content.slice(..))
        );
    
        let mut unsorted: Vec<(usize, usize, usize, T)> = Vec::new();
    
        while let Some(m) = query_matches.next() {
            for capture in m.captures {
                let name = capture_names[capture.index as usize];
                // let node_text = self.content
                //     .byte_slice(capture.node.start_byte()..capture.node.end_byte()).as_str()
                //     .unwrap_or_default(); // debug

                if let Some(value) = theme.get(name) {
                    unsorted.push((
                        capture.node.start_byte(),
                        capture.node.end_byte(),
                        capture.index as usize,
                        *value,
                    ));
                }
            }
        }
    
        // Sort by length descending, then by capture index
        unsorted.sort_by(|a, b| {
            let len_a = a.1 - a.0;
            let len_b = b.1 - b.0;
            match len_b.cmp(&len_a) {
                std::cmp::Ordering::Equal => b.2.cmp(&a.2),
                other => other,
            }
        });
    
        unsorted.into_iter()
            .map(|(start, end, _, value)| (start, end, value))
            .collect()
    }
    
    pub fn undo(&mut self) -> Option<EditBatch> {
        let batch = self.history.undo()?;
        self.applying_history = false;
    
        for edit in batch.iter().rev() {
            match edit.kind {
                EditKind::Insert { offset, ref text } => {
                    self.remove(offset, offset + text.chars().count());
                }
                EditKind::Remove { offset, ref text } => {
                    self.insert(offset, text);
                }
            }
        }
    
        self.applying_history = true;
        Some(batch)
    }
    
    pub fn redo(&mut self) -> Option<EditBatch> {
        let batch = self.history.redo()?;
        self.applying_history = false;
    
        for edit in &batch {
            match edit.kind {
                EditKind::Insert { offset, ref text } => {
                    self.insert(offset, text);
                }
                EditKind::Remove { offset, ref text } => {
                    self.remove(offset, offset + text.chars().count());
                }
            }
        }
    
        self.applying_history = true;
        Some(batch)
    }
    
    pub fn word_boundaries(&self, pos: usize) -> (usize, usize) {
        let len = self.content.len_chars();
        if pos >= len {
            return (pos, pos);
        }
    
        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
    
        let mut start = pos;
        while start > 0 {
            let c = self.content.char(start - 1);
            if !is_word_char(c) {
                break;
            }
            start -= 1;
        }
    
        let mut end = pos;
        while end < len {
            let c = self.content.char(end);
            if !is_word_char(c) {
                break;
            }
            end += 1;
        }
    
        (start, end)
    }

    pub fn line_boundaries(&self, pos: usize) -> (usize, usize) {
        let total_chars = self.content.len_chars();
        if pos >= total_chars {
            return (pos, pos);
        }

        let line = self.content.char_to_line(pos);
        let start = self.content.line_to_char(line);
        let end = start + self.content.line(line).len_chars();

        (start, end)
    }
    
    pub fn indent(&self) -> String {
        indent(&self.lang)
    }

    pub fn indentation_level(&self, line: usize, col: usize) -> usize {
        if self.lang == "unknown" || self.lang == "" { return 0 }
        let indent_str = self.indent();
        let line_slice = self.line(line);
        let line_str = line_slice.to_string();
        let mut count = 0;
        let mut chars = line_str.chars().peekable();
        let mut total_chars = 0;

        while chars.peek().is_some() {
            let mut matched = true;
            let mut indent_chars = 0;
            for ch in indent_str.chars() {
                if Some(&ch) != chars.peek() {
                    matched = false;
                    break;
                }
                chars.next();
                indent_chars += 1;
            }
            total_chars += indent_chars;
            if total_chars > col {
                break;
            }
            if matched {
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    pub fn is_only_indentation_before(&self, r: usize, c: usize) -> bool {
        if self.lang == "unknown" || self.lang == "" { return false }
        if r >= self.len_lines() || c == 0 { return false; }

        let line = self.line(r);

        let mut col = 0;
        for ch in line.chars() {
            if col >= c { break; } // Reached the specified column
            // Found a non-whitespace character before the specified position
            if !ch.is_whitespace() { return false; }
            col += 1;
        }
        true
    }

}

/// An iterator over byte slices of Rope chunks.
/// This is used to feed `tree-sitter` without allocating a full `String`.
pub struct ChunksBytes<'a> {
    chunks: ropey::iter::Chunks<'a>,
}

impl<'a> Iterator for ChunksBytes<'a> {
    type Item = &'a [u8];

    /// Returns the next chunk as a byte slice.
    /// Internally converts a `&str` to a `&[u8]` without allocation.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.chunks.next().map(str::as_bytes)
    }
}

/// A lightweight wrapper around a `RopeSlice`
/// that implements `tree_sitter::TextProvider`.
/// This allows using `tree-sitter`'s `QueryCursor::matches`
/// directly on a `Rope` without converting it to a `String`.
pub struct RopeProvider<'a>(pub RopeSlice<'a>);

impl<'a> tree_sitter::TextProvider<&'a [u8]> for RopeProvider<'a> {
    type I = ChunksBytes<'a>;

    /// Provides an iterator over chunks of text corresponding to the given node.
    /// This avoids allocation by working directly with Rope slices.
    #[inline]
    fn text(&mut self, node: tree_sitter::Node) -> Self::I {
        let fragment = self.0.byte_slice(node.start_byte()..node.end_byte());
        ChunksBytes {
            chunks: fragment.chunks(),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert() {
        let mut code = Code::new("", "").unwrap();
        code.insert(0, "Hello ");
        code.insert(6, "World");
        assert_eq!(code.content.to_string(), "Hello World");
    }
    
    #[test]
    fn test_remove() {
        let mut code = Code::new("Hello World", "").unwrap();
        code.remove(5, 11);
        assert_eq!(code.content.to_string(), "Hello");
    }
    
    #[test]
    fn test_undo() {
        let mut code = Code::new("", "").unwrap();
        
        code.begin_batch();
        code.insert(0, "Hello ");
        code.commit_batch();
        
        code.begin_batch();
        code.insert(6, "World");
        code.commit_batch();
        
        code.undo();
        assert_eq!(code.content.to_string(), "Hello ");
        
        code.undo();
        assert_eq!(code.content.to_string(), "");
    }
    
    #[test]
    fn test_redo() {
        let mut code = Code::new("", "").unwrap();
        
        code.begin_batch();
        code.insert(0, "Hello");
        code.commit_batch();
        
        code.undo();
        assert_eq!(code.content.to_string(), "");
        
        code.redo();
        assert_eq!(code.content.to_string(), "Hello");
    }

    #[test]
    fn test_highlight() {
        let ch_width = unicode_width::UnicodeWidthChar::width('\t');
        println!("ch_width: {:?}", ch_width);
        // assert_eq!(ch_width, 1);
    }

    #[test]
    fn test_indentation_level0() {
        let mut code = Code::new("", "unknown").unwrap();
        code.insert(0, "    hello world");
        assert_eq!(code.indentation_level(0, 10), 0);
    }

    #[test]
    fn test_indentation_level() {
        let mut code = Code::new("", "python").unwrap();
        code.insert(0, "    print('Hello, World!')");
        assert_eq!(code.indentation_level(0, 10), 1);
    }

    #[test]
    fn test_indentation_level2() {
        let mut code = Code::new("", "python").unwrap();
        code.insert(0, "        print('Hello, World!')");
        assert_eq!(code.indentation_level(0, 10), 2);
    }

    #[test]
    fn test_is_only_indentation_before() {
        let mut code = Code::new("", "python").unwrap();
        code.insert(0, "    print('Hello, World!')");
        assert_eq!(code.is_only_indentation_before(0, 4), true);
        assert_eq!(code.is_only_indentation_before(0, 10), false);
    }

    #[test]
    fn test_is_only_indentation_before2() {
        let mut code = Code::new("", "").unwrap();
        code.insert(0, "    Hello, World");
        assert_eq!(code.is_only_indentation_before(0, 4), false);
        assert_eq!(code.is_only_indentation_before(0, 10), false);
    }
}

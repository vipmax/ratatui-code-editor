use anyhow::{Result, anyhow};
use ropey::Rope;
use streaming_iterator::StreamingIterator;
use tree_sitter::{InputEdit, Point, QueryCursor};
use tree_sitter::{Language, Parser, Query, Tree};
use crate::history::{History, EditBatch, Edit, EditKind};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = ""]
#[include = "langs/*/*"]
struct LangAssets;

fn get_language(lang: &str) -> Option<Language> {
    match lang {
        "rust" => Some(tree_sitter_rust::LANGUAGE.into()),
        "javascript" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "typescript" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
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

pub struct Code {
    pub content: ropey::Rope,
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
        let (tree, parser, query) = match get_language(lang) {
            Some(language) => {
                let highlights = get_highlights(lang)?;
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
            tree, parser, query,
            applying_history: true,
            history: History::new(1000),
            current_batch: Vec::new(),
        })
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
        
        self.edit_tree(InputEdit {
            start_byte: byte_idx,
            old_end_byte: byte_idx,
            new_end_byte: byte_idx + byte_len,
            start_position: Point { row: 0, column: 0 },
            old_end_position: Point { row: 0, column: 0 },
            new_end_position: Point { row: 0, column: 0 },
        });
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
        
        self.edit_tree(InputEdit {
            start_byte: from_byte,
            old_end_byte: to_byte,
            new_end_byte: from_byte,
            start_position: Point { row: 0, column: 0 },
            old_end_position: Point { row: 0, column: 0 },
            new_end_position: Point { row: 0, column: 0 },
        });
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

    pub fn is_highlightable(&self) -> bool {
        self.query.is_some()
    }

pub fn query_matches(
    &self, start_line: usize, end_line: usize,  allowed: &Vec<&String>,
) -> Vec<(usize, usize, String)> {
    let Some(query) = &self.query else {
        return vec![];
    };
    let Some(tree) = &self.tree else {
        return vec![];
    };

    let mut query_cursor = QueryCursor::new();
    let start_byte = self.content.line_to_byte(start_line);
    let end_byte = self
        .content
        .line_to_byte(end_line.min(self.content.len_lines()));

    query_cursor.set_byte_range(start_byte..end_byte);

    let source = self.content.to_string();
    let root_node = tree.root_node();

    let mut unsorted: Vec<(usize, usize, usize, String)> = Vec::new(); // include index for sorting
    let mut query_matches = query_cursor.matches(query, root_node, source.as_bytes());

    while let Some(m) = query_matches.next() {
        for capture in m.captures {
            let name = &query.capture_names()[capture.index as usize];

            if !allowed.iter().any(|a| name.contains(*a)) {
                continue;
            }

            unsorted.push((
                capture.node.start_byte(),
                capture.node.end_byte(),
                capture.index as usize, // only for sorting
                name.to_string(),
            ));
        }
    }

    // Sort by length descending, then capture.index ascending
    unsorted.sort_by(|a, b| {
        let len_a = a.1 - a.0;
        let len_b = b.1 - b.0;
        len_b.cmp(&len_a).then(a.2.cmp(&b.2))
    });

    // Drop capture index from result
    unsorted
        .into_iter()
        .map(|(start, end, _index, name)| (start, end, name))
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

    pub fn slice(&self, start: usize, end: usize) -> String {
        self.content.slice(start..end).to_string()
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
}

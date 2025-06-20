use anyhow::{Result, anyhow};
use ropey::Rope;
use streaming_iterator::StreamingIterator;
use tree_sitter::{InputEdit, Point, QueryCursor};
use tree_sitter::{Language, Parser, Query, Tree};

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
    pub content: Rope,
    tree: Option<Tree>,
    parser: Option<Parser>,
    query: Option<Query>,
}

impl Code {
    pub fn new(text: &str, lang: &str) -> Result<Self> {
        let maybe_language = get_language(lang);

        if let Some(language) = maybe_language {
            let highlights = get_highlights(lang)?;
            let mut parser = Parser::new();
            parser.set_language(&language)?;
            let tree = parser.parse(text, None);
            let query = Query::new(&language, &highlights)?;

            Ok(Self {
                content: Rope::from_str(text),
                tree,
                parser: Some(parser),
                query: Some(query),
            })
        } else {
            Ok(Self {
                content: Rope::from_str(text),
                tree: None,
                parser: None,
                query: None,
            })
        }
    }

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        let byte_idx = self.content.char_to_byte(char_idx);
        self.content.insert(char_idx, text);
        let byte_len: usize = text.chars().map(|ch| ch.len_utf8()).sum();
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
        self.content.remove(from..to);
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

    pub fn is_highlighted(&self) -> bool {
        self.query.is_some()
    }

    pub fn query_matches(
        &self,
        start_line: usize,
        end_line: usize,
        allowed: &Vec<&String>,
    ) -> Vec<(usize, usize, Point, Point, String)> {
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

        let mut result = vec![];
        let source = self.content.to_string();
        let root_node = tree.root_node();

        let mut query_matches = query_cursor.matches(
            query, root_node, source.as_bytes()
        );

        while let Some(m) = query_matches.next() {
            for capture in m.captures {
                // let node_text = self.content.byte_slice(
                //     capture.node.start_byte()..capture.node.end_byte()
                // ).as_str().unwrap_or_default(); // for debug only

                let name = &query.capture_names()[capture.index as usize];

                if !allowed.iter().any(|a| a == name) {
                    continue;
                }

                let r = (
                    capture.node.start_byte(),
                    capture.node.end_byte(),
                    capture.node.start_position(),
                    capture.node.end_position(),
                    name.to_string(),
                );

                result.push(r);
            }
        }

        // sort by length of range in descending order (longest first)
        result.sort_by(|a, b| {
            let len_a = a.1 - a.0;
            let len_b = b.1 - b.0;
            len_b.cmp(&len_a)
        });

        result
    }
}

use anyhow::{Result, anyhow};
use ropey::{Rope, RopeSlice};
use streaming_iterator::StreamingIterator;
use tree_sitter::{InputEdit, Point, QueryCursor};
use tree_sitter::{Language, Parser, Query, Tree, Node};
use crate::history::{History};
use crate::selection::Selection;
use rust_embed::RustEmbed;
use std::collections::HashMap;
use crate::utils::{indent, count_indent_units, comment as lang_comment, calculate_end_position};
use std::cell::RefCell;
use std::rc::Rc;
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};
use unicode_width::{UnicodeWidthStr};

#[derive(RustEmbed)]
#[folder = ""]
#[include = "langs/*/*"]
struct LangAssets;


#[derive(Clone)]
pub enum EditKind {
    Insert { offset: usize, text: String },
    Remove { offset: usize, text: String },
}

#[derive(Clone)]
pub struct Edit {
    pub kind: EditKind,
}

#[derive(Clone)]
pub struct EditBatch {
    pub edits: Vec<Edit>,
    pub state_before: Option<EditState>,
    pub state_after: Option<EditState>,
}

impl EditBatch {
    pub fn new() -> Self {
        Self { 
            edits: Vec::new(), 
            state_before: None,
            state_after: None,
        }
    }

}

#[derive(Clone, Copy)]
pub struct EditState {
    pub offset: usize,
    pub selection: Option<Selection>,
}


pub struct Code {
    content: ropey::Rope,
    lang: String,
    tree: Option<Tree>,
    parser: Option<Parser>,
    query: Option<Query>,
    applying_history: bool,
    history: History,
    current_batch: EditBatch,
    injection_parsers: Option<HashMap<String, Rc<RefCell<Parser>>>>,
    injection_queries: Option<HashMap<String, Query>>,
    change_callback: Option<Box<dyn Fn(Vec<(usize, usize, usize, usize, String)>)>>,
    custom_highlights: Option<HashMap<String, String>>,
}

impl Code {
    /// Create a new `Code` instance with the given text and language.
    pub fn new(
        text: &str,
        lang: &str,
        custom_highlights: Option<HashMap<String, String>>,
    ) -> Result<Self> {
        let mut code = Self {
            content: Rope::from_str(text),
            lang: lang.to_string(),
            tree: None,
            parser: None,
            query: None,
            applying_history: true,
            history: History::new(1000),
            current_batch: EditBatch::new(),
            injection_parsers: None,
            injection_queries: None,
            change_callback: None,
            custom_highlights,
        };

        if let Some(language) = Self::get_language(lang) {
            let highlights = code.get_highlights(lang)?;
            let mut parser = Parser::new();
            parser.set_language(&language)?;
            let tree = parser.parse(text, None);
            let query = Query::new(&language, &highlights)?;
            let (iparsers, iqueries) = code.init_injections(&query)?;
            code.tree = tree;
            code.parser = Some(parser);
            code.query = Some(query);
            code.injection_parsers = Some(iparsers);
            code.injection_queries = Some(iqueries);
        }

        Ok(code)
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
            "markdown" => Some(tree_sitter_md::LANGUAGE.into()),
            "markdown-inline" => Some(tree_sitter_md::INLINE_LANGUAGE.into()),
            _ => None,
        }
    }

    fn get_highlights(&self, lang: &str) -> anyhow::Result<String> {
        if let Some(highlights_map) = &self.custom_highlights {
            if let Some(highlights) = highlights_map.get(lang) {
                return Ok(highlights.clone());
            }
        }
        let p = format!("langs/{}/highlights.scm", lang);
        let highlights_bytes =
            LangAssets::get(&p).ok_or_else(|| anyhow!("No highlights found for {}", lang))?;
        let highlights_bytes = highlights_bytes.data.as_ref();
        let highlights = std::str::from_utf8(highlights_bytes)?;
        Ok(highlights.to_string())
    }

    fn init_injections(
        &self,
        query: &Query,
    ) -> anyhow::Result<(
        HashMap<String, Rc<RefCell<Parser>>>,
        HashMap<String, Query>,
    )> {
        let mut injection_parsers = HashMap::new();
        let mut injection_queries = HashMap::new();

        for name in query.capture_names() {
            if let Some(lang) = name.strip_prefix("injection.content.") {
                if injection_parsers.contains_key(lang) {
                    continue;
                }
                if let Some(language) = Self::get_language(lang) {
                    let mut parser = Parser::new();
                    parser.set_language(&language)?;
                    let highlights = self.get_highlights(lang)?;
                    let inj_query = Query::new(&language, &highlights)?;

                    injection_parsers.insert(lang.to_string(), Rc::new(RefCell::new(parser)));
                    injection_queries.insert(lang.to_string(), inj_query);
                } else {
                    eprintln!("Unknown injection language: {}", lang);
                }
            }
        }

        Ok((injection_parsers, injection_queries))
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

    pub fn line_len(&self, idx: usize) -> usize {
        let line = self.content.line(idx);
        let len = line.len_chars();
        if idx == self.content.len_lines() - 1 {
            len
        } else {
            len.saturating_sub(1)
        }
    }
    
    pub fn line(&self, line_idx: usize) -> RopeSlice<'_> {
        self.content.line(line_idx)
    }

    pub fn char_to_line(&self, char_idx: usize) -> usize {
        self.content.char_to_line(char_idx)
    }
    
    pub fn char_slice(&self, start: usize, end: usize) -> RopeSlice<'_> {
        self.content.slice(start..end)
    }
    
    pub fn byte_slice(&self, start: usize, end: usize) -> RopeSlice<'_> {
        self.content.byte_slice(start..end)
    }
    
    pub fn byte_to_line(&self, byte_idx: usize) -> usize {
        self.content.byte_to_line(byte_idx)
    }
    
    pub fn byte_to_char(&self, byte_idx: usize) -> usize {
        self.content.byte_to_char(byte_idx)
    }
    
    pub fn tx(&mut self) {
        self.current_batch = EditBatch::new();
    }

    pub fn set_state_before(&mut self, offset: usize, selection: Option<Selection>) {
        self.current_batch.state_before = Some(EditState { offset, selection });
    }

    pub fn set_state_after(&mut self, offset: usize, selection: Option<Selection>) {
        self.current_batch.state_after = Some(EditState { offset, selection });
    }

    pub fn commit(&mut self) {
        if !self.current_batch.edits.is_empty() {
            self.notify_changes(&self.current_batch.edits);
            self.history.push(self.current_batch.clone());
            self.current_batch = EditBatch::new();
        }
    }
    
    pub fn insert(&mut self, from: usize, text: &str) {
        let byte_idx = self.content.char_to_byte(from);
        let byte_len: usize = text.chars().map(|ch| ch.len_utf8()).sum();
        
        self.content.insert(from, text);
        
        if self.applying_history {
            self.current_batch.edits.push(Edit {
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
            self.current_batch.edits.push(Edit {
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
        if start > end { panic!("Invalid range") }

        let Some(query) = &self.query else { return vec![]; };
        let Some(tree) = &self.tree else { return vec![]; };

        let text = self.content.slice(..);
        let root_node = tree.root_node();

        let mut results = Self::highlight(
            text,
            start,
            end,
            query,
            root_node,
            theme,
            self.injection_parsers.as_ref(),
            self.injection_queries.as_ref(),
        );

        results.sort_by(|a, b| {
            let len_a = a.1 - a.0;
            let len_b = b.1 - b.0;
            match len_b.cmp(&len_a) {
                std::cmp::Ordering::Equal => b.2.cmp(&a.2),
                other => other,
            }
        });

        results
            .into_iter()
            .map(|(start, end, _, value)| (start, end, value))
            .collect()
    }

    fn highlight<T: Copy>(
        text: RopeSlice<'_>,
        start_byte: usize,
        end_byte: usize,
        query: &Query,
        root_node: Node,
        theme: &HashMap<String, T>,
        injection_parsers: Option<&HashMap<String, Rc<RefCell<Parser>>>>,
        injection_queries: Option<&HashMap<String, Query>>,
    ) -> Vec<(usize, usize, usize, T)> {
        let mut cursor = QueryCursor::new();
        cursor.set_byte_range(start_byte..end_byte);

        let mut matches = cursor.matches(query, root_node, RopeProvider(text));

        let mut results = Vec::new();
        let capture_names = query.capture_names();

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let name = capture_names[capture.index as usize];
                if let Some(value) = theme.get(name) {
                    results.push((
                        capture.node.start_byte(),
                        capture.node.end_byte(),
                        capture.index as usize,
                        *value,
                    ));
                } else if let Some(lang) = name.strip_prefix("injection.content.") {
                    let Some(injection_parsers) = injection_parsers else { continue };
                    let Some(injection_queries) = injection_queries else { continue };
                    let Some(parser) = injection_parsers.get(lang) else { continue };
                    let Some(injection_query) = injection_queries.get(lang) else { continue };

                    let start = capture.node.start_byte();
                    let end = capture.node.end_byte();
                    let slice = text.byte_slice(start..end);

                    let mut parser = parser.borrow_mut();
                    let Some(inj_tree) = parser.parse(slice.to_string(), None) else { continue };

                    let injection_results = Self::highlight(
                        slice,
                        0,
                        end - start,
                        injection_query,
                        inj_tree.root_node(),
                        theme,
                        injection_parsers.into(),
                        injection_queries.into(),
                    );

                    for (s, e, i, v) in injection_results {
                        results.push((s + start, e + start, i, v));
                    }
                }
            }
        }

        results
    }


    pub fn undo(&mut self) -> Option<EditBatch> {
        let batch = self.history.undo()?;
        self.applying_history = false;
    
        for edit in batch.edits.iter().rev() {
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
    
        for edit in &batch.edits {
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

    pub fn comment(&self) -> String {
        lang_comment(&self.lang).to_string()
    }

    pub fn indentation_level(&self, line: usize, col: usize) -> usize {
        if self.lang == "unknown" || self.lang.is_empty() { return 0; }
        let line_str = self.line(line);
        count_indent_units(line_str, &self.indent(), Some(col))
    }

    pub fn is_only_indentation_before(&self, r: usize, c: usize) -> bool {
        if self.lang == "unknown" || self.lang.is_empty() { return false; }
        if r >= self.len_lines() || c == 0 { return false; }
    
        let line = self.line(r);
        let indent_unit = self.indent();
    
        if indent_unit.is_empty() {
            return line.chars().take(c).all(|ch| ch.is_whitespace());
        }
    
        let count_units = count_indent_units(line, &indent_unit, Some(c));
        let only_indent = count_units * indent_unit.chars().count() >= c;
        only_indent
    }

    pub fn find_indent_at_line_start(&self, line_idx: usize) -> Option<usize> {
        if line_idx >= self.len_lines() { return None; }
    
        let line = self.line(line_idx);
        let indent_unit = self.indent();
        if indent_unit.is_empty() { return None; }
    
        let count_units = count_indent_units(line, &indent_unit, None);
        let col = count_units * indent_unit.chars().count();
        if col > 0 { Some(col) } else { None }
    }

    /// Paste text with **indentation awareness**.
    /// 
    /// 1. Determine the indentation level at the cursor (`base_level`).
    /// 2. The first line of the pasted block is inserted at the cursor level (trimmed).
    /// 3. Subsequent lines adjust their indentation **relative to the previous non-empty line in the pasted block**:
    ///    - Compute `diff` = change in indentation from the previous non-empty line in the source block (clamped ±1).
    ///    - Apply `diff` to `prev_nonempty_level` to calculate the new insertion level.
    /// 4. Empty lines are inserted as-is and do not affect subsequent indentation.
    /// 
    /// This ensures that pasted blocks keep their relative structure while aligning to the cursor.


    /// Inserts `text` with indentation-awareness at `offset`.
    /// Returns number of characters inserted.
    pub fn smart_paste(&mut self, offset: usize, text: &str) -> usize {
        let (row, col) = self.point(offset);
        let base_level = self.indentation_level(row, col);
        let indent_unit = self.indent();

        if indent_unit.is_empty() {
            self.insert(offset, text);
            return text.chars().count();
        }

        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return 0;
        }

        // Compute indentation levels of all lines in the source block
        let mut line_levels = Vec::with_capacity(lines.len());
        for line in &lines {
            let mut lvl = 0;
            let mut rest = *line;
            while rest.starts_with(&indent_unit) {
                lvl += 1;
                rest = &rest[indent_unit.len()..];
            }
            line_levels.push(lvl);
        }

        let mut result = Vec::with_capacity(lines.len());

        let first_line_trimmed = lines[0].trim_start();
        result.push(first_line_trimmed.to_string());

        let mut prev_nonempty_level = base_level;
        let mut prev_line_level_in_block = line_levels[0];

        for i in 1..lines.len() {
            let line = lines[i];

            if line.trim().is_empty() {
                result.push(line.to_string());
                continue;
            }

            // diff relative to previous non-empty line in the source block
            let diff = (line_levels[i] as isize - prev_line_level_in_block as isize).clamp(-1, 1);
            let new_level = (prev_nonempty_level as isize + diff).max(0) as usize;
            let indents = indent_unit.repeat(new_level);
            let result_line = format!("{}{}", indents, line.trim_start());
            result.push(result_line);

            // update levels only for non-empty line
            prev_nonempty_level = new_level;
            prev_line_level_in_block = line_levels[i];
        }

        let to_insert = result.join("\n");
        self.insert(offset, &to_insert);
        to_insert.chars().count()
    }

    /// Set the change callback function for handling document changes
    pub fn set_change_callback(&mut self, callback: Box<dyn Fn(Vec<(usize, usize, usize, usize, String)>)>) {
        self.change_callback = Some(callback);
    }

    /// Notify about document changes
    fn notify_changes(&self, edits: &[Edit]) {
        if let Some(callback) = &self.change_callback {
            let mut changes = Vec::new();
            
            for edit in edits {
                match &edit.kind {
                    EditKind::Insert { offset, text } => {
                        let (start_row, start_col) = self.point(*offset);
                        changes.push((start_row, start_col, start_row, start_col, text.clone()));
                    }
                    EditKind::Remove { offset, text } => {
                        let (start_row, start_col) = self.point(*offset);
                        let (end_row, end_col) = calculate_end_position(start_row, start_col, text);
                        changes.push((start_row, start_col, end_row, end_col, String::new()));
                    }
                }
            }
            
            if !changes.is_empty() {
                callback(changes);
            }
        }
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

/// An implementation of a graphemes iterator, for iterating over the graphemes of a RopeSlice.
pub struct RopeGraphemes<'a> {
    text: ropey::RopeSlice<'a>,
    chunks: ropey::iter::Chunks<'a>,
    cur_chunk: &'a str,
    cur_chunk_start: usize,
    cursor: GraphemeCursor,
}

impl<'a> RopeGraphemes<'a> {
    pub fn new<'b>(slice: &RopeSlice<'b>) -> RopeGraphemes<'b> {
        let mut chunks = slice.chunks();
        let first_chunk = chunks.next().unwrap_or("");
        RopeGraphemes {
            text: *slice,
            chunks: chunks,
            cur_chunk: first_chunk,
            cur_chunk_start: 0,
            cursor: GraphemeCursor::new(0, slice.len_bytes(), true),
        }
    }
}

impl<'a> Iterator for RopeGraphemes<'a> {
    type Item = RopeSlice<'a>;

    fn next(&mut self) -> Option<RopeSlice<'a>> {
        let a = self.cursor.cur_cursor();
        let b;
        loop {
            match self
                .cursor
                .next_boundary(self.cur_chunk, self.cur_chunk_start)
            {
                Ok(None) => {
                    return None;
                }
                Ok(Some(n)) => {
                    b = n;
                    break;
                }
                Err(GraphemeIncomplete::NextChunk) => {
                    self.cur_chunk_start += self.cur_chunk.len();
                    self.cur_chunk = self.chunks.next().unwrap_or("");
                }
                Err(GraphemeIncomplete::PreContext(idx)) => {
                    let (chunk, byte_idx, _, _) = self.text.chunk_at_byte(idx.saturating_sub(1));
                    self.cursor.provide_context(chunk, byte_idx);
                }
                _ => unreachable!(),
            }
        }

        if a < self.cur_chunk_start {
            let a_char = self.text.byte_to_char(a);
            let b_char = self.text.byte_to_char(b);

            Some(self.text.slice(a_char..b_char))
        } else {
            let a2 = a - self.cur_chunk_start;
            let b2 = b - self.cur_chunk_start;
            Some((&self.cur_chunk[a2..b2]).into())
        }
    }
}

pub fn grapheme_width_and_chars_len(g: RopeSlice) -> (usize, usize) {
    if let Some(g_str) = g.as_str() {
        (UnicodeWidthStr::width(g_str), g_str.chars().count())
    } else {
        let g_string = g.to_string();
        let g_str = g_string.as_str();
        (UnicodeWidthStr::width(g_str), g_str.chars().count())
    }
}

pub fn grapheme_width_and_bytes_len(g: RopeSlice) -> (usize, usize) {
    if let Some(g_str) = g.as_str() {
        (UnicodeWidthStr::width(g_str), g_str.len())
    } else {
        let g_string = g.to_string();
        let g_str = g_string.as_str();
        (UnicodeWidthStr::width(g_str), g_str.len())
    }
}

pub fn grapheme_width(g: RopeSlice) -> usize {
    if let Some(s) = g.as_str() {
        UnicodeWidthStr::width(s)
    } else {
        let s = g.to_string();
        UnicodeWidthStr::width(s.as_str())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert() {
        let mut code = Code::new("", "", None).unwrap();
        code.insert(0, "Hello ");
        code.insert(6, "World");
        assert_eq!(code.content.to_string(), "Hello World");
    }

    #[test]
    fn test_remove() {
        let mut code = Code::new("Hello World", "", None).unwrap();
        code.remove(5, 11);
        assert_eq!(code.content.to_string(), "Hello");
    }

    #[test]
    fn test_undo() {
        let mut code = Code::new("", "", None).unwrap();

        code.tx();
        code.insert(0, "Hello ");
        code.commit();

        code.tx();
        code.insert(6, "World");
        code.commit();

        code.undo();
        assert_eq!(code.content.to_string(), "Hello ");

        code.undo();
        assert_eq!(code.content.to_string(), "");
    }

    #[test]
    fn test_redo() {
        let mut code = Code::new("", "", None).unwrap();

        code.tx();
        code.insert(0, "Hello");
        code.commit();

        code.undo();
        assert_eq!(code.content.to_string(), "");

        code.redo();
        assert_eq!(code.content.to_string(), "Hello");
    }

    #[test]
    fn test_indentation_level0() {
        let mut code = Code::new("", "unknown", None).unwrap();
        code.insert(0, "    hello world");
        assert_eq!(code.indentation_level(0, 10), 0);
    }

    #[test]
    fn test_indentation_level() {
        let mut code = Code::new("", "python", None).unwrap();
        code.insert(0, "    print('Hello, World!')");
        assert_eq!(code.indentation_level(0, 10), 1);
    }

    #[test]
    fn test_indentation_level2() {
        let mut code = Code::new("", "python", None).unwrap();
        code.insert(0, "        print('Hello, World!')");
        assert_eq!(code.indentation_level(0, 10), 2);
    }

    #[test]
    fn test_is_only_indentation_before() {
        let mut code = Code::new("", "python", None).unwrap();
        code.insert(0, "    print('Hello, World!')");
        assert_eq!(code.is_only_indentation_before(0, 4), true);
        assert_eq!(code.is_only_indentation_before(0, 10), false);
    }

    #[test]
    fn test_is_only_indentation_before2() {
        let mut code = Code::new("", "", None).unwrap();
        code.insert(0, "    Hello, World");
        assert_eq!(code.is_only_indentation_before(0, 4), false);
        assert_eq!(code.is_only_indentation_before(0, 10), false);
    }

    #[test]
    fn test_smart_paste_1() {
        let initial = "fn foo() {\n    let x = 1;\n    \n}";
        let mut code = Code::new(initial, "rust", None).unwrap();

        let offset = 30;
        let paste = "if start == end && start == self.code.len() {\n    return;\n}";
        code.smart_paste(offset, paste);

        let expected =
            "fn foo() {\n    let x = 1;\n    if start == end && start == self.code.len() {\n        return;\n    }\n}";
        assert_eq!(code.get_content(), expected);
    }

    #[test]
    fn test_smart_paste_2() {
        let initial = "fn foo() {\n    let x = 1;\n    \n}";
        let mut code = Code::new(initial, "rust", None).unwrap();

        let offset = 30;
        let paste = "    if start == end && start == self.code.len() {\n        return;\n    }";
        code.smart_paste(offset, paste);

        let expected =
            "fn foo() {\n    let x = 1;\n    if start == end && start == self.code.len() {\n        return;\n    }\n}";
        assert_eq!(code.get_content(), expected);
    }
}

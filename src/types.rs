use ratatui_core::style::Style;
use std::collections::HashMap;

// keyword and ratatui style
pub(crate) type Theme = HashMap<String, Style>;
// start byte, end byte, style
pub(crate) type Hightlight = (usize, usize, Style);
// source id, start offset, end offset
pub(crate) type HightlightCache = HashMap<(u8, usize, usize), Vec<Hightlight>>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LineDiff {
    pub(crate) deletions: Vec<(usize, usize)>,
    pub(crate) additions: Vec<(usize, usize)>,
}

pub(crate) type LineDiffCache = HashMap<(usize, usize), LineDiff>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum VisualRow {
    Real {
        line_idx: usize,
        is_added: bool,
        orig_line_idx: Option<usize>,
    },
    FoldSeparator {
        hidden_lines: usize,
        hidden_start: usize,
        hidden_end: usize,
    },
    GhostDeleted {
        anchor_line: usize,
        original_line_idx: usize,
        curr_line_idx: Option<usize>,
    },
}

impl VisualRow {
    pub(crate) fn is_changed(&self) -> bool {
        match self {
            VisualRow::Real { is_added, .. } => *is_added,
            VisualRow::FoldSeparator { .. } => false,
            VisualRow::GhostDeleted { .. } => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiffOptions {
    pub focus_context: usize,
    pub expand_amount: usize,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            focus_context: 3,
            expand_amount: 5,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FoldIndicators {
    pub expanded: String,
    pub collapsed: String,
}

impl FoldIndicators {
    pub fn unicode() -> Self {
        Self {
            expanded: "▼".into(),
            collapsed: "▶".into(),
        }
    }
    pub fn ascii() -> Self {
        Self {
            expanded: "v".into(),
            collapsed: ">".into(),
        }
    }
}

impl Default for FoldIndicators {
    fn default() -> Self {
        Self::unicode()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeFoldingOptions {
    pub enabled: bool,
    pub indicators: FoldIndicators,
}

impl Default for CodeFoldingOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            indicators: FoldIndicators::default(),
        }
    }
}

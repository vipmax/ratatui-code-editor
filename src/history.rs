use std::collections::VecDeque;

#[derive(Clone)]
pub enum EditKind {
    Insert { offset: usize, text: String },
    Remove { offset: usize, text: String },
}

#[derive(Clone)]
pub struct Edit {
    pub kind: EditKind,
}

pub type EditBatch = Vec<Edit>;

pub struct History {
    index: usize,
    max_items: usize,
    edits: VecDeque<EditBatch>,
}

impl History {
    pub fn new(max_items: usize) -> Self {
        Self {
            index: 0,
            max_items,
            edits: VecDeque::new(),
        }
    }

    pub fn push(&mut self, batch: EditBatch) {
        while self.edits.len() > self.index {
            self.edits.pop_back();
        }

        if self.edits.len() == self.max_items {
            self.edits.pop_front();
            self.index -= 1;
        }

        self.edits.push_back(batch);
        self.index += 1;
    }

    pub fn undo(&mut self) -> Option<EditBatch> {
        if self.index == 0 {
            None
        } else {
            self.index -= 1;
            self.edits.get(self.index).cloned()
        }
    }

    pub fn redo(&mut self) -> Option<EditBatch> {
        if self.index >= self.edits.len() {
            None
        } else {
            let batch = self.edits.get(self.index).cloned();
            self.index += 1;
            batch
        }
    }
}
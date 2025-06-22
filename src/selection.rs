#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub start: usize, 
    pub end: usize,  
}

impl Selection {
    pub fn new(a: usize, b: usize) -> Self {
        Self {
            start: a.min(b),
            end: a.max(b),
        }
    }
    
    pub fn from_anchor_and_cursor(anchor: usize, cursor: usize) -> Self {
        if anchor <= cursor {
            Selection { start: anchor, end: cursor }
        } else {
            Selection { start: cursor, end: anchor }
        }
    }

    pub fn is_active(&self) -> bool {
        self.start != self.end
    }

    pub fn contains(&self, index: usize) -> bool {
        index >= self.start && index < self.end
    }
}

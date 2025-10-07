use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickKind {
    Single,
    Double,
    Triple,
}

#[derive(Debug, Clone, Copy)]
pub struct ClickTracker {
    pub last: Option<(Instant, usize)>,
    pub prev: Option<(Instant, usize)>,
    pub max_dt: Duration,
}

impl ClickTracker {
    pub fn new(max_dt: Duration) -> Self {
        Self { last: None, prev: None, max_dt }
    }

    pub fn register(&mut self, cursor: usize) -> ClickKind {
        let now = Instant::now();
        let dbl = self.last
            .map(|(t, p)| p == cursor && now.duration_since(t) < self.max_dt)
            .unwrap_or(false);
        let tpl = self.last.zip(self.prev)
            .map(|((t1, p1), (t0, p0))| {
                p0 == cursor && p1 == cursor &&
                now.duration_since(t0) < self.max_dt &&
                t1.duration_since(t0) < self.max_dt
            })
            .unwrap_or(false);

        self.prev = self.last;
        self.last = Some((now, cursor));

        if tpl { ClickKind::Triple }
        else if dbl { ClickKind::Double }
        else { ClickKind::Single }
    }
}



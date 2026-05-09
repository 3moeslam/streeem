#![cfg_attr(test, allow(clippy::unwrap_used))]
//! Bounded ring buffer of OutputLine values per tile.
//!
//! Rule (per spec §11): when the buffer is full, push evicts the oldest
//! line in O(1) and returns a `LinesDropped(1)` marker. Consecutive drops
//! collapse into one marker (e.g. 5 evictions in a row → one
//! `LinesDropped(5)`) to keep the visible noise low.

use std::collections::VecDeque;

use crate::output_line::OutputLine;
use crate::scrollback_capacity::ScrollbackCapacity;

#[derive(Debug, Clone)]
pub struct Scrollback {
    capacity: ScrollbackCapacity,
    lines: VecDeque<OutputLine>,
}

impl Scrollback {
    pub fn new(capacity: ScrollbackCapacity) -> Self {
        Self {
            capacity,
            lines: VecDeque::new(),
        }
    }

    pub fn push(&mut self, line: OutputLine) {
        if self.lines.len() >= self.capacity.value() {
            let evicted = self.lines.pop_front();
            // How many logical lines did the evicted slot represent?
            let evicted_count = match evicted {
                Some(OutputLine::LinesDropped(n)) => n.saturating_add(1),
                Some(_) => 1,
                None => 1,
            };
            self.bump_or_insert_dropped_marker(evicted_count);
        }
        self.lines.push_back(line);
    }

    fn bump_or_insert_dropped_marker(&mut self, count: usize) {
        if let Some(OutputLine::LinesDropped(n)) = self.lines.front_mut() {
            *n = n.saturating_add(count);
        } else {
            self.lines.push_front(OutputLine::LinesDropped(count));
        }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &OutputLine> {
        self.lines.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(n: usize) -> ScrollbackCapacity {
        ScrollbackCapacity::new(n.max(ScrollbackCapacity::MIN)).unwrap()
    }

    #[test]
    fn starts_empty() {
        let s = Scrollback::new(cap(100));
        assert!(s.is_empty());
    }

    #[test]
    fn push_appends_until_capacity() {
        let mut s = Scrollback::new(cap(100));
        for i in 0..50 {
            s.push(OutputLine::plain_text(format!("line {i}")));
        }
        assert_eq!(s.len(), 50);
    }

    #[test]
    fn push_at_capacity_evicts_oldest_and_inserts_marker() {
        let mut s = Scrollback::new(cap(100));
        for i in 0..100 {
            s.push(OutputLine::plain_text(format!("line {i}")));
        }
        s.push(OutputLine::plain_text("overflow"));
        let first = s.iter().next().unwrap();
        assert_eq!(*first, OutputLine::LinesDropped(1));
    }

    #[test]
    fn consecutive_drops_collapse_into_single_marker() {
        let mut s = Scrollback::new(cap(100));
        for i in 0..100 {
            s.push(OutputLine::plain_text(format!("line {i}")));
        }
        for i in 0..5 {
            s.push(OutputLine::plain_text(format!("over {i}")));
        }
        let first = s.iter().next().unwrap();
        assert_eq!(*first, OutputLine::LinesDropped(5));
    }
}

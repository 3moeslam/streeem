//! A monotonic, opaque identifier for a hosted tile.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TileId(u32);

impl TileId {
    pub fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Default)]
pub struct TileIdFactory {
    next: u32,
}

impl TileIdFactory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_id(&mut self) -> TileId {
        let id = TileId(self.next);
        self.next = self.next.saturating_add(1);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_starts_at_zero() {
        let mut f = TileIdFactory::new();
        assert_eq!(f.next_id(), TileId(0));
    }

    #[test]
    fn factory_increments_monotonically() {
        let mut f = TileIdFactory::new();
        let a = f.next_id();
        let b = f.next_id();
        let c = f.next_id();
        assert!(a < b && b < c);
        assert_eq!(b.raw(), a.raw() + 1);
    }

    #[test]
    fn factory_saturates_at_u32_max_without_panicking() {
        let mut f = TileIdFactory { next: u32::MAX };
        let last = f.next_id();
        let after = f.next_id();
        assert_eq!(last, TileId(u32::MAX));
        assert_eq!(after, TileId(u32::MAX));
    }
}

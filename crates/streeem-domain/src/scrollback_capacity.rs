#![cfg_attr(test, allow(clippy::unwrap_used))]
//! Maximum number of lines retained per tile. Default 10_000; minimum 100.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScrollbackCapacity(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbackCapacityError {
    BelowMinimum,
}

impl ScrollbackCapacity {
    pub const MIN: usize = 100;
    pub const DEFAULT: usize = 10_000;

    pub fn new(value: usize) -> Result<Self, ScrollbackCapacityError> {
        if value < Self::MIN {
            Err(ScrollbackCapacityError::BelowMinimum)
        } else {
            Ok(Self(value))
        }
    }

    pub fn value(self) -> usize {
        self.0
    }
}

impl Default for ScrollbackCapacity {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_ten_thousand() {
        assert_eq!(ScrollbackCapacity::default().value(), 10_000);
    }

    #[test]
    fn rejects_below_minimum() {
        assert_eq!(
            ScrollbackCapacity::new(99),
            Err(ScrollbackCapacityError::BelowMinimum)
        );
    }

    #[test]
    fn accepts_minimum() {
        assert_eq!(ScrollbackCapacity::new(100).map(|c| c.value()), Ok(100));
    }
}

#![cfg_attr(test, allow(clippy::unwrap_used))]
//! Number of columns in the staggered grid. Bounded 1..=32.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ColumnCount(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnCountError {
    BelowMinimum,
    AboveMaximum,
}

impl ColumnCount {
    pub const MIN: u16 = 1;
    pub const MAX: u16 = 32;

    pub fn new(value: u16) -> Result<Self, ColumnCountError> {
        if value < Self::MIN {
            Err(ColumnCountError::BelowMinimum)
        } else if value > Self::MAX {
            Err(ColumnCountError::AboveMaximum)
        } else {
            Ok(Self(value))
        }
    }

    pub fn value(self) -> u16 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero() {
        assert_eq!(ColumnCount::new(0), Err(ColumnCountError::BelowMinimum));
    }

    #[test]
    fn accepts_one() {
        assert_eq!(ColumnCount::new(1).map(|c| c.value()), Ok(1));
    }

    #[test]
    fn rejects_above_thirty_two() {
        assert_eq!(ColumnCount::new(33), Err(ColumnCountError::AboveMaximum));
    }
}

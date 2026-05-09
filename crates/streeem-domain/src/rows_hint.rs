#![cfg_attr(test, allow(clippy::unwrap_used))]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
//! Per-tile row-count hint. Bounded 1..=200; default 10.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RowsHint(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowsHintError {
    BelowMinimum,
    AboveMaximum,
}

impl RowsHint {
    pub const MIN: u16 = 1;
    pub const MAX: u16 = 200;
    pub const DEFAULT: u16 = 10;

    pub fn new(value: u16) -> Result<Self, RowsHintError> {
        if value < Self::MIN {
            Err(RowsHintError::BelowMinimum)
        } else if value > Self::MAX {
            Err(RowsHintError::AboveMaximum)
        } else {
            Ok(Self(value))
        }
    }

    pub fn value(self) -> u16 {
        self.0
    }

    pub fn saturating_add(self, delta: i16) -> Self {
        let next = (self.0 as i32 + delta as i32).clamp(Self::MIN as i32, Self::MAX as i32) as u16;
        Self(next)
    }
}

impl Default for RowsHint {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_ten() {
        assert_eq!(RowsHint::default().value(), 10);
    }

    #[test]
    fn rejects_zero() {
        assert_eq!(RowsHint::new(0), Err(RowsHintError::BelowMinimum));
    }

    #[test]
    fn rejects_above_two_hundred() {
        assert_eq!(RowsHint::new(201), Err(RowsHintError::AboveMaximum));
    }

    #[test]
    fn accepts_boundary_values() {
        assert!(RowsHint::new(1).is_ok());
        assert!(RowsHint::new(200).is_ok());
    }

    #[test]
    fn saturating_add_clamps_to_min() {
        let r = RowsHint::new(2).unwrap();
        assert_eq!(r.saturating_add(-10), RowsHint::new(1).unwrap());
    }

    #[test]
    fn saturating_add_clamps_to_max() {
        let r = RowsHint::new(195).unwrap();
        assert_eq!(r.saturating_add(20), RowsHint::new(200).unwrap());
    }
}

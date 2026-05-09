#![cfg_attr(
    any(test, feature = "test-support"),
    allow(clippy::expect_used, clippy::unwrap_used)
)]
//! Returns the current terminal size in columns x rows.

pub trait TerminalSize: Send + Sync {
    fn size(&self) -> (u16, u16);
}

#[cfg(any(test, feature = "test-support"))]
pub mod fakes {
    use super::TerminalSize;
    use std::sync::Mutex;

    pub struct FakeTerminalSize {
        size: Mutex<(u16, u16)>,
    }

    impl FakeTerminalSize {
        pub fn new(width: u16, height: u16) -> Self {
            Self {
                size: Mutex::new((width, height)),
            }
        }

        pub fn set(&self, width: u16, height: u16) {
            *self.size.lock().expect("FakeTerminalSize mutex poisoned") = (width, height);
        }
    }

    impl TerminalSize for FakeTerminalSize {
        fn size(&self) -> (u16, u16) {
            *self.size.lock().expect("FakeTerminalSize mutex poisoned")
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn returns_initial_then_updated_size() {
            let s = FakeTerminalSize::new(80, 30);
            assert_eq!(s.size(), (80, 30));
            s.set(120, 40);
            assert_eq!(s.size(), (120, 40));
        }
    }
}

#![cfg_attr(
    any(test, feature = "test-support"),
    allow(clippy::expect_used, clippy::unwrap_used)
)]
//! Read-only access to "now" for the application layer.

use std::time::Instant;

pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

#[cfg(any(test, feature = "test-support"))]
pub mod fakes {
    use super::Clock;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    pub struct FakeClock {
        current: Mutex<Instant>,
    }

    impl FakeClock {
        pub fn new(start: Instant) -> Self {
            Self {
                current: Mutex::new(start),
            }
        }

        pub fn advance(&self, by: Duration) {
            let mut guard = self
                .current
                .lock()
                .expect("FakeClock mutex poisoned in test");
            *guard += by;
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> Instant {
            *self
                .current
                .lock()
                .expect("FakeClock mutex poisoned in test")
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::time::Duration;

        #[test]
        fn advance_changes_now() {
            let start = Instant::now();
            let clock = FakeClock::new(start);
            clock.advance(Duration::from_secs(5));
            assert!(clock.now() >= start + Duration::from_secs(5));
        }
    }
}

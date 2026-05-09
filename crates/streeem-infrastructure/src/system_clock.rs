use std::time::Instant;
use streeem_domain::ports::clock::Clock;

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn now_advances_monotonically() {
        let c = SystemClock;
        let a = c.now();
        std::thread::sleep(Duration::from_millis(2));
        let b = c.now();
        assert!(b > a);
    }
}

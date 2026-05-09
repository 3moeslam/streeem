//! Result of a hosted process exit. Either an OS exit code or a terminating signal.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExitStatus {
    Code(i32),
    Signal(i32),
}

impl ExitStatus {
    pub fn is_success(self) -> bool {
        matches!(self, ExitStatus::Code(0))
    }

    pub fn label(self) -> String {
        match self {
            ExitStatus::Code(0) => "exit 0".to_string(),
            ExitStatus::Code(c) => format!("exit {c}"),
            ExitStatus::Signal(s) => format!("signal {s}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_zero_is_success() {
        assert!(ExitStatus::Code(0).is_success());
    }

    #[test]
    fn nonzero_code_is_not_success() {
        assert!(!ExitStatus::Code(1).is_success());
    }

    #[test]
    fn signal_is_not_success() {
        assert!(!ExitStatus::Signal(9).is_success());
    }

    #[test]
    fn labels_render_for_each_variant() {
        assert_eq!(ExitStatus::Code(0).label(), "exit 0");
        assert_eq!(ExitStatus::Code(137).label(), "exit 137");
        assert_eq!(ExitStatus::Signal(15).label(), "signal 15");
    }
}

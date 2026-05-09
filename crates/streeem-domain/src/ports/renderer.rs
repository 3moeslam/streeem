#![cfg_attr(
    any(test, feature = "test-support"),
    allow(clippy::expect_used, clippy::unwrap_used)
)]
//! Sink for FrameDescriptions (defined in streeem-presentation).
//!
//! The trait is generic over `F` so the domain doesn't need to know the
//! concrete FrameDescription type. The application layer threads the
//! presentation crate's FrameDescription as the `F` parameter.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderError(pub String);

pub trait Renderer<F>: Send {
    fn render(&mut self, frame: &F) -> Result<(), RenderError>;
}

#[cfg(any(test, feature = "test-support"))]
pub mod fakes {
    use super::*;
    use std::sync::Mutex;

    pub struct FakeRenderer<F: Clone + Send> {
        rendered: Mutex<Vec<F>>,
    }

    impl<F: Clone + Send> Default for FakeRenderer<F> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<F: Clone + Send> FakeRenderer<F> {
        pub fn new() -> Self {
            Self {
                rendered: Mutex::new(Vec::new()),
            }
        }
        pub fn frames(&self) -> Vec<F> {
            self.rendered.lock().expect("rendered mutex").clone()
        }
    }

    impl<F: Clone + Send> Renderer<F> for FakeRenderer<F> {
        fn render(&mut self, frame: &F) -> Result<(), RenderError> {
            self.rendered
                .lock()
                .expect("rendered mutex")
                .push(frame.clone());
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn records_each_frame_in_order() {
            let mut r: FakeRenderer<String> = FakeRenderer::new();
            r.render(&"a".to_string()).unwrap();
            r.render(&"b".to_string()).unwrap();
            assert_eq!(r.frames(), vec!["a".to_string(), "b".to_string()]);
        }
    }
}

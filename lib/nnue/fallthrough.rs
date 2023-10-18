use crate::nnue::Layer;

/// A fallthrough [`Layer`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Fallthrough;

impl<T: Copy, const N: usize> Layer<[T; N]> for Fallthrough {
    type Output = [T; N];

    fn forward(&self, input: &[T; N]) -> Self::Output {
        *input
    }
}

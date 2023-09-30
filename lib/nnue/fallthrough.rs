use crate::nnue::{Layer, Vector};
use num_traits::PrimInt;

/// A fallthrough [`Layer`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Fallthrough;

impl<I: PrimInt, const N: usize> Layer<Vector<I, N>> for Fallthrough {
    type Output = Vector<I, N>;

    fn forward(&self, input: &Vector<I, N>) -> Self::Output {
        *input
    }
}

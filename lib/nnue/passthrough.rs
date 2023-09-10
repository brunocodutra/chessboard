use crate::nnue::Layer;
use num_traits::PrimInt;

/// A passthrough [`Layer`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Passthrough;

impl<I: PrimInt, const N: usize> Layer<[I; N]> for Passthrough {
    type Output = [I; N];

    fn forward(&self, input: [I; N]) -> Self::Output {
        input
    }
}

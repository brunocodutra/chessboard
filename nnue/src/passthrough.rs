use crate::Layer;
use num_traits::PrimInt;
use test_strategy::Arbitrary;

/// A passthrough [`Layer`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub struct Passthrough;

impl<I: PrimInt, const N: usize> Layer<[I; N]> for Passthrough {
    type Output = [I; N];

    #[inline]
    fn forward(&self, input: [I; N]) -> Self::Output {
        input
    }
}

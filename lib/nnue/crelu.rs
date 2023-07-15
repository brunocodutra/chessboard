use crate::nnue::Layer;
use num_traits::{AsPrimitive, PrimInt};
use test_strategy::Arbitrary;

/// A clipped [rectifier][ReLU].
///
/// [ReLU]: https://en.wikipedia.org/wiki/Rectifier_(neural_networks)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub struct CReLU<L>(pub(super) L);

impl<L, I, T, const N: usize> Layer<I> for CReLU<L>
where
    L: Layer<I, Output = [T; N]>,
    T: PrimInt + AsPrimitive<i8> + From<i8>,
{
    type Output = [i8; N];

    fn forward(&self, input: I) -> Self::Output {
        self.0
            .forward(input)
            .map(|v| v.clamp(T::zero(), i8::MAX.into()).as_())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nnue::Passthrough;
    use test_strategy::proptest;

    #[proptest]
    fn clipped_relu_saturates_between_0_and_max(l: CReLU<Passthrough>, i: [i32; 3]) {
        assert_eq!(l.forward(i), i.map(|v| v.clamp(0, i8::MAX as _) as _));
    }
}

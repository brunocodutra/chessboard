use crate::Layer;
use num_traits::{AsPrimitive, PrimInt};
use test_strategy::Arbitrary;

/// A clipped [rectifier][ReLU].
///
/// [ReLU]: https://en.wikipedia.org/wiki/Rectifier_(neural_networks)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub struct CReLU;

impl<T: PrimInt + AsPrimitive<i8> + From<i8>, const N: usize> Layer<[T; N]> for CReLU {
    type Output = [i8; N];

    #[inline]
    fn forward(&self, input: [T; N]) -> Self::Output {
        input.map(|v| v.clamp(0.into(), i8::MAX.into()).as_())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn clipped_relu_saturates_between_0_and_max(l: CReLU, i: [i32; 3]) {
        assert_eq!(l.forward(i), i.map(|v| v.clamp(0, i8::MAX as _) as _));
    }
}

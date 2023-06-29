use crate::Layer;
use num_traits::PrimInt;
use test_strategy::Arbitrary;

/// Damps neuron activation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub struct Damp<const SCALE: i8>;

impl<T: PrimInt + From<i8>, const N: usize, const SCALE: i8> Layer<[T; N]> for Damp<SCALE> {
    type Output = [T; N];

    #[inline]
    fn forward(&self, input: [T; N]) -> Self::Output {
        input.map(|v| v / SCALE.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn damp_scales(l: Damp<8>, i: [i8; 3]) {
        assert_eq!(l.forward(i), i.map(|v| v / 8));
    }
}

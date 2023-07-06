use crate::Layer;
use num_traits::PrimInt;
use test_strategy::Arbitrary;

/// Damps neuron activation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub struct Damp<L, const SCALE: i8>(pub(crate) L);

impl<L, I, T, const N: usize, const SCALE: i8> Layer<I> for Damp<L, SCALE>
where
    L: Layer<I, Output = [T; N]>,
    T: PrimInt + From<i8>,
{
    type Output = [T; N];

    #[inline]
    fn forward(&self, input: I) -> Self::Output {
        self.0.forward(input).map(|v| v / SCALE.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Passthrough;
    use test_strategy::proptest;

    #[proptest]
    fn damp_scales(l: Damp<Passthrough, 8>, i: [i8; 3]) {
        assert_eq!(l.forward(i), i.map(|v| v / 8));
    }
}

use crate::nnue::{Layer, Vector};
use num_traits::AsPrimitive;

/// A clipped [rectifier][ReLU].
///
/// [ReLU]: https://en.wikipedia.org/wiki/Rectifier_(neural_networks)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct CReLU<L>(pub(super) L);

impl<L, T, const N: usize> Layer<Vector<T, N>> for CReLU<L>
where
    L: Layer<Vector<i8, N>>,
    T: Ord + AsPrimitive<i8> + From<i8>,
{
    type Output = L::Output;

    fn forward(&self, input: &Vector<T, N>) -> Self::Output {
        self.0
            .forward(&input.map(|v| v.clamp(0i8.into(), i8::MAX.into()).as_()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nnue::Fallthrough;
    use test_strategy::proptest;

    #[proptest]
    fn clipped_relu_saturates_between_0_and_max(i: [i32; 3]) {
        assert_eq!(
            CReLU(Fallthrough).forward(&i.into()),
            Vector(i.map(|v| v.clamp(0, i8::MAX as _) as _))
        );
    }
}

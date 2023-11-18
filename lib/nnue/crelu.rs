use crate::nnue::Layer;
use derive_more::Constructor;
use num_traits::AsPrimitive;

/// A clipped [rectifier][ReLU].
///
/// [ReLU]: https://en.wikipedia.org/wiki/Rectifier_(neural_networks)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Constructor)]
pub struct CReLU<L> {
    pub(super) next: L,
}

impl<L, T, const N: usize> Layer<[T; N]> for CReLU<L>
where
    L: Layer<[i8; N]>,
    T: Ord + AsPrimitive<i8> + From<i8>,
{
    type Output = L::Output;

    fn forward(&self, input: &[T; N]) -> Self::Output {
        self.next
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
            CReLU::new(Fallthrough).forward(&i),
            i.map(|v| v.clamp(0, i8::MAX as _) as _)
        );
    }
}

use crate::Layer;
use test_strategy::Arbitrary;

/// The composition of two [`Layer`]s in series.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub struct Chain<T, U>(pub(crate) T, pub(crate) U);

impl<I, T: Layer<I>, U: Layer<T::Output>> Layer<I> for Chain<T, U> {
    type Output = U::Output;

    #[inline]
    fn forward(&self, input: I) -> Self::Output {
        self.1.forward(self.0.forward(input))
    }
}

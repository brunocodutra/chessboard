use num_traits::PrimInt;

mod affine;
mod chain;
mod crelu;
mod damp;
mod feature;

use affine::*;
use chain::*;
use crelu::*;
use damp::*;
use feature::*;

/// Trait for types that can compose a neural network.
trait Layer<Input> {
    /// The transformed neurons.
    type Output;

    /// Transforms input neurons.
    fn forward(&self, input: Input) -> Self::Output;
}

/// Trait for types that can incrementally transform features.
trait Transformer {
    /// A type that can accumulate transformed features.
    type Accumulator;

    /// Refreshes accumulator.
    fn refresh(&self, features: &[usize], accumulator: &mut Self::Accumulator);

    /// Updates the accumulator by adding features.
    fn add(&self, feature: usize, accumulator: &mut Self::Accumulator);

    /// Updates the accumulator by removing features.
    fn remove(&self, feature: usize, accumulator: &mut Self::Accumulator);
}

impl<T: Transformer<Accumulator = [U; N]>, U: PrimInt, const N: usize> Layer<&[usize]> for T {
    type Output = [U; N];

    #[inline]
    fn forward(&self, input: &[usize]) -> Self::Output {
        let mut accumulator = [U::zero(); N];
        self.refresh(input, &mut accumulator);
        accumulator
    }
}

#![feature(const_maybe_uninit_write, const_mut_refs, const_transmute_copy)]

use num_traits::PrimInt;

mod affine;
mod chain;
mod crelu;
mod damp;
mod evaluator;
mod feature;
mod nn;
mod psqt;
mod transformer;

pub use affine::*;
pub use chain::*;
pub use crelu::*;
pub use damp::*;
pub use evaluator::*;
pub use feature::*;
pub use nn::*;
pub use psqt::*;
pub use transformer::*;

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

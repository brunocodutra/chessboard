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

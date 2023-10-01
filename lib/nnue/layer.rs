/// Trait for types that can compose a neural network.
pub trait Layer<Input: ?Sized> {
    /// The transformed neurons.
    type Output;

    /// Transforms input neurons.
    fn forward(&self, input: &Input) -> Self::Output;
}

use crate::nnue::Layer;

/// Damps neuron activation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Damp<L, const SCALE: i32>(pub(super) L);

impl<L, I, const N: usize, const SCALE: i32> Layer<I> for Damp<L, SCALE>
where
    L: Layer<I, Output = [i32; N]>,
{
    type Output = [i32; N];

    fn forward(&self, input: I) -> Self::Output {
        self.0.forward(input).map(|v| v / SCALE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nnue::Input;
    use test_strategy::proptest;

    #[proptest]
    fn damp_scales(l: Damp<Input, 8>, i: [i32; 3]) {
        assert_eq!(l.forward(i), i.map(|v| v / 8));
    }
}

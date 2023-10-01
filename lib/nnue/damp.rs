use crate::nnue::{Layer, Vector};

/// Damps neuron activation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Damp<L, const SCALE: i32>(pub(super) L);

impl<L, const N: usize, const SCALE: i32> Layer<Vector<i32, N>> for Damp<L, SCALE>
where
    L: Layer<Vector<i32, N>>,
{
    type Output = L::Output;

    fn forward(&self, input: &Vector<i32, N>) -> Self::Output {
        self.0.forward(&input.map(|v| v / SCALE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nnue::Fallthrough;
    use test_strategy::proptest;

    #[proptest]
    fn damp_scales(i: [i32; 3]) {
        assert_eq!(
            Damp::<_, 8>(Fallthrough).forward(&i.into()),
            Vector(i.map(|v| v / 8))
        );
    }
}

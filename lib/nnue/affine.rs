use crate::nnue::{Axpy, Layer};
use derive_more::Constructor;

/// An [affine] transformer.
///
/// [affine]: https://en.wikipedia.org/wiki/Affine_transformation
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Constructor)]
#[repr(align(64))]
pub struct Affine<L, const I: usize, const O: usize> {
    pub(super) bias: [i32; O],
    pub(super) weight: [[i8; I]; O],
    pub(super) next: L,
}

impl<L: Layer<[i32; O]>, const I: usize, const O: usize> Layer<[i8; I]> for Affine<L, I, O> {
    type Output = L::Output;

    fn forward(&self, input: &[i8; I]) -> Self::Output {
        let mut output = self.bias;
        output.axpy(&self.weight, input);
        self.next.forward(&output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nnue::Fallthrough;
    use test_strategy::proptest;

    #[proptest]
    fn affine_multiplies_by_weight_matrix(w: [[i8; 3]; 2], i: [i8; 3]) {
        let mut y = [0; 2];
        y.axpy(&w, &i);

        assert_eq!(Affine::new([0; 2], w, Fallthrough).forward(&i), y);
    }

    #[proptest]
    fn affine_adds_bias_vector(b: [i32; 3], i: [i8; 1]) {
        assert_eq!(
            Affine::new(b, [[1]; 3], Fallthrough).forward(&i),
            [i[0] as i32 + b[0], i[0] as i32 + b[1], i[0] as i32 + b[2]]
        );
    }
}

use crate::nnue::{Axpy, Layer, Matrix, Vector};

/// An [affine] transformer.
///
/// [affine]: https://en.wikipedia.org/wiki/Affine_transformation
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Affine<L, const I: usize, const O: usize>(
    pub(super) Matrix<i8, I, O>,
    pub(super) Vector<i32, O>,
    pub(super) L,
);

impl<L: Layer<Vector<i32, O>>, const I: usize, const O: usize> Layer<Vector<i8, I>>
    for Affine<L, I, O>
{
    type Output = L::Output;

    fn forward(&self, input: &Vector<i8, I>) -> Self::Output {
        let mut output = self.1;
        output.axpy(&self.0, input);
        self.2.forward(&output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nnue::Fallthrough;
    use test_strategy::proptest;

    #[proptest]
    fn affine_multiplies_by_weight_matrix(w: [[i8; 3]; 2], i: [i8; 3]) {
        assert_eq!(
            Affine(w.into(), Vector::default(), Fallthrough).forward(&i.into()),
            Vector([
                i[0] as i32 * w[0][0] as i32
                    + i[1] as i32 * w[0][1] as i32
                    + i[2] as i32 * w[0][2] as i32,
                i[0] as i32 * w[1][0] as i32
                    + i[1] as i32 * w[1][1] as i32
                    + i[2] as i32 * w[1][2] as i32,
            ])
        );
    }

    #[proptest]
    fn affine_adds_bias_vector(b: [i32; 3], i: [i8; 1]) {
        assert_eq!(
            Affine(Matrix([[1]; 3]), b.into(), Fallthrough).forward(&i.into()),
            Vector([i[0] as i32 + b[0], i[0] as i32 + b[1], i[0] as i32 + b[2]])
        );
    }
}

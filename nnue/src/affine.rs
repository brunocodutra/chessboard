use crate::Layer;
use test_strategy::Arbitrary;

/// An [affine] transformer.
///
/// [affine]: https://en.wikipedia.org/wiki/Affine_transformation
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub struct Affine<const I: usize, const O: usize>(pub [[i8; I]; O], pub [i32; O]);

impl<const I: usize, const O: usize> Layer<[i8; I]> for Affine<I, O> {
    type Output = [i32; O];

    #[inline]
    fn forward(&self, input: [i8; I]) -> Self::Output {
        let mut output = self.1;
        for (i, o) in output.iter_mut().enumerate() {
            for (j, v) in input.iter().enumerate() {
                *o += *v as i32 * self.0[i][j] as i32
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn affine_multiplies_by_weight_matrix(w: [[i8; 3]; 2], i: [i8; 3]) {
        assert_eq!(
            Affine(w, [0; 2]).forward(i),
            [
                i[0] as i32 * w[0][0] as i32
                    + i[1] as i32 * w[0][1] as i32
                    + i[2] as i32 * w[0][2] as i32,
                i[0] as i32 * w[1][0] as i32
                    + i[1] as i32 * w[1][1] as i32
                    + i[2] as i32 * w[1][2] as i32,
            ]
        );
    }

    #[proptest]
    fn affine_adds_bias_vector(b: [i32; 2], i: [i8; 2]) {
        assert_eq!(
            Affine([[1, 0], [0, 1]], b).forward(i),
            [i[0] as i32 + b[0], i[1] as i32 + b[1]]
        );
    }
}

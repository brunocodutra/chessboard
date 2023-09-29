use crate::nnue::Layer;

/// An [affine] transformer.
///
/// [affine]: https://en.wikipedia.org/wiki/Affine_transformation
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Affine<L, const I: usize, const O: usize>(
    pub(super) [[i8; I]; O],
    pub(super) [i32; O],
    pub(super) L,
);

impl<L: Layer<[i32; O]>, const I: usize, const O: usize> Layer<[i8; I]> for Affine<L, I, O> {
    type Output = L::Output;

    fn forward(&self, input: &[i8; I]) -> Self::Output {
        debug_assert_eq!(O % 8, 0);

        let mut output = self.1;
        for (i, o) in output.chunks_mut(8).enumerate() {
            for (j, v) in input.iter().enumerate() {
                o[0] += *v as i32 * self.0[i * 8][j] as i32;
                o[1] += *v as i32 * self.0[i * 8 + 1][j] as i32;
                o[2] += *v as i32 * self.0[i * 8 + 2][j] as i32;
                o[3] += *v as i32 * self.0[i * 8 + 3][j] as i32;
                o[4] += *v as i32 * self.0[i * 8 + 4][j] as i32;
                o[5] += *v as i32 * self.0[i * 8 + 5][j] as i32;
                o[6] += *v as i32 * self.0[i * 8 + 6][j] as i32;
                o[7] += *v as i32 * self.0[i * 8 + 7][j] as i32;
            }
        }

        self.2.forward(&output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nnue::Fallthrough;
    use test_strategy::proptest;

    #[proptest]
    fn affine_multiplies_by_weight_matrix(w: [[i8; 3]; 8], i: [i8; 3]) {
        assert_eq!(
            Affine(w, [0; 8], Fallthrough).forward(&i),
            [
                i[0] as i32 * w[0][0] as i32
                    + i[1] as i32 * w[0][1] as i32
                    + i[2] as i32 * w[0][2] as i32,
                i[0] as i32 * w[1][0] as i32
                    + i[1] as i32 * w[1][1] as i32
                    + i[2] as i32 * w[1][2] as i32,
                i[0] as i32 * w[2][0] as i32
                    + i[1] as i32 * w[2][1] as i32
                    + i[2] as i32 * w[2][2] as i32,
                i[0] as i32 * w[3][0] as i32
                    + i[1] as i32 * w[3][1] as i32
                    + i[2] as i32 * w[3][2] as i32,
                i[0] as i32 * w[4][0] as i32
                    + i[1] as i32 * w[4][1] as i32
                    + i[2] as i32 * w[4][2] as i32,
                i[0] as i32 * w[5][0] as i32
                    + i[1] as i32 * w[5][1] as i32
                    + i[2] as i32 * w[5][2] as i32,
                i[0] as i32 * w[6][0] as i32
                    + i[1] as i32 * w[6][1] as i32
                    + i[2] as i32 * w[6][2] as i32,
                i[0] as i32 * w[7][0] as i32
                    + i[1] as i32 * w[7][1] as i32
                    + i[2] as i32 * w[7][2] as i32,
            ]
        );
    }

    #[proptest]
    fn affine_adds_bias_vector(b: [i32; 8], i: [i8; 2]) {
        assert_eq!(
            Affine([[1, 1]; 8], b, Fallthrough).forward(&i),
            [
                i[0] as i32 + i[1] as i32 + b[0],
                i[0] as i32 + i[1] as i32 + b[1],
                i[0] as i32 + i[1] as i32 + b[2],
                i[0] as i32 + i[1] as i32 + b[3],
                i[0] as i32 + i[1] as i32 + b[4],
                i[0] as i32 + i[1] as i32 + b[5],
                i[0] as i32 + i[1] as i32 + b[6],
                i[0] as i32 + i[1] as i32 + b[7],
            ]
        );
    }
}

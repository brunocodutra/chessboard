use crate::nnue::Layer;

/// An [affine] transformer.
///
/// [affine]: https://en.wikipedia.org/wiki/Affine_transformation
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Affine<L, const I: usize, const O: usize>(
    pub(super) L,
    pub(super) [[i8; I]; O],
    pub(super) [i32; O],
);

impl<L, T, const I: usize, const O: usize> Layer<T> for Affine<L, I, O>
where
    L: Layer<T, Output = [i8; I]>,
{
    type Output = [i32; O];

    fn forward(&self, input: T) -> Self::Output {
        let input = self.0.forward(input);
        let mut output = self.2;
        for (i, o) in output.iter_mut().enumerate() {
            for (j, v) in input.iter().enumerate() {
                *o += *v as i32 * self.1[i][j] as i32
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nnue::Passthrough;
    use test_strategy::proptest;

    #[proptest]
    fn affine_multiplies_by_weight_matrix(w: [[i8; 3]; 2], i: [i8; 3]) {
        assert_eq!(
            Affine(Passthrough, w, [0; 2]).forward(i),
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
            Affine(Passthrough, [[1, 0], [0, 1]], b).forward(i),
            [i[0] as i32 + b[0], i[1] as i32 + b[1]]
        );
    }
}

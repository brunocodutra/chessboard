use crate::nnue::Layer;

/// The output transformer.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Output<L, const I: usize>(pub(super) L, pub(super) [i8; I], pub(super) i32);

impl<L, T, const I: usize> Layer<T> for Output<L, I>
where
    L: Layer<T, Output = [i8; I]>,
{
    type Output = i32;

    fn forward(&self, input: T) -> Self::Output {
        let mut output = self.2;
        let input = self.0.forward(input);
        for (v, w) in input.into_iter().zip(self.1) {
            output += v as i32 * w as i32;
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nnue::Input;
    use test_strategy::proptest;

    #[proptest]
    fn output_multiplies_by_weight_vector(w: [i8; 3], i: [i8; 3]) {
        assert_eq!(
            Output(Input, w, 0).forward(i),
            i[0] as i32 * w[0] as i32 + i[1] as i32 * w[1] as i32 + i[2] as i32 * w[2] as i32,
        );
    }

    #[proptest]
    fn output_adds_bias(b: i32, i: [i8; 2]) {
        assert_eq!(
            Output(Input, [1, 1], b).forward(i),
            i[0] as i32 + i[1] as i32 + b
        );
    }
}

use crate::nnue::{Axpy, Layer};

/// The output transformer.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Output<const I: usize>(pub(super) i32, pub(super) [i8; I]);

impl<const N: usize> Layer<[i8; N]> for Output<N> {
    type Output = i32;

    fn forward(&self, input: &[i8; N]) -> Self::Output {
        let mut output = self.0;
        output.axpy(&self.1, input);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn output_multiplies_by_weight_vector(w: [i8; 3], i: [i8; 3]) {
        let mut y = 0;
        y.axpy(&w, &i);

        assert_eq!(Output(0, w).forward(&i), y);
    }

    #[proptest]
    fn output_adds_bias(b: i32, i: [i8; 1]) {
        assert_eq!(Output(b, [1]).forward(&i), i[0] as i32 + b);
    }
}

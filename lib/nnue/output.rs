use crate::nnue::{Axpy, Layer, Vector};

/// The output transformer.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Output<const I: usize>(pub(super) Vector<i8, I>, pub(super) i32);

impl<const N: usize> Layer<Vector<i8, N>> for Output<N> {
    type Output = i32;

    fn forward(&self, input: &Vector<i8, N>) -> Self::Output {
        let mut output = self.1;
        output.axpy(&self.0, input);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn output_multiplies_by_weight_vector(w: [i8; 3], i: [i8; 3]) {
        assert_eq!(
            Output(w.into(), 0).forward(&i.into()),
            i[0] as i32 * w[0] as i32 + i[1] as i32 * w[1] as i32 + i[2] as i32 * w[2] as i32,
        );
    }

    #[proptest]
    fn output_adds_bias(b: i32, i: [i8; 1]) {
        assert_eq!(Output(Vector([1]), b).forward(&i.into()), i[0] as i32 + b);
    }
}

use crate::nnue::{Axpy, Layer, Matrix, Vector};
use std::ops::{AddAssign, SubAssign};

/// An [affine] feature transformer.
///
/// [affine]: https://en.wikipedia.org/wiki/Affine_transformation
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Transformer<T, const I: usize, const O: usize>(
    pub(super) Matrix<T, O, I>,
    pub(super) Vector<T, O>,
);

impl<T, const I: usize, const O: usize> Transformer<T, I, O>
where
    T: Copy + AddAssign + SubAssign,
{
    /// Refreshes accumulator.
    pub fn refresh(&self, features: &[usize], accumulator: &mut Vector<T, O>) {
        debug_assert!(features.len() <= 32);
        *accumulator = self.1;
        accumulator.axpy(&self.0, features)
    }

    /// Updates the accumulator by adding features.
    pub fn add(&self, feature: usize, accumulator: &mut Vector<T, O>) {
        for (i, a) in accumulator.iter_mut().enumerate() {
            *a += self.0[feature][i]
        }
    }

    /// Updates the accumulator by removing features.
    pub fn remove(&self, feature: usize, accumulator: &mut Vector<T, O>) {
        for (i, a) in accumulator.iter_mut().enumerate() {
            *a -= self.0[feature][i]
        }
    }
}

impl<T, const I: usize, const O: usize> Layer<[usize]> for Transformer<T, I, O>
where
    T: Default + Copy + AddAssign + SubAssign,
{
    type Output = Vector<T, O>;

    fn forward(&self, input: &[usize]) -> Self::Output {
        let mut accumulator = Vector::default();
        self.refresh(input, &mut accumulator);
        accumulator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nnue::Layer;
    use proptest::sample::size_range;
    use test_strategy::proptest;

    #[proptest]
    fn transformer_selects_weight_matrix(
        #[strategy([[-9i16..=9, -9..=9], [-9..=9, -9..=9], [-9..=9, -9..=9]])] w: [[i16; 2]; 3],
        #[strategy([..3usize, ..3, ..3])] i: [usize; 3],
    ) {
        assert_eq!(
            Transformer(w.into(), Vector::default()).forward(&i),
            Vector([
                w[i[0]][0] + w[i[1]][0] + w[i[2]][0],
                w[i[0]][1] + w[i[1]][1] + w[i[2]][1],
            ])
        );
    }

    #[proptest]
    fn transformer_adds_bias_vector(w: Matrix<i16, 2, 3>, b: Vector<i16, 2>) {
        assert_eq!(Transformer(w, b).forward(&[]), b);
    }

    #[proptest]
    #[should_panic]
    fn transformer_panics_if_too_many_inputs(
        #[strategy([[-9i16..=9, -9..=9], [-9..=9, -9..=9], [-9..=9, -9..=9]])] w: [[i16; 2]; 3],
        #[strategy([-9i16..=9, -9..=9])] b: [i16; 2],
        #[any(size_range(33..=99).lift())] i: Vec<usize>,
    ) {
        Transformer(w.into(), b.into()).forward(&i);
    }

    #[proptest]
    fn add_updates_accumulator(
        #[strategy([[-9i16..=9, -9..=9], [-9..=9, -9..=9], [-9..=9, -9..=9]])] w: [[i16; 2]; 3],
        b: [i16; 2],
        #[strategy([-9i16..=9, -9..=9])] prev: [i16; 2],
        #[strategy(..3usize)] f: usize,
    ) {
        let mut new = Vector(prev);
        Transformer(w.into(), b.into()).add(f, &mut new);
        assert_eq!(new, Vector([prev[0] + w[f][0], prev[1] + w[f][1]]));
    }

    #[proptest]
    fn remove_updates_accumulator(
        #[strategy([[-9i16..=9, -9..=9], [-9..=9, -9..=9], [-9..=9, -9..=9]])] w: [[i16; 2]; 3],
        b: [i16; 2],
        #[strategy([-9i16..=9, -9..=9])] prev: [i16; 2],
        #[strategy(..3usize)] f: usize,
    ) {
        let mut new = Vector(prev);
        Transformer(w.into(), b.into()).remove(f, &mut new);
        assert_eq!(new, Vector([prev[0] - w[f][0], prev[1] - w[f][1]]));
    }
}

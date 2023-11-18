use crate::nnue::{Axpy, Invert, Layer};
use derive_more::Constructor;
use std::ops::{AddAssign, SubAssign};

/// A feature transformer.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Constructor)]
#[repr(align(64))]
pub struct Transformer<T, const I: usize, const O: usize> {
    pub(super) bias: [T; O],
    pub(super) weight: [[T; O]; I],
}

impl<T, const I: usize, const O: usize> Transformer<T, I, O>
where
    T: Copy + AddAssign + SubAssign,
{
    /// Refreshes accumulator.
    pub fn refresh(&self, features: &[u16], accumulator: &mut [T; O]) {
        debug_assert!(features.len() <= 32);
        *accumulator = self.bias;
        accumulator.axpy(&self.weight, features)
    }

    /// Updates the accumulator by adding features.
    pub fn add(&self, feature: u16, accumulator: &mut [T; O]) {
        accumulator.axpy(&self.weight, &feature);
    }

    /// Updates the accumulator by removing features.
    pub fn remove(&self, feature: u16, accumulator: &mut [T; O]) {
        accumulator.axpy(&self.weight, &Invert(feature));
    }
}

impl<T, const I: usize, const O: usize> Layer<[u16]> for Transformer<T, I, O>
where
    T: Default + Copy + AddAssign + SubAssign,
{
    type Output = [T; O];

    fn forward(&self, input: &[u16]) -> Self::Output {
        let mut accumulator = [Default::default(); O];
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
        #[strategy([..3u16, ..3, ..3])] i: [u16; 3],
    ) {
        assert_eq!(
            Transformer::new([0; 2], w).forward(&i),
            [
                w[i[0] as usize][0] + w[i[1] as usize][0] + w[i[2] as usize][0],
                w[i[0] as usize][1] + w[i[1] as usize][1] + w[i[2] as usize][1],
            ]
        );
    }

    #[proptest]
    fn transformer_adds_bias_vector(b: [i16; 2], w: [[i16; 2]; 3]) {
        assert_eq!(Transformer::new(b, w).forward(&[]), b);
    }

    #[proptest]
    #[should_panic]
    fn transformer_panics_if_too_many_inputs(
        #[strategy([-9i16..=9, -9..=9])] b: [i16; 2],
        #[strategy([[-9i16..=9, -9..=9], [-9..=9, -9..=9], [-9..=9, -9..=9]])] w: [[i16; 2]; 3],
        #[any(size_range(33..=99).lift())] i: Vec<u16>,
    ) {
        Transformer::new(b, w).forward(&i);
    }

    #[proptest]
    fn add_updates_accumulator(
        b: [i16; 2],
        #[strategy([[-9i16..=9, -9..=9], [-9..=9, -9..=9], [-9..=9, -9..=9]])] w: [[i16; 2]; 3],
        #[strategy([-9i16..=9, -9..=9])] prev: [i16; 2],
        #[strategy(..3u16)] f: u16,
    ) {
        let mut new = prev;
        Transformer::new(b, w).add(f, &mut new);

        assert_eq!(
            new,
            [prev[0] + w[f as usize][0], prev[1] + w[f as usize][1]]
        );
    }

    #[proptest]
    fn remove_updates_accumulator(
        b: [i16; 2],
        #[strategy([[-9i16..=9, -9..=9], [-9..=9, -9..=9], [-9..=9, -9..=9]])] w: [[i16; 2]; 3],
        #[strategy([-9i16..=9, -9..=9])] prev: [i16; 2],
        #[strategy(..3u16)] f: u16,
    ) {
        let mut new = prev;
        Transformer::new(b, w).remove(f, &mut new);

        assert_eq!(
            new,
            [prev[0] - w[f as usize][0], prev[1] - w[f as usize][1]]
        );
    }
}

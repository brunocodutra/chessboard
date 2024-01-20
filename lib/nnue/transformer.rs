use crate::nnue::Layer;
use crate::util::{AlignTo64, Assume};
use derive_more::Constructor;
use std::ops::{AddAssign, SubAssign};

#[cfg(test)]
use proptest::prelude::*;

#[cfg(test)]
use std::fmt::Debug;

/// A feature transformer.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Constructor)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(bound(T: 'static + Debug + Arbitrary + From<i8>)))]
pub struct Transformer<T, const I: usize, const O: usize> {
    #[cfg_attr(test, map(|b: [i8; O]| AlignTo64(b.map(T::from))))]
    pub(super) bias: AlignTo64<[T; O]>,
    #[cfg_attr(test, map(|w:[ [i8; O]; I]| AlignTo64(w.map(|v| v.map(T::from)))))]
    pub(super) weight: AlignTo64<[[T; O]; I]>,
}

impl<T, const I: usize, const O: usize> Transformer<T, I, O>
where
    T: Copy + AddAssign + SubAssign,
{
    /// Refreshes accumulator.
    #[inline(always)]
    pub fn refresh(&self, features: &[u16], accumulator: &mut [T; O]) {
        debug_assert!(features.len() <= 32);
        *accumulator = *self.bias;
        for i in features {
            self.add(*i, accumulator)
        }
    }

    /// Updates the accumulator by adding features.
    #[inline(always)]
    pub fn add(&self, feature: u16, accumulator: &mut [T; O]) {
        let a = self.weight.get(feature as usize).assume();
        for (y, a) in accumulator.iter_mut().zip(a) {
            *y += *a
        }
    }

    /// Updates the accumulator by removing features.
    #[inline(always)]
    pub fn remove(&self, feature: u16, accumulator: &mut [T; O]) {
        let a = self.weight.get(feature as usize).assume();
        for (y, a) in accumulator.iter_mut().zip(a) {
            *y -= *a
        }
    }
}

impl<T, const I: usize, const O: usize> Layer<[u16]> for Transformer<T, I, O>
where
    T: Default + Copy + AddAssign + SubAssign,
{
    type Output = [T; O];

    #[inline(always)]
    fn forward(&self, input: &[u16]) -> Self::Output {
        let mut accumulator = [T::default(); O];
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
    fn transformer_selects_weight_matrix_and_adds_bias(
        t: Transformer<i16, 3, 2>,
        #[strategy([..3u16, ..3, ..3])] i: [u16; 3],
    ) {
        assert_eq!(
            t.forward(&i),
            [
                t.bias[0]
                    + t.weight[i[0] as usize][0]
                    + t.weight[i[1] as usize][0]
                    + t.weight[i[2] as usize][0],
                t.bias[1]
                    + t.weight[i[0] as usize][1]
                    + t.weight[i[1] as usize][1]
                    + t.weight[i[2] as usize][1],
            ]
        );
    }

    #[proptest]
    #[should_panic]
    fn transformer_panics_if_too_many_inputs(
        t: Transformer<i16, 3, 2>,
        #[any(size_range(33..=99).lift())] i: Vec<u16>,
    ) {
        t.forward(&i);
    }

    #[proptest]
    fn add_updates_accumulator(
        t: Transformer<i16, 3, 2>,
        #[map(|v: [i8; 2]| v.map(i16::from))] prev: [i16; 2],
        #[strategy(..3u16)] f: u16,
    ) {
        let mut new = prev;
        t.add(f, &mut new);

        assert_eq!(
            new,
            [
                prev[0] + t.weight[f as usize][0],
                prev[1] + t.weight[f as usize][1]
            ]
        );
    }

    #[proptest]
    fn remove_updates_accumulator(
        t: Transformer<i16, 3, 2>,
        #[map(|v: [i8; 2]| v.map(i16::from))] prev: [i16; 2],
        #[strategy(..3u16)] f: u16,
    ) {
        let mut new = prev;
        t.remove(f, &mut new);

        assert_eq!(
            new,
            [
                prev[0] - t.weight[f as usize][0],
                prev[1] - t.weight[f as usize][1]
            ]
        );
    }
}

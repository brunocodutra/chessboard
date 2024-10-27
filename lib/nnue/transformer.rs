use crate::nnue::Feature;
use crate::util::{AlignTo64, Assume, Integer};
use std::ops::{Add, AddAssign, Sub, SubAssign};

#[cfg(test)]
use proptest::{prelude::*, sample::Index};

#[cfg(test)]
use std::ops::Range;

/// A feature transformer.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Transformer<T, const N: usize> {
    pub(super) bias: AlignTo64<[T; N]>,
    pub(super) weight: AlignTo64<[[T; N]; Feature::LEN]>,
}

#[cfg(test)]
impl<const N: usize> Arbitrary for Box<Transformer<i16, N>> {
    type Parameters = Range<i16>;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(Range { start, end }: Self::Parameters) -> Self::Strategy {
        (any::<Index>())
            .prop_map(move |rng| {
                let mut transformer = unsafe { Self::new_zeroed().assume_init() };

                for v in transformer.bias.iter_mut() {
                    *v = rng.index((end - start) as _) as i16 + start
                }

                for v in &mut transformer.weight.iter_mut().flatten() {
                    *v = rng.index((end - start) as _) as i16 + start
                }

                transformer
            })
            .no_shrink()
            .boxed()
    }
}

impl<T, const N: usize> Transformer<T, N>
where
    T: Copy + Add<Output = T> + AddAssign + Sub<Output = T> + SubAssign,
{
    /// A fresh accumulator.
    #[inline(always)]
    pub fn fresh(&self) -> [T; N] {
        *self.bias
    }

    /// Updates the accumulator by adding features.
    #[inline(always)]
    pub fn add(&self, feature: Feature, accumulator: &mut [T; N]) {
        let a = self.weight.get(feature.cast::<usize>()).assume().iter();
        for (y, a) in accumulator.iter_mut().zip(a) {
            *y += *a;
        }
    }

    /// Updates the accumulator by removing features.
    #[inline(always)]
    pub fn remove(&self, feature: Feature, accumulator: &mut [T; N]) {
        let a = self.weight.get(feature.cast::<usize>()).assume().iter();
        for (y, a) in accumulator.iter_mut().zip(a) {
            *y -= *a;
        }
    }

    /// Updates the accumulator by replacing features.
    #[inline(always)]
    pub fn replace(&self, remove: Feature, add: Feature, accumulator: &mut [T; N]) {
        let a = self.weight.get(add.cast::<usize>()).assume().iter();
        let b = self.weight.get(remove.cast::<usize>()).assume().iter();
        for (y, (a, b)) in accumulator.iter_mut().zip(Iterator::zip(a, b)) {
            *y += *a - *b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::array::uniform3;
    use test_strategy::proptest;

    #[proptest]
    fn fresh_accumulator_equals_bias(#[any(-128i16..128)] t: Box<Transformer<i16, 2>>) {
        assert_eq!(t.fresh(), *t.bias);
    }

    #[proptest]
    fn add_updates_accumulator(
        #[any(-128i16..128)] t: Box<Transformer<i16, 3>>,
        ft: Feature,
        #[strategy(uniform3(-128..128i16))] prev: [i16; 3],
    ) {
        let mut new = prev;
        t.add(ft, &mut new);

        assert_eq!(
            new,
            [
                prev[0] + t.weight[ft.cast::<usize>()][0],
                prev[1] + t.weight[ft.cast::<usize>()][1],
                prev[2] + t.weight[ft.cast::<usize>()][2],
            ]
        );
    }

    #[proptest]
    fn remove_updates_accumulator(
        #[any(-128..128i16)] t: Box<Transformer<i16, 3>>,
        ft: Feature,
        #[strategy(uniform3(-128..128i16))] prev: [i16; 3],
    ) {
        let mut new = prev;
        t.remove(ft, &mut new);

        assert_eq!(
            new,
            [
                prev[0] - t.weight[ft.cast::<usize>()][0],
                prev[1] - t.weight[ft.cast::<usize>()][1],
                prev[2] - t.weight[ft.cast::<usize>()][2],
            ]
        );
    }
}

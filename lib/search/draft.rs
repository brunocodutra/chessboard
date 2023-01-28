use crate::util::Saturate;
use derive_more::{Display, Neg};
use test_strategy::Arbitrary;

#[derive(
    Debug, Display, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary, Neg,
)]
pub struct Draft(#[strategy(Self::MIN.get()..=Self::MAX.get())] i8);

impl Saturate for Draft {
    type Primitive = i8;

    const ZERO: Self = Draft(0);

    #[cfg(not(test))]
    const MIN: Self = Draft(-31);

    #[cfg(not(test))]
    const MAX: Self = Draft(31);

    #[cfg(test)]
    const MIN: Self = Draft(-3);

    #[cfg(test)]
    const MAX: Self = Draft(3);

    #[inline]
    fn new(i: Self::Primitive) -> Self {
        assert!((Self::MIN.get()..=Self::MAX.get()).contains(&i));
        Draft(i)
    }

    #[inline]
    fn get(&self) -> Self::Primitive {
        self.0
    }
}

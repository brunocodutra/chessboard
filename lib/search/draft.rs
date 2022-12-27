use derive_more::{DebugCustom, Display, Error, Neg};
use num_traits::{clamp, AsPrimitive};
use std::ops::{Add, Sub};
use test_strategy::Arbitrary;

#[derive(
    DebugCustom, Display, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary, Neg,
)]
#[debug(fmt = "Draft({})", self)]
#[display(fmt = "{:+}", _0)]
pub struct Draft(#[strategy(Self::MIN.get()..Self::MAX.get())] i8);

impl Draft {
    pub const ZERO: Self = Draft(0);

    #[cfg(not(test))]
    pub const MIN: Self = Draft(-31);
    #[cfg(test)]
    pub const MIN: Self = Draft(-3);

    #[cfg(not(test))]
    pub const MAX: Self = Draft(31);
    #[cfg(test)]
    pub const MAX: Self = Draft(3);

    /// Constructs [`Draft`] from a raw number.
    ///
    /// # Panics
    ///
    /// Panics if `d` is outside of the bounds.
    #[inline]
    pub fn new(d: i8) -> Self {
        d.try_into().unwrap()
    }

    /// Returns the raw draft.
    #[inline]
    pub fn get(&self) -> i8 {
        self.0
    }

    /// Safely constructs [`Draft`] from a raw value through saturation.
    #[inline]
    pub fn saturate<T: AsPrimitive<i8> + From<i8> + PartialOrd>(i: T) -> Self {
        Draft::new(clamp(i, Self::MIN.get().into(), Self::MAX.get().into()).as_())
    }
}

/// The reason why converting [`Draft`] from an integer failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(
    fmt = "expected integer in the range `({}..={})`",
    Draft::MIN,
    Draft::MAX
)]
pub struct DraftOutOfRange;

impl TryFrom<i8> for Draft {
    type Error = DraftOutOfRange;

    #[inline]
    fn try_from(n: i8) -> Result<Self, Self::Error> {
        if (Self::MIN.get()..=Self::MAX.get()).contains(&n) {
            Ok(Draft(n))
        } else {
            Err(DraftOutOfRange)
        }
    }
}

impl Add<i8> for Draft {
    type Output = Self;

    #[inline]
    fn add(self, rhs: i8) -> Self::Output {
        Draft::saturate(self.get().saturating_add(rhs))
    }
}

impl Add for Draft {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        self + rhs.get()
    }
}

impl Sub<i8> for Draft {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: i8) -> Self::Output {
        Draft::saturate(self.get().saturating_sub(rhs))
    }
}

impl Sub for Draft {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self - rhs.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn new_accepts_numbers_within_bounds(#[strategy(Draft::MIN.get()..=Draft::MAX.get())] d: i8) {
        assert_eq!(Draft::new(d).get(), d);
    }

    #[proptest]
    #[should_panic]
    fn new_panics_if_number_greater_than_max(#[strategy(Draft::MAX.get() + 1..)] d: i8) {
        Draft::new(d);
    }

    #[proptest]
    fn saturate_preserves_drafts_within_bounds(
        #[strategy(Draft::MIN.get()..=Draft::MAX.get())] d: i8,
    ) {
        assert_eq!(Draft::saturate(d), Draft::new(d));
    }

    #[proptest]
    fn saturate_caps_if_draft_greater_than_max(#[strategy(Draft::MAX.get() + 1..)] d: i8) {
        assert_eq!(Draft::saturate(d), Draft::MAX);
    }

    #[proptest]
    fn saturate_caps_if_draft_smaller_than_min(#[strategy(..Draft::MIN.get())] d: i8) {
        assert_eq!(Draft::saturate(d), Draft::MIN);
    }

    #[proptest]
    fn get_returns_raw_draft(d: Draft) {
        assert_eq!(d.get(), d.0);
    }

    #[proptest]
    fn double_negation_is_idempotent(v: Draft) {
        assert_eq!(-(-v), v);
    }

    #[proptest]
    fn addition_is_symmetric(
        a: Draft,
        #[filter((Draft::MIN.get()..=Draft::MAX.get()).contains(&(#a.get() + #b.get())))] b: Draft,
    ) {
        assert_eq!(a + b, b + a);
    }

    #[proptest]
    fn subtraction_is_antisymmetric(
        a: Draft,
        #[filter((Draft::MIN.get()..=Draft::MAX.get()).contains(&(#a.get() - #b.get())))] b: Draft,
    ) {
        assert_eq!(a - b, -(b - a));
    }
}

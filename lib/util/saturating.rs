use derive_more::{Display, Error};
use num_traits::{cast, clamp, Bounded, NumCast, PrimInt};
use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Neg, RangeInclusive, Sub};
use std::{cmp::Ordering, fmt};
use test_strategy::Arbitrary;

/// A saturating numeric type.
#[derive(
    Debug, Display, Default, Copy, Clone, Eq, Ord, Hash, Arbitrary, Serialize, Deserialize,
)]
#[arbitrary(bound(T: fmt::Debug + 'static, RangeInclusive<T>: Strategy<Value = T>))]
#[display(bound = "T: fmt::Display")]
#[serde(into = "i64", try_from = "i64")]
pub struct Saturating<T: PrimInt, const MIN: i64, const MAX: i64>(
    #[strategy(Self::lower().get()..=Self::upper().get())] T,
);

impl<T: PrimInt, const MIN: i64, const MAX: i64> Saturating<T, MIN, MAX> {
    /// Returns the lower bound.
    #[inline]
    pub fn lower() -> Self {
        Saturating(cast(MIN).unwrap())
    }

    /// Returns the upper bound.
    #[inline]
    pub fn upper() -> Self {
        Saturating(cast(MAX).unwrap())
    }

    /// Constructs `Self` from the raw integer.
    ///
    /// # Panics
    ///
    /// Panics if `i` is outside of the bounds.
    #[inline]
    pub fn new(i: T) -> Self {
        assert!((cast(MIN).unwrap()..=cast(MAX).unwrap()).contains(&i));
        Saturating(i)
    }

    /// Returns the raw integer.
    #[inline]
    pub fn get(&self) -> T {
        self.0
    }

    /// Constructs `Self` from a raw integer through saturation.
    #[inline]
    pub fn saturate<U: NumCast + Bounded + PartialOrd>(i: U) -> Self {
        let min = cast(MIN).unwrap_or_else(U::min_value);
        let max = cast(MAX).unwrap_or_else(U::max_value);
        Saturating::new(cast(clamp(i, min, max)).unwrap())
    }

    /// Lossy conversion between `Saturating` integers.
    #[inline]
    pub fn cast<U: PrimInt, const A: i64, const B: i64>(&self) -> Saturating<U, A, B> {
        Saturating::saturate(self.get())
    }
}

impl<T: PrimInt, const MIN: i64, const MAX: i64> From<Saturating<T, MIN, MAX>> for i64 {
    #[inline]
    fn from(s: Saturating<T, MIN, MAX>) -> Self {
        s.get().to_i64().unwrap()
    }
}

/// The reason why converting [`Saturating`] from an integer failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(fmt = "expected integer in the range `({MIN}..={MAX})`")]
pub struct OutOfRange<const MIN: i64, const MAX: i64>;

impl<T: PrimInt, const MIN: i64, const MAX: i64> TryFrom<i64> for Saturating<T, MIN, MAX> {
    type Error = OutOfRange<MIN, MAX>;

    #[inline]
    fn try_from(i: i64) -> Result<Self, Self::Error> {
        if (MIN..=MAX).contains(&i) {
            Ok(Saturating(cast(i).unwrap()))
        } else {
            Err(OutOfRange)
        }
    }
}

impl<T: PrimInt, U: PrimInt, const A: i64, const B: i64, const X: i64, const Y: i64>
    PartialEq<Saturating<U, X, Y>> for Saturating<T, A, B>
{
    #[inline]
    fn eq(&self, other: &Saturating<U, X, Y>) -> bool {
        self.eq(&other.get())
    }
}

impl<T: PrimInt, U: PrimInt, const A: i64, const B: i64> PartialEq<U> for Saturating<T, A, B> {
    #[inline]
    fn eq(&self, other: &U) -> bool {
        i64::eq(&self.get().to_i64().unwrap(), &other.to_i64().unwrap())
    }
}

impl<T: PrimInt, U: PrimInt, const A: i64, const B: i64, const X: i64, const Y: i64>
    PartialOrd<Saturating<U, X, Y>> for Saturating<T, A, B>
{
    #[inline]
    fn partial_cmp(&self, other: &Saturating<U, X, Y>) -> Option<Ordering> {
        self.partial_cmp(&other.get())
    }
}

impl<T: PrimInt, U: PrimInt, const A: i64, const B: i64> PartialOrd<U> for Saturating<T, A, B> {
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<Ordering> {
        i64::partial_cmp(&self.get().to_i64().unwrap(), &other.to_i64().unwrap())
    }
}

impl<T: PrimInt, const A: i64, const B: i64> Neg for Saturating<T, A, B> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Saturating::saturate(T::zero().saturating_sub(self.get()))
    }
}

impl<T: PrimInt, U: PrimInt, const A: i64, const B: i64, const X: i64, const Y: i64>
    Add<Saturating<U, X, Y>> for Saturating<T, A, B>
{
    type Output = Self;

    #[inline]
    fn add(self, rhs: Saturating<U, X, Y>) -> Self::Output {
        self + rhs.get()
    }
}

impl<T: PrimInt, U: PrimInt, const A: i64, const B: i64> Add<U> for Saturating<T, A, B> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: U) -> Self::Output {
        Saturating::saturate(i64::saturating_add(
            self.get().to_i64().unwrap(),
            rhs.to_i64().unwrap(),
        ))
    }
}

impl<T: PrimInt, U: PrimInt, const A: i64, const B: i64, const X: i64, const Y: i64>
    Sub<Saturating<U, X, Y>> for Saturating<T, A, B>
{
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Saturating<U, X, Y>) -> Self::Output {
        self - rhs.get()
    }
}

impl<T: PrimInt, U: PrimInt, const A: i64, const B: i64> Sub<U> for Saturating<T, A, B> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: U) -> Self::Output {
        Saturating::saturate(i64::saturating_sub(
            self.get().to_i64().unwrap(),
            rhs.to_i64().unwrap(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn new_accepts_integers_within_bounds(#[strategy(-5..=9)] i: i32) {
        assert_eq!(Saturating::<i32, -5, 9>::new(i).get(), i);
    }

    #[proptest]
    #[should_panic]
    fn new_panics_if_integer_greater_than_max(#[strategy(10..)] i: i32) {
        Saturating::<i32, -5, 9>::new(i);
    }

    #[proptest]
    #[should_panic]
    fn new_panics_if_integer_smaller_than_min(#[strategy(..-5)] i: i32) {
        Saturating::<i32, -5, 9>::new(i);
    }

    #[proptest]
    fn get_returns_raw_integer(s: Saturating<i32, -5, 9>) {
        assert_eq!(s.get(), s.0);
    }

    #[proptest]
    fn saturate_preserves_integers_within_bounds(#[strategy(-5..=9)] i: i32) {
        assert_eq!(
            Saturating::<i32, -5, 9>::saturate(i),
            Saturating::<i32, -5, 9>::new(i)
        );
    }

    #[proptest]
    fn saturate_caps_if_greater_than_max(#[strategy(10..)] i: i32) {
        assert_eq!(
            Saturating::<i32, -5, 9>::saturate(i),
            Saturating::<i32, -5, 9>::upper()
        );
    }

    #[proptest]
    fn saturate_caps_if_smaller_than_min(#[strategy(..-5)] i: i32) {
        assert_eq!(
            Saturating::<i32, -5, 9>::saturate(i),
            Saturating::<i32, -5, 9>::lower()
        );
    }

    #[proptest]
    fn negation_saturates(s: Saturating<i32, -5, 9>) {
        assert_eq!(-s, Saturating::<i32, -5, 9>::saturate(-s.get()));
    }

    #[proptest]
    fn addition_saturates(a: Saturating<i32, -5, 9>, b: Saturating<i8, -9, 5>) {
        let r = Saturating::<i32, -5, 9>::saturate(a.get() + b.get() as i32);

        assert_eq!(a + b, r);
        assert_eq!(a + b.get(), r);
    }

    #[proptest]
    fn subtraction_saturates(a: Saturating<i32, -5, 9>, b: Saturating<i8, -9, 5>) {
        let r = Saturating::<i32, -5, 9>::saturate(a.get() - b.get() as i32);

        assert_eq!(a - b, r);
        assert_eq!(a - b.get(), r);
    }

    #[proptest]
    fn double_negation_is_idempotent(s: Saturating<i32, -9, 9>) {
        assert_eq!(-(-s), s);
    }

    #[proptest]
    fn addition_is_symmetric(a: Saturating<i32, -9, 9>, b: Saturating<i8, -9, 9>) {
        assert_eq!(a + b, b + a);
    }

    #[proptest]
    fn subtraction_is_antisymmetric(a: Saturating<i32, -9, 9>, b: Saturating<i8, -9, 9>) {
        assert_eq!(a - b, -(b - a));
    }

    #[proptest]
    fn display_is_transparent(s: Saturating<i32, -5, 9>) {
        assert_eq!(s.to_string(), s.get().to_string());
    }

    #[proptest]
    fn serialization_is_transparent(s: Saturating<i32, -5, 9>) {
        assert_eq!(ron::ser::to_string(&s), ron::ser::to_string(&s.get()));
    }

    #[proptest]
    fn deserializing_succeeds_if_within_bounds(s: Saturating<i32, -5, 9>) {
        assert_eq!(ron::de::from_str(&s.to_string()), Ok(s));
    }

    #[proptest]
    fn deserializing_fails_if_greater_than_max(#[strategy(10..)] i: i32) {
        assert!(ron::de::from_str::<Saturating<i32, -5, 9>>(&i.to_string()).is_err());
    }

    #[proptest]
    fn deserializing_fails_if_smaller_than_max(#[strategy(..-5)] i: i32) {
        assert!(ron::de::from_str::<Saturating<i32, -5, 9>>(&i.to_string()).is_err());
    }
}

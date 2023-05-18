use super::Bounds;
use derive_more::{DebugCustom, Display, Error};
use num_traits::{cast, clamp, PrimInt};
use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::ops::{Add, Div, Mul, Neg, RangeInclusive, Sub};
use std::{cmp::Ordering, fmt, marker::PhantomData};
use test_strategy::Arbitrary;

/// A saturating bounded integer.
#[derive(DebugCustom, Display, Arbitrary, Serialize, Deserialize)]
#[arbitrary(bound(T: fmt::Debug + 'static, X: 'static, RangeInclusive<T>: Strategy<Value = T>))]
#[debug(bound = "T: fmt::Debug")]
#[debug(fmt = "Saturating({_0:?})")]
#[display(bound = "T: fmt::Display")]
#[display(fmt = "{_0}")]
#[serde(into = "i64", try_from = "i64")]
pub struct Saturating<T: PrimInt, X: Bounds<T>>(
    #[strategy(X::LOWER..=X::UPPER)]
    #[serde(bound = "T: Into<i64>")]
    T,
    PhantomData<X>,
);

impl<T: PrimInt, X: Bounds<T>> Saturating<T, X> {
    /// Returns the lower bound.
    #[inline]
    pub fn lower() -> Self {
        Saturating(X::LOWER, PhantomData)
    }

    /// Returns the upper bound.
    #[inline]
    pub fn upper() -> Self {
        Saturating(X::UPPER, PhantomData)
    }

    /// Constructs `Self` from the raw integer.
    ///
    /// # Panics
    ///
    /// Panics if `i` is outside of the bounds.
    #[inline]
    pub fn new(i: T) -> Self {
        assert!((X::LOWER..=X::UPPER).contains(&i));
        Saturating(i, PhantomData)
    }

    /// Returns the raw integer.
    #[inline]
    pub fn get(&self) -> T {
        self.0
    }

    /// Constructs `Self` from a raw integer through saturation.
    #[inline]
    pub fn saturate<U: PrimInt>(i: U) -> Self {
        let min = cast(X::LOWER).unwrap_or_else(U::min_value);
        let max = cast(X::UPPER).unwrap_or_else(U::max_value);
        Saturating::new(cast(clamp(i, min, max)).unwrap())
    }

    /// Lossy conversion between saturating integers.
    #[inline]
    pub fn cast<U: PrimInt, Y: Bounds<U>>(&self) -> Saturating<U, Y> {
        Saturating::saturate(self.get())
    }
}

impl<T: PrimInt + Default, X: Bounds<T>> Default for Saturating<T, X> {
    #[inline]
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

impl<T: PrimInt, X: Bounds<T>> Copy for Saturating<T, X> {}

impl<T: PrimInt, X: Bounds<T>> Clone for Saturating<T, X> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}

impl<T: PrimInt + Hash, X: Bounds<T>> Hash for Saturating<T, X> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: PrimInt + Into<i64>, X: Bounds<T>> From<Saturating<T, X>> for i64 {
    #[inline]
    fn from(s: Saturating<T, X>) -> Self {
        s.get().into()
    }
}

/// The reason why converting [`Saturating`] from an integer failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(fmt = "expected integer in the range `({_0}..={_1})`")]
pub struct OutOfRange(i64, i64);

impl<T, X> TryFrom<i64> for Saturating<T, X>
where
    T: PrimInt + Into<i64>,
    X: Bounds<T>,
{
    type Error = OutOfRange;

    fn try_from(i: i64) -> Result<Self, Self::Error> {
        if (X::LOWER.into()..=X::UPPER.into()).contains(&i) {
            Ok(Saturating::saturate(i))
        } else {
            Err(OutOfRange(X::LOWER.into(), X::UPPER.into()))
        }
    }
}

impl<T: PrimInt + Into<i64>, X: Bounds<T>> Eq for Saturating<T, X> {}

impl<T: PrimInt + Into<i64>, X: Bounds<T>, U: PrimInt + Into<i64>, Y: Bounds<U>>
    PartialEq<Saturating<U, Y>> for Saturating<T, X>
{
    #[inline]
    fn eq(&self, other: &Saturating<U, Y>) -> bool {
        self.eq(&other.get())
    }
}

impl<T: PrimInt + Into<i64>, X: Bounds<T>, U: PrimInt + Into<i64>> PartialEq<U>
    for Saturating<T, X>
{
    #[inline]
    fn eq(&self, &other: &U) -> bool {
        i64::eq(&self.get().into(), &other.into())
    }
}

impl<T: PrimInt + Into<i64>, X: Bounds<T>> Ord for Saturating<T, X> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T: PrimInt + Into<i64>, X: Bounds<T>, U: PrimInt + Into<i64>, Y: Bounds<U>>
    PartialOrd<Saturating<U, Y>> for Saturating<T, X>
{
    #[inline]
    fn partial_cmp(&self, other: &Saturating<U, Y>) -> Option<Ordering> {
        self.partial_cmp(&other.get())
    }
}

impl<T: PrimInt + Into<i64>, X: Bounds<T>, U: PrimInt + Into<i64>> PartialOrd<U>
    for Saturating<T, X>
{
    #[inline]
    fn partial_cmp(&self, &other: &U) -> Option<Ordering> {
        i64::partial_cmp(&self.get().into(), &other.into())
    }
}

impl<T: PrimInt + Into<i64>, X: Bounds<T>> Neg for Saturating<T, X> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Saturating::saturate(i64::saturating_neg(self.get().into()))
    }
}

impl<T: PrimInt + Into<i64>, X: Bounds<T>, U: PrimInt + Into<i64>, Y: Bounds<U>>
    Add<Saturating<U, Y>> for Saturating<T, X>
{
    type Output = Self;

    #[inline]
    fn add(self, rhs: Saturating<U, Y>) -> Self::Output {
        self + rhs.get()
    }
}

impl<T: PrimInt + Into<i64>, X: Bounds<T>, U: PrimInt + Into<i64>> Add<U> for Saturating<T, X> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: U) -> Self::Output {
        Saturating::saturate(i64::saturating_add(self.get().into(), rhs.into()))
    }
}

impl<T: PrimInt + Into<i64>, X: Bounds<T>, U: PrimInt + Into<i64>, Y: Bounds<U>>
    Sub<Saturating<U, Y>> for Saturating<T, X>
{
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Saturating<U, Y>) -> Self::Output {
        self - rhs.get()
    }
}

impl<T: PrimInt + Into<i64>, X: Bounds<T>, U: PrimInt + Into<i64>> Sub<U> for Saturating<T, X> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: U) -> Self::Output {
        Saturating::saturate(i64::saturating_sub(self.get().into(), rhs.into()))
    }
}

impl<T: PrimInt + Into<i64>, B: Bounds<T>, U: PrimInt + Into<i64>, C: Bounds<U>>
    Mul<Saturating<U, C>> for Saturating<T, B>
{
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Saturating<U, C>) -> Self::Output {
        self * rhs.get()
    }
}

impl<T: PrimInt + Into<i64>, B: Bounds<T>, U: PrimInt + Into<i64>> Mul<U> for Saturating<T, B> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: U) -> Self::Output {
        Saturating::saturate(i64::saturating_mul(self.get().into(), rhs.into()))
    }
}

impl<T: PrimInt + Into<i64>, B: Bounds<T>, U: PrimInt + Into<i64>, C: Bounds<U>>
    Div<Saturating<U, C>> for Saturating<T, B>
{
    type Output = Self;

    #[inline]
    fn div(self, rhs: Saturating<U, C>) -> Self::Output {
        self / rhs.get()
    }
}

impl<T: PrimInt + Into<i64>, B: Bounds<T>, U: PrimInt + Into<i64>> Div<U> for Saturating<T, B> {
    type Output = Self;

    #[inline]
    fn div(self, rhs: U) -> Self::Output {
        Saturating::saturate(i64::saturating_div(self.get().into(), rhs.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    struct AsymmetricBounds;

    impl Bounds<i8> for AsymmetricBounds {
        const LOWER: i8 = -5;
        const UPPER: i8 = 9;
    }

    #[proptest]
    fn new_accepts_integers_within_bounds(
        #[strategy(AsymmetricBounds::LOWER..=AsymmetricBounds::UPPER)] i: i8,
    ) {
        assert_eq!(Saturating::<i8, AsymmetricBounds>::new(i).get(), i);
    }

    #[proptest]
    #[should_panic]
    fn new_panics_if_integer_greater_than_max(#[strategy(AsymmetricBounds::UPPER + 1..)] i: i8) {
        Saturating::<i8, AsymmetricBounds>::new(i);
    }

    #[proptest]
    #[should_panic]
    fn new_panics_if_integer_smaller_than_min(#[strategy(..AsymmetricBounds::LOWER)] i: i8) {
        Saturating::<i8, AsymmetricBounds>::new(i);
    }

    #[proptest]
    fn get_returns_raw_integer(s: Saturating<i8, AsymmetricBounds>) {
        assert_eq!(s.get(), s.0);
    }

    #[proptest]
    fn saturate_preserves_integers_within_bounds(
        #[strategy(AsymmetricBounds::LOWER..=AsymmetricBounds::UPPER)] i: i8,
    ) {
        assert_eq!(
            Saturating::<i8, AsymmetricBounds>::saturate(i),
            Saturating::<i8, AsymmetricBounds>::new(i)
        );
    }

    #[proptest]
    fn saturate_caps_if_greater_than_max(#[strategy(AsymmetricBounds::UPPER + 1..)] i: i8) {
        assert_eq!(
            Saturating::<i8, AsymmetricBounds>::saturate(i),
            Saturating::<i8, AsymmetricBounds>::upper()
        );
    }

    #[proptest]
    fn saturate_caps_if_smaller_than_min(#[strategy(..AsymmetricBounds::LOWER)] i: i8) {
        assert_eq!(
            Saturating::<i8, AsymmetricBounds>::saturate(i),
            Saturating::<i8, AsymmetricBounds>::lower()
        );
    }

    #[proptest]
    fn negation_saturates(s: Saturating<i8, AsymmetricBounds>) {
        assert_eq!(-s, Saturating::<i8, AsymmetricBounds>::saturate(-s.get()));
    }

    #[proptest]
    fn addition_saturates(
        a: Saturating<i8, AsymmetricBounds>,
        b: Saturating<i8, AsymmetricBounds>,
    ) {
        let r = Saturating::<i8, AsymmetricBounds>::saturate(a.get() + b.get());

        assert_eq!(a + b, r);
        assert_eq!(a + b.get(), r);
    }

    #[proptest]
    fn subtraction_saturates(
        a: Saturating<i8, AsymmetricBounds>,
        b: Saturating<i8, AsymmetricBounds>,
    ) {
        let r = Saturating::<i8, AsymmetricBounds>::saturate(a.get() - b.get());

        assert_eq!(a - b, r);
        assert_eq!(a - b.get(), r);
    }

    #[proptest]
    fn multiplication_saturates(
        a: Saturating<i8, AsymmetricBounds>,
        b: Saturating<i8, AsymmetricBounds>,
    ) {
        let r = Saturating::<i8, AsymmetricBounds>::saturate(a.get() * b.get());

        assert_eq!(a * b, r);
        assert_eq!(a * b.get(), r);
    }

    #[proptest]
    fn division_saturates(
        a: Saturating<i8, AsymmetricBounds>,
        #[filter(#b != 0)] b: Saturating<i8, AsymmetricBounds>,
    ) {
        let r = Saturating::<i8, AsymmetricBounds>::saturate(a.get() / b.get());

        assert_eq!(a / b, r);
        assert_eq!(a / b.get(), r);
    }

    #[proptest]
    fn display_is_transparent(s: Saturating<i8, AsymmetricBounds>) {
        assert_eq!(s.to_string(), s.get().to_string());
    }

    #[proptest]
    fn serialization_is_transparent(s: Saturating<i8, AsymmetricBounds>) {
        assert_eq!(ron::ser::to_string(&s), ron::ser::to_string(&s.get()));
    }

    #[proptest]
    fn deserializing_succeeds_if_within_bounds(s: Saturating<i8, AsymmetricBounds>) {
        assert_eq!(ron::de::from_str(&s.to_string()), Ok(s));
    }

    #[proptest]
    fn deserializing_fails_if_greater_than_max(#[strategy(AsymmetricBounds::UPPER + 1..)] i: i8) {
        assert!(ron::de::from_str::<Saturating<i8, AsymmetricBounds>>(&i.to_string()).is_err());
    }

    #[proptest]
    fn deserializing_fails_if_smaller_than_max(#[strategy(..AsymmetricBounds::LOWER)] i: i8) {
        assert!(ron::de::from_str::<Saturating<i8, AsymmetricBounds>>(&i.to_string()).is_err());
    }
}

use crate::util::Bounds;
use derive_more::{DebugCustom, Display, Error};
use num_traits::{cast, clamp, AsPrimitive, PrimInt};
use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::ops::{Add, Div, Mul, Neg, RangeInclusive, Sub};
use std::{cmp::Ordering, fmt::Debug, marker::PhantomData};
use test_strategy::Arbitrary;

/// A saturating bounded integer.
#[derive(DebugCustom, Arbitrary, Serialize, Deserialize)]
#[arbitrary(bound(T::Integer: 'static + Debug, RangeInclusive<T::Integer>: Strategy<Value = T::Integer>))]
#[debug(fmt = "Saturating({:?})", "i32::from(*self)")]
#[serde(into = "i32", try_from = "i32")]
pub struct Saturating<T: Bounds>(#[strategy(T::LOWER..=T::UPPER)] T::Integer);

impl<T: Bounds> Saturating<T> {
    /// Returns the lower bound.
    pub const fn lower() -> Self {
        Saturating(T::LOWER)
    }

    /// Returns the upper bound.
    pub const fn upper() -> Self {
        Saturating(T::UPPER)
    }

    /// Constructs `Self` from the raw integer.
    ///
    /// # Panics
    ///
    /// Panics if `i` is outside of the bounds.
    pub fn new(i: T::Integer) -> Self {
        assert!((T::LOWER..=T::UPPER).contains(&i));
        Saturating(i)
    }

    /// Returns the raw integer.
    pub fn get(&self) -> T::Integer {
        self.0
    }

    /// Constructs `Self` from a raw integer through saturation.
    pub fn saturate<U: PrimInt>(i: U) -> Self {
        let min = cast(T::LOWER).unwrap_or_else(U::min_value);
        let max = cast(T::UPPER).unwrap_or_else(U::max_value);
        Saturating::new(cast(clamp(i, min, max)).unwrap())
    }

    /// Lossy conversion between saturating integers.
    pub fn cast<U: Bounds>(&self) -> Saturating<U> {
        Saturating::saturate(self.get())
    }
}

impl<T: Bounds> Copy for Saturating<T> {}

impl<T: Bounds> Clone for Saturating<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Bounds> Hash for Saturating<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_i32(self.get().as_());
    }
}

impl<T: Bounds> From<Saturating<T>> for i32 {
    fn from(s: Saturating<T>) -> Self {
        s.get().into()
    }
}

/// The reason why converting [`Saturating`] from an integer failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(
    fmt = "expected integer in the range `({}..={})`",
    "T::LOWER.into()",
    "T::UPPER.into()"
)]
pub struct OutOfRange<T>(PhantomData<T>)
where
    T: Bounds;

impl<T: Bounds> TryFrom<i32> for Saturating<T> {
    type Error = OutOfRange<T>;

    fn try_from(i: i32) -> Result<Self, Self::Error> {
        if (T::LOWER.into()..=T::UPPER.into()).contains(&i) {
            Ok(Saturating::saturate(i))
        } else {
            Err(OutOfRange(PhantomData))
        }
    }
}

impl<T: Bounds> Eq for Saturating<T> {}

impl<T: Bounds, U: Bounds> PartialEq<Saturating<U>> for Saturating<T> {
    fn eq(&self, other: &Saturating<U>) -> bool {
        self.eq(&other.get())
    }
}

impl<T: Bounds, U: PrimInt + Into<i32> + AsPrimitive<i32>> PartialEq<U> for Saturating<T> {
    fn eq(&self, other: &U) -> bool {
        i32::eq(&self.get().as_(), &other.as_())
    }
}

impl<T: Bounds> Ord for Saturating<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T: Bounds, U: Bounds> PartialOrd<Saturating<U>> for Saturating<T> {
    fn partial_cmp(&self, other: &Saturating<U>) -> Option<Ordering> {
        self.partial_cmp(&other.get())
    }
}

impl<T: Bounds, U: PrimInt + Into<i32> + AsPrimitive<i32>> PartialOrd<U> for Saturating<T> {
    fn partial_cmp(&self, other: &U) -> Option<Ordering> {
        i32::partial_cmp(&self.get().as_(), &other.as_())
    }
}

impl<T: Bounds> Neg for Saturating<T> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Saturating::saturate(i32::saturating_neg(self.get().as_()))
    }
}

impl<T: Bounds, U: Bounds> Add<Saturating<U>> for Saturating<T> {
    type Output = Self;

    fn add(self, rhs: Saturating<U>) -> Self::Output {
        self + rhs.get()
    }
}

impl<T: Bounds, U: PrimInt + Into<i32> + AsPrimitive<i32>> Add<U> for Saturating<T> {
    type Output = Self;

    fn add(self, rhs: U) -> Self::Output {
        Saturating::saturate(i32::saturating_add(self.get().as_(), rhs.as_()))
    }
}

impl<T: Bounds, U: Bounds> Sub<Saturating<U>> for Saturating<T> {
    type Output = Self;

    fn sub(self, rhs: Saturating<U>) -> Self::Output {
        self - rhs.get()
    }
}

impl<T: Bounds, U: PrimInt + Into<i32> + AsPrimitive<i32>> Sub<U> for Saturating<T> {
    type Output = Self;

    fn sub(self, rhs: U) -> Self::Output {
        Saturating::saturate(i32::saturating_sub(self.get().as_(), rhs.as_()))
    }
}

impl<T: Bounds, U: Bounds> Mul<Saturating<U>> for Saturating<T> {
    type Output = Self;

    fn mul(self, rhs: Saturating<U>) -> Self::Output {
        self * rhs.get()
    }
}

impl<T: Bounds, U: PrimInt + Into<i32> + AsPrimitive<i32>> Mul<U> for Saturating<T> {
    type Output = Self;

    fn mul(self, rhs: U) -> Self::Output {
        Saturating::saturate(i32::saturating_mul(self.get().as_(), rhs.as_()))
    }
}

impl<T: Bounds, U: Bounds> Div<Saturating<U>> for Saturating<T> {
    type Output = Self;

    fn div(self, rhs: Saturating<U>) -> Self::Output {
        self / rhs.get()
    }
}

impl<T: Bounds, U: PrimInt + Into<i32> + AsPrimitive<i32>> Div<U> for Saturating<T> {
    type Output = Self;

    fn div(self, rhs: U) -> Self::Output {
        Saturating::saturate(i32::saturating_div(self.get().as_(), rhs.as_()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    struct AsymmetricBounds;

    impl Bounds for AsymmetricBounds {
        type Integer = i8;
        const LOWER: Self::Integer = -5;
        const UPPER: Self::Integer = 9;
    }

    #[proptest]
    fn new_accepts_integers_within_bounds(
        #[strategy(AsymmetricBounds::LOWER..=AsymmetricBounds::UPPER)] i: i8,
    ) {
        assert_eq!(Saturating::<AsymmetricBounds>::new(i).get(), i);
    }

    #[proptest]
    #[should_panic]
    fn new_panics_if_integer_greater_than_max(#[strategy(AsymmetricBounds::UPPER + 1..)] i: i8) {
        Saturating::<AsymmetricBounds>::new(i);
    }

    #[proptest]
    #[should_panic]
    fn new_panics_if_integer_smaller_than_min(#[strategy(..AsymmetricBounds::LOWER)] i: i8) {
        Saturating::<AsymmetricBounds>::new(i);
    }

    #[proptest]
    fn get_returns_raw_integer(s: Saturating<AsymmetricBounds>) {
        assert_eq!(s.get(), s.0);
    }

    #[proptest]
    fn saturate_preserves_integers_within_bounds(
        #[strategy(AsymmetricBounds::LOWER..=AsymmetricBounds::UPPER)] i: i8,
    ) {
        assert_eq!(
            Saturating::<AsymmetricBounds>::saturate(i),
            Saturating::<AsymmetricBounds>::new(i)
        );
    }

    #[proptest]
    fn saturate_caps_if_greater_than_max(#[strategy(AsymmetricBounds::UPPER + 1..)] i: i8) {
        assert_eq!(
            Saturating::<AsymmetricBounds>::saturate(i),
            Saturating::<AsymmetricBounds>::upper()
        );
    }

    #[proptest]
    fn saturate_caps_if_smaller_than_min(#[strategy(..AsymmetricBounds::LOWER)] i: i8) {
        assert_eq!(
            Saturating::<AsymmetricBounds>::saturate(i),
            Saturating::<AsymmetricBounds>::lower()
        );
    }

    #[proptest]
    fn negation_saturates(s: Saturating<AsymmetricBounds>) {
        assert_eq!(-s, Saturating::<AsymmetricBounds>::saturate(-s.get()));
    }

    #[proptest]
    fn addition_saturates(a: Saturating<AsymmetricBounds>, b: Saturating<AsymmetricBounds>) {
        let r = Saturating::<AsymmetricBounds>::saturate(a.get() + b.get());

        assert_eq!(a + b, r);
        assert_eq!(a + b.get(), r);
    }

    #[proptest]
    fn subtraction_saturates(a: Saturating<AsymmetricBounds>, b: Saturating<AsymmetricBounds>) {
        let r = Saturating::<AsymmetricBounds>::saturate(a.get() - b.get());

        assert_eq!(a - b, r);
        assert_eq!(a - b.get(), r);
    }

    #[proptest]
    fn multiplication_saturates(a: Saturating<AsymmetricBounds>, b: Saturating<AsymmetricBounds>) {
        let r = Saturating::<AsymmetricBounds>::saturate(a.get() * b.get());

        assert_eq!(a * b, r);
        assert_eq!(a * b.get(), r);
    }

    #[proptest]
    fn division_saturates(
        a: Saturating<AsymmetricBounds>,
        #[filter(#b != 0)] b: Saturating<AsymmetricBounds>,
    ) {
        let r = Saturating::<AsymmetricBounds>::saturate(a.get() / b.get());

        assert_eq!(a / b, r);
        assert_eq!(a / b.get(), r);
    }

    #[proptest]
    fn serialization_is_transparent(s: Saturating<AsymmetricBounds>) {
        assert_eq!(ron::ser::to_string(&s), ron::ser::to_string(&s.get()));
    }

    #[proptest]
    fn deserializing_succeeds_if_within_bounds(s: Saturating<AsymmetricBounds>) {
        assert_eq!(ron::de::from_str(&s.get().to_string()), Ok(s));
    }

    #[proptest]
    fn deserializing_fails_if_greater_than_max(#[strategy(AsymmetricBounds::UPPER + 1..)] i: i8) {
        assert!(ron::de::from_str::<Saturating<AsymmetricBounds>>(&i.to_string()).is_err());
    }

    #[proptest]
    fn deserializing_fails_if_smaller_than_max(#[strategy(..AsymmetricBounds::LOWER)] i: i8) {
        assert!(ron::de::from_str::<Saturating<AsymmetricBounds>>(&i.to_string()).is_err());
    }
}

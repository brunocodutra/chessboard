use crate::util::{Assume, Bounds};
use derive_more::Debug;
use num_traits::{cast, clamp, AsPrimitive, PrimInt, Signed};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Div, Mul, Neg, Sub};

#[cfg(test)]
use proptest::prelude::*;

#[cfg(test)]
use std::ops::RangeInclusive;

trait Larger {
    type Integer: PrimInt + Signed;
}

impl<T: PrimInt + Signed> Larger for (T, T) {
    type Integer = T;
}

impl Larger for (i8, i16) {
    type Integer = i16;
}

impl Larger for (i16, i8) {
    type Integer = i16;
}

impl Larger for (i8, i32) {
    type Integer = i32;
}

impl Larger for (i32, i8) {
    type Integer = i32;
}

impl Larger for (i8, i64) {
    type Integer = i64;
}

impl Larger for (i64, i8) {
    type Integer = i64;
}

impl Larger for (i16, i32) {
    type Integer = i32;
}

impl Larger for (i32, i16) {
    type Integer = i32;
}

impl Larger for (i16, i64) {
    type Integer = i64;
}

impl Larger for (i64, i16) {
    type Integer = i64;
}

impl Larger for (i32, i64) {
    type Integer = i64;
}

impl Larger for (i64, i32) {
    type Integer = i64;
}

trait NextLarger {
    type Integer: PrimInt + Signed;
}

impl<I, J, K: NextLarger> NextLarger for (I, J)
where
    Self: Larger<Integer = K>,
{
    type Integer = K::Integer;
}

impl NextLarger for i8 {
    type Integer = i16;
}

impl NextLarger for i16 {
    type Integer = i32;
}

impl NextLarger for i32 {
    type Integer = i64;
}

/// A saturating bounded integer.
#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(bound(T::Integer: 'static + Debug, RangeInclusive<T::Integer>: Strategy<Value = T::Integer>)))]
#[debug("Saturating({_0:?})")]
#[repr(transparent)]
pub struct Saturating<T: Bounds>(#[cfg_attr(test, strategy(T::LOWER..=T::UPPER))] T::Integer);

impl<T: Bounds> Saturating<T> {
    /// The lower bound.
    pub const LOWER: Self = Saturating(T::LOWER);

    /// The upper bound.
    pub const UPPER: Self = Saturating(T::UPPER);

    /// Constructs `Self` from the raw integer.
    ///
    /// # Panics
    ///
    /// Panics if `i` is outside of the bounds.
    #[inline(always)]
    pub fn new(i: T::Integer) -> Self {
        assert!((T::LOWER..=T::UPPER).contains(&i));
        Saturating(i)
    }

    /// Returns the raw integer.
    #[inline(always)]
    pub const fn get(&self) -> T::Integer {
        self.0
    }

    /// Constructs `Self` from a raw integer through saturation.
    #[inline(always)]
    pub fn saturate<I: PrimInt>(i: I) -> Self {
        let min = cast(T::LOWER).unwrap_or_else(I::min_value);
        let max = cast(T::UPPER).unwrap_or_else(I::max_value);
        Saturating::new(cast(clamp(i, min, max)).assume())
    }

    /// Lossy conversion between saturating integers.
    #[inline(always)]
    pub fn cast<U: Bounds>(&self) -> Saturating<U> {
        Saturating::saturate(self.get())
    }
}

impl<T: Bounds> Copy for Saturating<T> {}

impl<T: Bounds> Clone for Saturating<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Bounds> Eq for Saturating<T> where Self: PartialEq<Self> {}

impl<T: Bounds, U: Bounds> PartialEq<Saturating<U>> for Saturating<T>
where
    Self: PartialEq<U::Integer>,
{
    #[inline(always)]
    fn eq(&self, other: &Saturating<U>) -> bool {
        self.eq(&other.get())
    }
}

impl<T: Bounds> Ord for Saturating<T>
where
    Self: PartialEq<Self> + PartialOrd,
{
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T: Bounds, U: Bounds> PartialOrd<Saturating<U>> for Saturating<T>
where
    Self: PartialOrd<U::Integer>,
{
    #[inline(always)]
    fn partial_cmp(&self, other: &Saturating<U>) -> Option<Ordering> {
        self.partial_cmp(&other.get())
    }
}

impl<T: Bounds<Integer = I>, I: Hash> Hash for Saturating<T> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.get().hash(state)
    }
}

impl<T: Bounds<Integer = I>, U: Bounds<Integer = J>, I, J, K> Add<Saturating<U>> for Saturating<T>
where
    I: AsPrimitive<K>,
    J: AsPrimitive<K>,
    K: 'static + PrimInt + Signed,
    (I, J): NextLarger<Integer = K>,
{
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Saturating<U>) -> Self::Output {
        self + rhs.get()
    }
}

impl<T: Bounds<Integer = I>, U: Bounds<Integer = J>, I, J, K> Sub<Saturating<U>> for Saturating<T>
where
    I: AsPrimitive<K>,
    J: AsPrimitive<K>,
    K: 'static + PrimInt + Signed,
    (I, J): NextLarger<Integer = K>,
{
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Saturating<U>) -> Self::Output {
        self - rhs.get()
    }
}

impl<T: Bounds<Integer = I>, U: Bounds<Integer = J>, I, J, K> Mul<Saturating<U>> for Saturating<T>
where
    I: AsPrimitive<K>,
    J: AsPrimitive<K>,
    K: 'static + PrimInt + Signed,
    (I, J): NextLarger<Integer = K>,
{
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Saturating<U>) -> Self::Output {
        self * rhs.get()
    }
}

impl<T: Bounds<Integer = I>, U: Bounds<Integer = J>, I, J, K> Div<Saturating<U>> for Saturating<T>
where
    I: AsPrimitive<K>,
    J: AsPrimitive<K>,
    K: 'static + PrimInt + Signed,
    (I, J): NextLarger<Integer = K>,
{
    type Output = Self;

    #[inline(always)]
    fn div(self, rhs: Saturating<U>) -> Self::Output {
        self / rhs.get()
    }
}

impl<T: Bounds<Integer = I>, I, J, K> PartialEq<J> for Saturating<T>
where
    I: AsPrimitive<K>,
    J: AsPrimitive<K>,
    K: 'static + PrimInt,
    (I, J): Larger<Integer = K>,
{
    #[inline(always)]
    fn eq(&self, other: &J) -> bool {
        K::eq(&self.get().as_(), &other.as_())
    }
}

impl<T: Bounds<Integer = I>, I, J, K> PartialOrd<J> for Saturating<T>
where
    I: AsPrimitive<K>,
    J: AsPrimitive<K>,
    K: 'static + PrimInt,
    (I, J): Larger<Integer = K>,
{
    #[inline(always)]
    fn partial_cmp(&self, other: &J) -> Option<Ordering> {
        K::partial_cmp(&self.get().as_(), &other.as_())
    }
}

impl<T: Bounds<Integer = I>, I, J> Neg for Saturating<T>
where
    I: AsPrimitive<J> + NextLarger<Integer = J>,
    J: 'static + PrimInt + Signed,
{
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self::Output {
        Saturating::saturate(J::neg(self.get().as_()))
    }
}

impl<T: Bounds<Integer = I>, I, J, K> Add<J> for Saturating<T>
where
    I: AsPrimitive<K>,
    J: AsPrimitive<K>,
    K: 'static + PrimInt + Signed,
    (I, J): NextLarger<Integer = K>,
{
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: J) -> Self::Output {
        Saturating::saturate(K::add(self.get().as_(), rhs.as_()))
    }
}

impl<T: Bounds<Integer = I>, I, J, K> Sub<J> for Saturating<T>
where
    I: AsPrimitive<K>,
    J: AsPrimitive<K>,
    K: 'static + PrimInt + Signed,
    (I, J): NextLarger<Integer = K>,
{
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: J) -> Self::Output {
        Saturating::saturate(K::sub(self.get().as_(), rhs.as_()))
    }
}

impl<T: Bounds<Integer = I>, I, J, K> Mul<J> for Saturating<T>
where
    I: AsPrimitive<K>,
    J: AsPrimitive<K>,
    K: 'static + PrimInt + Signed,
    (I, J): NextLarger<Integer = K>,
{
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: J) -> Self::Output {
        Saturating::saturate(K::mul(self.get().as_(), rhs.as_()))
    }
}

impl<T: Bounds<Integer = I>, I, J, K> Div<J> for Saturating<T>
where
    I: AsPrimitive<K>,
    J: AsPrimitive<K>,
    K: 'static + PrimInt + Signed,
    (I, J): NextLarger<Integer = K>,
{
    type Output = Self;

    #[inline(always)]
    fn div(self, rhs: J) -> Self::Output {
        Saturating::saturate(K::div(self.get().as_(), rhs.as_()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;
    use test_strategy::proptest;

    struct AsymmetricBounds;

    impl Bounds for AsymmetricBounds {
        type Integer = i8;
        const LOWER: Self::Integer = -89;
        const UPPER: Self::Integer = 111;
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
            Saturating::<AsymmetricBounds>::UPPER
        );
    }

    #[proptest]
    fn saturate_caps_if_smaller_than_min(#[strategy(..AsymmetricBounds::LOWER)] i: i8) {
        assert_eq!(
            Saturating::<AsymmetricBounds>::saturate(i),
            Saturating::<AsymmetricBounds>::LOWER
        );
    }

    #[proptest]
    fn negation_saturates(s: Saturating<AsymmetricBounds>) {
        let r = Saturating::<AsymmetricBounds>::saturate(s.get().saturating_neg());

        assert_eq!(-s, r);
    }

    #[proptest]
    fn addition_saturates(a: Saturating<AsymmetricBounds>, b: Saturating<AsymmetricBounds>) {
        let r = Saturating::<AsymmetricBounds>::saturate(i8::saturating_add(a.get(), b.get()));

        assert_eq!(a + b, r);
        assert_eq!(a + b.get(), r);
    }

    #[proptest]
    fn subtraction_saturates(a: Saturating<AsymmetricBounds>, b: Saturating<AsymmetricBounds>) {
        let r = Saturating::<AsymmetricBounds>::saturate(i8::saturating_sub(a.get(), b.get()));

        assert_eq!(a - b, r);
        assert_eq!(a - b.get(), r);
    }

    #[proptest]
    fn multiplication_saturates(a: Saturating<AsymmetricBounds>, b: Saturating<AsymmetricBounds>) {
        let r = Saturating::<AsymmetricBounds>::saturate(i8::saturating_mul(a.get(), b.get()));

        assert_eq!(a * b, r);
        assert_eq!(a * b.get(), r);
    }

    #[proptest]
    fn division_saturates(
        a: Saturating<AsymmetricBounds>,
        #[filter(#b != 0)] b: Saturating<AsymmetricBounds>,
    ) {
        let r = Saturating::<AsymmetricBounds>::saturate(i8::saturating_div(a.get(), b.get()));

        assert_eq!(a / b, r);
        assert_eq!(a / b.get(), r);
    }
}

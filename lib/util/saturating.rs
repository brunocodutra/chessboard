use crate::util::{Assume, Integer};
use num_traits::{cast, clamp, AsPrimitive, PrimInt, Signed};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Div, Mul, Neg, Sub};

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
#[derive(Debug, Default, Copy, Clone)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(transparent)]
pub struct Saturating<T: Integer>(T);

impl<T: Integer> Saturating<T> {
    /// Constructs `Self` from the raw integer.
    #[inline(always)]
    pub fn new(i: T::Repr) -> Self {
        Self::from_repr(i)
    }

    /// Returns the raw integer.
    #[inline(always)]
    pub fn get(&self) -> T::Repr {
        self.repr()
    }

    /// Constructs `Self` from a raw integer through saturation.
    #[inline(always)]
    pub fn saturate<I: PrimInt>(i: I) -> Self {
        let min = cast(T::MIN).unwrap_or_else(I::min_value);
        let max = cast(T::MAX).unwrap_or_else(I::max_value);
        Saturating::new(cast(clamp(i, min, max)).assume())
    }

    /// Lossy conversion between saturating integers.
    #[inline(always)]
    pub fn cast<U: Integer>(&self) -> Saturating<U> {
        Saturating::saturate(self.get())
    }
}

unsafe impl<T: Integer> Integer for Saturating<T> {
    type Repr = T::Repr;

    const MIN: Self::Repr = T::MIN;
    const MAX: Self::Repr = T::MAX;

    #[inline(always)]
    fn repr(&self) -> Self::Repr {
        self.0.repr()
    }
}

impl<T: Integer> Eq for Saturating<T> where Self: PartialEq<Self> {}

impl<T: Integer, U: Integer> PartialEq<Saturating<U>> for Saturating<T>
where
    Self: PartialEq<U::Repr>,
{
    #[inline(always)]
    fn eq(&self, other: &Saturating<U>) -> bool {
        self.eq(&other.get())
    }
}

impl<T: Integer> Ord for Saturating<T>
where
    Self: PartialEq<Self> + PartialOrd,
{
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T: Integer, U: Integer> PartialOrd<Saturating<U>> for Saturating<T>
where
    Self: PartialOrd<U::Repr>,
{
    #[inline(always)]
    fn partial_cmp(&self, other: &Saturating<U>) -> Option<Ordering> {
        self.partial_cmp(&other.get())
    }
}

impl<T: Integer<Repr = I>, I: Hash> Hash for Saturating<T> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.get().hash(state)
    }
}

impl<T: Integer<Repr = I>, U: Integer<Repr = J>, I, J, K> Add<Saturating<U>> for Saturating<T>
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

impl<T: Integer<Repr = I>, U: Integer<Repr = J>, I, J, K> Sub<Saturating<U>> for Saturating<T>
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

impl<T: Integer<Repr = I>, U: Integer<Repr = J>, I, J, K> Mul<Saturating<U>> for Saturating<T>
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

impl<T: Integer<Repr = I>, U: Integer<Repr = J>, I, J, K> Div<Saturating<U>> for Saturating<T>
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

impl<T: Integer<Repr = I>, I, J, K> PartialEq<J> for Saturating<T>
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

impl<T: Integer<Repr = I>, I, J, K> PartialOrd<J> for Saturating<T>
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

impl<T: Integer<Repr = I>, I, J> Neg for Saturating<T>
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

impl<T: Integer<Repr = I>, I, J, K> Add<J> for Saturating<T>
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

impl<T: Integer<Repr = I>, I, J, K> Sub<J> for Saturating<T>
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

impl<T: Integer<Repr = I>, I, J, K> Mul<J> for Saturating<T>
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

impl<T: Integer<Repr = I>, I, J, K> Div<J> for Saturating<T>
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

    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    #[cfg_attr(test, derive(test_strategy::Arbitrary))]
    struct Asymmetric(#[cfg_attr(test, strategy(Self::RANGE))] <Self as Integer>::Repr);

    unsafe impl Integer for Asymmetric {
        type Repr = i8;

        const MIN: Self::Repr = -89;
        const MAX: Self::Repr = 111;

        #[inline(always)]
        fn repr(&self) -> Self::Repr {
            self.0
        }
    }

    #[proptest]
    fn new_accepts_integers_within_bounds(#[strategy(Asymmetric::RANGE)] i: i8) {
        assert_eq!(Saturating::<Asymmetric>::new(i).get(), i);
    }

    #[proptest]
    #[should_panic]
    fn new_panics_if_integer_greater_than_max(#[strategy(Asymmetric::MAX + 1..)] i: i8) {
        Saturating::<Asymmetric>::new(i);
    }

    #[proptest]
    #[should_panic]
    fn new_panics_if_integer_smaller_than_min(#[strategy(..Asymmetric::MIN)] i: i8) {
        Saturating::<Asymmetric>::new(i);
    }

    #[proptest]
    fn get_returns_raw_integer(s: Saturating<Asymmetric>) {
        assert_eq!(s.get(), s.repr());
    }

    #[proptest]
    fn saturate_preserves_integers_within_bounds(#[strategy(Asymmetric::RANGE)] i: i8) {
        assert_eq!(
            Saturating::<Asymmetric>::saturate(i),
            Saturating::<Asymmetric>::new(i)
        );
    }

    #[proptest]
    fn saturate_caps_if_greater_than_max(#[strategy(Asymmetric::MAX + 1..)] i: i8) {
        assert_eq!(
            Saturating::<Asymmetric>::saturate(i),
            Saturating::<Asymmetric>::MAX
        );
    }

    #[proptest]
    fn saturate_caps_if_smaller_than_min(#[strategy(..Asymmetric::MIN)] i: i8) {
        assert_eq!(
            Saturating::<Asymmetric>::saturate(i),
            Saturating::<Asymmetric>::MIN
        );
    }

    #[proptest]
    fn negation_saturates(s: Saturating<Asymmetric>) {
        let r = Saturating::<Asymmetric>::saturate(s.get().saturating_neg());

        assert_eq!(-s, r);
    }

    #[proptest]
    fn addition_saturates(a: Saturating<Asymmetric>, b: Saturating<Asymmetric>) {
        let r = Saturating::<Asymmetric>::saturate(i8::saturating_add(a.get(), b.get()));

        assert_eq!(a + b, r);
        assert_eq!(a + b.get(), r);
    }

    #[proptest]
    fn subtraction_saturates(a: Saturating<Asymmetric>, b: Saturating<Asymmetric>) {
        let r = Saturating::<Asymmetric>::saturate(i8::saturating_sub(a.get(), b.get()));

        assert_eq!(a - b, r);
        assert_eq!(a - b.get(), r);
    }

    #[proptest]
    fn multiplication_saturates(a: Saturating<Asymmetric>, b: Saturating<Asymmetric>) {
        let r = Saturating::<Asymmetric>::saturate(i8::saturating_mul(a.get(), b.get()));

        assert_eq!(a * b, r);
        assert_eq!(a * b.get(), r);
    }

    #[proptest]
    fn division_saturates(a: Saturating<Asymmetric>, #[filter(#b != 0)] b: Saturating<Asymmetric>) {
        let r = Saturating::<Asymmetric>::saturate(i8::saturating_div(a.get(), b.get()));

        assert_eq!(a / b, r);
        assert_eq!(a / b.get(), r);
    }
}

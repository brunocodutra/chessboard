use crate::util::{Integer, Signed};
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::{cmp::Ordering, mem::size_of};

/// A saturating bounded integer.
#[derive(Debug, Default, Copy, Clone, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(transparent)]
pub struct Saturating<T>(T);

unsafe impl<T: ~const Integer> const Integer for Saturating<T> {
    type Repr = T::Repr;
    const MIN: Self::Repr = T::MIN;
    const MAX: Self::Repr = T::MAX;
}

impl<T: Integer> Eq for Saturating<T> where Self: PartialEq<Self> {}

impl<T, U, I, J> PartialEq<U> for Saturating<T>
where
    T: Integer<Repr = I>,
    U: Integer<Repr = J>,
    I: Signed,
    J: Signed,
{
    #[inline(always)]
    fn eq(&self, other: &U) -> bool {
        if size_of::<I>() <= size_of::<J>() {
            J::eq(&self.cast(), &other.cast())
        } else {
            I::eq(&self.cast(), &other.cast())
        }
    }
}

impl<T, I> Ord for Saturating<T>
where
    T: Integer<Repr = I>,
    I: Signed + Ord,
{
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T, U, I, J> PartialOrd<U> for Saturating<T>
where
    T: Integer<Repr = I>,
    U: Integer<Repr = J>,
    I: Signed,
    J: Signed,
{
    #[inline(always)]
    fn partial_cmp(&self, other: &U) -> Option<Ordering> {
        if size_of::<I>() <= size_of::<J>() {
            J::partial_cmp(&self.cast(), &other.cast())
        } else {
            I::partial_cmp(&self.cast(), &other.cast())
        }
    }
}

impl<T, I, J> Neg for Saturating<T>
where
    T: Integer<Repr = I>,
    I: Widen<Wider = J>,
    J: Signed + Neg<Output = J>,
{
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self::Output {
        J::neg(self.cast()).saturate()
    }
}

impl<T, U, I, J> Add<U> for Saturating<T>
where
    T: Integer<Repr = I>,
    U: Integer<Repr = J>,
    I: Widen,
    J: Widen,
{
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: U) -> Self::Output {
        if size_of::<I::Wider>() <= size_of::<J::Wider>() {
            J::Wider::add(self.cast(), rhs.cast()).saturate()
        } else {
            I::Wider::add(self.cast(), rhs.cast()).saturate()
        }
    }
}

impl<T, U, I, J> Sub<U> for Saturating<T>
where
    T: Integer<Repr = I>,
    U: Integer<Repr = J>,
    I: Widen,
    J: Widen,
{
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: U) -> Self::Output {
        if size_of::<I::Wider>() <= size_of::<J::Wider>() {
            J::Wider::sub(self.cast(), rhs.cast()).saturate()
        } else {
            I::Wider::sub(self.cast(), rhs.cast()).saturate()
        }
    }
}

impl<T, U, I, J> Mul<U> for Saturating<T>
where
    T: Integer<Repr = I>,
    U: Integer<Repr = J>,
    I: Widen,
    J: Widen,
{
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: U) -> Self::Output {
        if size_of::<I::Wider>() <= size_of::<J::Wider>() {
            J::Wider::mul(self.cast(), rhs.cast()).saturate()
        } else {
            I::Wider::mul(self.cast(), rhs.cast()).saturate()
        }
    }
}

impl<T, U, I, J> Div<U> for Saturating<T>
where
    T: Integer<Repr = I>,
    U: Integer<Repr = J>,
    I: Widen,
    J: Widen,
{
    type Output = Self;

    #[inline(always)]
    fn div(self, rhs: U) -> Self::Output {
        if size_of::<I::Wider>() <= size_of::<J::Wider>() {
            J::Wider::div(self.cast(), rhs.cast()).saturate()
        } else {
            I::Wider::div(self.cast(), rhs.cast()).saturate()
        }
    }
}

trait Widen: Signed {
    type Wider: Signed;
}

impl Widen for i8 {
    type Wider = i16;
}

impl Widen for i16 {
    type Wider = i32;
}

impl Widen for i32 {
    type Wider = i64;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;
    use test_strategy::proptest;

    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    #[cfg_attr(test, derive(test_strategy::Arbitrary))]
    #[repr(transparent)]
    struct Asymmetric(#[cfg_attr(test, strategy(Self::MIN..=Self::MAX))] <Self as Integer>::Repr);

    unsafe impl Integer for Asymmetric {
        type Repr = i16;
        const MIN: Self::Repr = -89;
        const MAX: Self::Repr = 131;
    }

    #[proptest]
    fn comparison_coerces(a: Saturating<Asymmetric>, b: i8) {
        assert_eq!(a == b, a.get() == b.into());
        assert_eq!(a <= b, a.get() <= b.into());
    }

    #[proptest]
    fn negation_saturates(s: Saturating<Asymmetric>) {
        assert_eq!(-s, s.get().saturating_neg().saturate::<Asymmetric>());
    }

    #[proptest]
    fn addition_saturates(a: Saturating<Asymmetric>, b: Saturating<i8>) {
        let r: Asymmetric = i16::saturating_add(a.cast(), b.cast()).saturate();
        assert_eq!(a + b, r);

        let r: i8 = i16::saturating_add(b.cast(), a.cast()).saturate();
        assert_eq!(b + a, r);
    }

    #[proptest]
    fn subtraction_saturates(a: Saturating<Asymmetric>, b: Saturating<i8>) {
        let r: Asymmetric = i16::saturating_sub(a.cast(), b.cast()).saturate();
        assert_eq!(a - b, r);

        let r: i8 = i16::saturating_sub(b.cast(), a.cast()).saturate();
        assert_eq!(b - a, r);
    }

    #[proptest]
    fn multiplication_saturates(a: Saturating<Asymmetric>, b: Saturating<i8>) {
        let r: Asymmetric = i16::saturating_mul(a.cast(), b.cast()).saturate();
        assert_eq!(a * b, r);

        let r: i8 = i16::saturating_mul(b.cast(), a.cast()).saturate();
        assert_eq!(b * a, r);
    }

    #[proptest]
    fn division_saturates(
        #[filter(#a != 0)] a: Saturating<Asymmetric>,
        #[filter(#b != 0)] b: Saturating<i8>,
    ) {
        let r: Asymmetric = i16::saturating_div(a.cast(), b.cast()).saturate();
        assert_eq!(a / b, r);

        let r: i8 = i16::saturating_div(b.cast(), a.cast()).saturate();
        assert_eq!(b / a, r);
    }
}

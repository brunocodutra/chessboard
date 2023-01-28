use super::Saturate;
use derive_more::{Deref, Display};
use std::ops::{Add, Div, Mul, Neg, Sub};
use test_strategy::Arbitrary;

/// A saturating numeric type.
#[derive(
    Debug, Display, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary, Deref,
)]
pub struct Saturating<S>(pub S);

impl<S: Saturate> Saturate for Saturating<S> {
    type Primitive = S::Primitive;

    const ZERO: Self = Saturating(S::ZERO);
    const MIN: Self = Saturating(S::MIN);
    const MAX: Self = Saturating(S::MAX);

    #[inline]
    fn new(i: Self::Primitive) -> Self {
        Saturating(S::new(i))
    }

    #[inline]
    fn get(&self) -> Self::Primitive {
        self.0.get()
    }
}

impl<S> Neg for Saturating<S>
where
    S: Saturate,
    S::Primitive: Into<i64>,
{
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Saturating::saturate(i64::saturating_neg(self.get().into()))
    }
}

impl<L, R> Add<R> for Saturating<L>
where
    L: Saturate,
    R: Saturate,
    L::Primitive: Into<i64>,
    R::Primitive: Into<i64>,
{
    type Output = Self;

    #[inline]
    fn add(self, rhs: R) -> Self::Output {
        Saturating::saturate(i64::saturating_add(self.get().into(), rhs.get().into()))
    }
}

impl<L, R> Sub<R> for Saturating<L>
where
    L: Saturate,
    R: Saturate,
    L::Primitive: Into<i64>,
    R::Primitive: Into<i64>,
{
    type Output = Self;

    #[inline]
    fn sub(self, rhs: R) -> Self::Output {
        Saturating::saturate(i64::saturating_sub(self.get().into(), rhs.get().into()))
    }
}

impl<L, R> Mul<R> for Saturating<L>
where
    L: Saturate,
    R: Saturate,
    L::Primitive: Into<i64>,
    R::Primitive: Into<i64>,
{
    type Output = Self;

    #[inline]
    fn mul(self, rhs: R) -> Self::Output {
        Saturating::saturate(i64::saturating_mul(self.get().into(), rhs.get().into()))
    }
}

impl<L, R> Div<R> for Saturating<L>
where
    L: Saturate,
    R: Saturate,
    L::Primitive: Into<i64>,
    R::Primitive: Into<i64>,
{
    type Output = Self;

    #[inline]
    fn div(self, rhs: R) -> Self::Output {
        Saturating::saturate(i64::saturating_div(self.get().into(), rhs.get().into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn negation_saturates(s: Saturating<i8>) {
        assert_eq!(-s, Saturating(s.get().saturating_neg()));
    }

    #[proptest]
    fn addition_saturates(a: Saturating<i8>, b: Saturating<i8>) {
        assert_eq!(a + b, Saturating(i8::saturating_add(a.get(), b.get())));
    }

    #[proptest]
    fn subtraction_saturates(a: Saturating<i8>, b: Saturating<i8>) {
        assert_eq!(a - b, Saturating(i8::saturating_sub(a.get(), b.get())));
    }

    #[proptest]
    fn multiplication_saturates(a: Saturating<i8>, b: Saturating<i8>) {
        assert_eq!(a * b, Saturating(i8::saturating_mul(a.get(), b.get())));
    }

    #[proptest]
    fn division_saturates(a: Saturating<i8>, #[filter(#b != Saturating::ZERO)] b: Saturating<i8>) {
        assert_eq!(a / b, Saturating(i8::saturating_div(a.get(), b.get())));
    }

    #[proptest]
    fn double_negation_is_idempotent(#[filter(#s != Saturating::MIN)] s: Saturating<i8>) {
        assert_eq!(-(-s), s);
    }

    #[proptest]
    fn display_is_transparent(s: Saturating<i8>) {
        assert_eq!(s.to_string(), s.get().to_string());
    }
}

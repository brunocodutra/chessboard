use super::Saturating;
use num_traits::{cast, clamp, PrimInt};

/// Trait for saturating integers.
pub trait Saturate: Copy {
    /// Equivalent primitive integer;
    type Primitive: PrimInt;

    /// This type's representation of the constant zero.
    const ZERO: Self;

    /// This type's minimum value.
    const MIN: Self = Self::ZERO;

    /// This type's maximum value.
    const MAX: Self;

    /// Constructs `Self` from an equivalent [`Self::Primitive`].
    ///
    /// # Panics
    ///
    /// Panics if `i` is outside of the bounds.
    fn new(i: Self::Primitive) -> Self;

    /// Returns the equivalent [`Self::Primitive`].
    fn get(&self) -> Self::Primitive;

    /// Constructs `Self` from another saturating integer.
    #[inline]
    fn saturate<S: Saturate>(s: S) -> Self {
        let min = cast(Self::MIN.get()).unwrap_or(S::MIN.get());
        let max = cast(Self::MAX.get()).unwrap_or(S::MAX.get());
        Self::new(cast(clamp(s.get(), min, max)).unwrap())
    }

    /// Cast `Self` to another saturating integer.
    #[inline]
    fn cast<S: Saturate>(self) -> S {
        S::saturate(self)
    }

    /// Constructs [`Saturating`] from `Self`.
    #[inline]
    fn saturating(self) -> Saturating<Self> {
        Saturating(self)
    }
}

macro_rules! impl_integer_for_primitive {
    ($i: ty) => {
        impl Saturate for $i {
            type Primitive = Self;

            const ZERO: Self = 0;
            const MIN: Self = <$i>::MIN;
            const MAX: Self = <$i>::MAX;

            #[inline]
            fn new(i: Self::Primitive) -> Self {
                i
            }

            #[inline]
            fn get(&self) -> Self::Primitive {
                *self
            }
        }
    };
}

impl_integer_for_primitive!(i8);
impl_integer_for_primitive!(i16);
impl_integer_for_primitive!(i32);
impl_integer_for_primitive!(i64);
impl_integer_for_primitive!(i128);
impl_integer_for_primitive!(isize);

impl_integer_for_primitive!(u8);
impl_integer_for_primitive!(u16);
impl_integer_for_primitive!(u32);
impl_integer_for_primitive!(u64);
impl_integer_for_primitive!(u128);
impl_integer_for_primitive!(usize);

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::{proptest, Arbitrary};

    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary)]
    struct Saturated<const MIN: i8, const MAX: i8>(#[strategy(MIN..=MAX)] i8);

    impl<const MIN: i8, const MAX: i8> Saturate for Saturated<MIN, MAX> {
        type Primitive = i8;

        const ZERO: Self = Saturated(0);
        const MIN: Self = Saturated(MIN);
        const MAX: Self = Saturated(MAX);

        fn new(i: Self::Primitive) -> Self {
            assert!((MIN..=MAX).contains(&i));
            Saturated(i)
        }

        fn get(&self) -> Self::Primitive {
            self.0
        }
    }

    #[proptest]
    fn new_accepts_integers_within_bounds(#[strategy(-5i8..=9)] i: i8) {
        assert_eq!(Saturated::<-5, 9>::new(i), Saturated(i));
    }

    #[proptest]
    fn get_returns_raw_integer(s: Saturated<-5, 9>) {
        assert_eq!(s.get(), s.0);
    }

    #[proptest]
    fn saturate_preserves_integers_within_bounds(#[strategy(-5..=9)] s: i32) {
        assert_eq!(Saturated::<-5, 9>::saturate(s), Saturated::new(s as _));
    }

    #[proptest]
    fn saturate_caps_if_greater_than_max(#[strategy(10..)] s: i32) {
        assert_eq!(Saturated::<-5, 9>::saturate(s), Saturated::<-5, 9>::MAX);
    }

    #[proptest]
    fn saturate_caps_if_smaller_than_min(#[strategy(..-5)] s: i32) {
        assert_eq!(Saturated::<-5, 9>::saturate(s), Saturated::<-5, 9>::MIN);
    }
}

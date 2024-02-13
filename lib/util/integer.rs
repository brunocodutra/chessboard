use num_traits::PrimInt;
use std::{iter::Map, mem::transmute_copy, ops::RangeInclusive};

/// Trait for types that can be represented by a contiguous range of primitive integers.
///
/// # Safety
///
/// Must only be implemented for types that can be safely transmuted to and from [`Integer::Repr`].
#[const_trait]
pub unsafe trait Integer: Copy {
    /// The equivalent primitive integer type.
    type Repr: PrimInt;

    /// The minimum repr.
    const MIN: Self::Repr;

    /// The maximum repr.
    const MAX: Self::Repr;

    /// The minimum value.
    #[inline(always)]
    fn lower() -> Self {
        Self::from_repr(Self::MIN)
    }

    /// The maximum value.
    #[inline(always)]
    fn upper() -> Self {
        Self::from_repr(Self::MAX)
    }

    /// Casts to [`Integer::Repr`].
    fn repr(&self) -> Self::Repr {
        unsafe { transmute_copy(self) }
    }

    /// Casts from [`Integer::Repr`].
    #[inline(always)]
    fn from_repr(i: Self::Repr) -> Self {
        unsafe { transmute_copy(&i) }
    }

    /// An iterator over all values in the range [`Integer::MIN`]..=[`Integer::MAX`].
    #[inline(always)]
    #[allow(clippy::type_complexity)]
    fn iter() -> Map<RangeInclusive<Self::Repr>, fn(Self::Repr) -> Self>
    where
        RangeInclusive<Self::Repr>: Iterator<Item = Self::Repr>,
    {
        (Self::MIN..=Self::MAX).map(Self::from_repr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::{proptest, Arbitrary};

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary)]
    #[repr(u16)]
    enum Digit {
        One = 1,
        Two,
        Three,
        Four,
        Five,
        Six,
        Seven,
        Eight,
        Nine,
    }

    unsafe impl const Integer for Digit {
        type Repr = u16;
        const MIN: Self::Repr = Digit::One as _;
        const MAX: Self::Repr = Digit::Nine as _;
    }

    #[proptest]
    fn can_be_cast_to_integer(d: Digit) {
        assert_eq!(Digit::from_repr(d.repr()), d);
    }

    #[proptest]

    fn can_be_cast_from_integer(#[strategy(1u16..10)] i: u16) {
        assert_eq!(Digit::from_repr(i).repr(), i);
    }

    #[proptest]
    fn is_ordered_by_repr(a: Digit, b: Digit) {
        assert_eq!(a < b, a.repr() < b.repr());
    }

    #[proptest]
    fn can_be_iterated_in_order() {
        assert_eq!(
            Vec::from_iter(Digit::iter()),
            vec![
                Digit::One,
                Digit::Two,
                Digit::Three,
                Digit::Four,
                Digit::Five,
                Digit::Six,
                Digit::Seven,
                Digit::Eight,
                Digit::Nine,
            ],
        );
    }
}

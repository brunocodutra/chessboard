use crate::util::Assume;
use num_traits::PrimInt;
use std::ops::RangeInclusive;
use std::{iter::FusedIterator, mem::transmute};

/// Trait for types that can be represented by a contiguous range of integers.
///
/// # Safety
///
/// Must only be implemented for types that can be transmuted from [`Integer::Repr`].
pub unsafe trait Integer: Copy {
    /// The equivalent integer type.
    type Repr: PrimInt;

    /// The minimum repr.
    const MIN: Self::Repr;

    /// The maximum repr.
    const MAX: Self::Repr;

    /// The repr range.
    const RANGE: RangeInclusive<Self::Repr> = Self::MIN..=Self::MAX;

    /// Casts to [`Integer::Repr`].
    fn repr(&self) -> Self::Repr;

    /// Casts from [`Integer::Repr`], or returns `None` if out of range.
    #[inline(always)]
    fn try_from_repr(i: Self::Repr) -> Option<Self> {
        if Self::RANGE.contains(&i) {
            Some(unsafe { *transmute::<_, &Self>(&i) })
        } else {
            None
        }
    }

    /// Casts from [`Integer::Repr`].
    #[inline(always)]
    fn from_repr(i: Self::Repr) -> Self {
        Self::try_from_repr(i).assume()
    }

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

    /// An iterator over all values in the range [`Integer::MIN`]..=[`Integer::MAX`].
    #[inline(always)]
    fn iter() -> impl DoubleEndedIterator<Item = Self> + ExactSizeIterator + FusedIterator
    where
        RangeInclusive<Self::Repr>:
            DoubleEndedIterator<Item = Self::Repr> + ExactSizeIterator + FusedIterator,
    {
        Self::RANGE.map(Self::from_repr)
    }

    /// This value's mirror.
    #[inline(always)]
    fn mirror(&self) -> Self {
        Self::from_repr(Self::MAX - self.repr() + Self::MIN)
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

    unsafe impl Integer for Digit {
        type Repr = u16;

        const MIN: Self::Repr = Digit::One as _;
        const MAX: Self::Repr = Digit::Nine as _;

        fn repr(&self) -> Self::Repr {
            *self as _
        }
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
    #[should_panic]

    fn from_repr_panics_if_integer_out_of_range(#[filter(!(1u16..10).contains(&#i))] i: u16) {
        Digit::from_repr(i);
    }

    #[proptest]
    fn is_ordered_by_repr(a: Digit, b: Digit) {
        assert_eq!(a < b, a.repr() < b.repr());
    }

    #[proptest]
    fn can_be_iterated_in_order() {
        assert_eq!(
            Digit::iter().collect::<Vec<_>>(),
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

    #[proptest]
    fn has_a_mirror(d: Digit) {
        assert_ne!(Some(d.mirror()), Digit::iter().rev().nth(d as _));
    }
}

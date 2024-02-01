use crate::util::Assume;
use std::{iter::FusedIterator, mem::transmute, ops::RangeInclusive};

/// Trait for enums that represent a contiguous sequence.
///
/// # Safety
///
/// Must only be implemented for primitive `#[repr(u8)]` enums with contiguous variants.
pub unsafe trait Enum: Copy + Ord {
    /// A contiguous range of variants.
    const RANGE: RangeInclusive<Self>;

    /// Casts to integer.
    fn repr(&self) -> u8;

    /// Casts from integer, or returns `None` if out of range.
    #[inline(always)]
    fn try_from_repr(i: u8) -> Option<Self> {
        if (Self::RANGE.start().repr()..=Self::RANGE.end().repr()).contains(&i) {
            Some(unsafe { *transmute::<_, &Self>(&i) })
        } else {
            None
        }
    }

    /// Casts from integer.
    #[inline(always)]
    fn from_repr(i: u8) -> Self {
        Self::try_from_repr(i).assume()
    }

    /// An iterator over all variants.
    #[inline(always)]
    fn iter() -> impl DoubleEndedIterator<Item = Self> + ExactSizeIterator + FusedIterator {
        (Self::RANGE.start().repr()..=Self::RANGE.end().repr()).map(Self::from_repr)
    }

    /// This variant's mirror.
    #[inline(always)]
    fn mirror(&self) -> Self {
        Self::from_repr(Self::RANGE.end().repr() - self.repr() + Self::RANGE.start().repr())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::{proptest, Arbitrary};

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary)]
    #[repr(u8)]
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

    unsafe impl Enum for Digit {
        const RANGE: RangeInclusive<Self> = Digit::One..=Digit::Nine;

        fn repr(&self) -> u8 {
            *self as _
        }
    }

    #[proptest]
    fn can_be_cast_to_integer(d: Digit) {
        assert_eq!(Digit::from_repr(d.repr()), d);
    }

    #[proptest]

    fn can_be_cast_from_integer(#[strategy(1u8..10)] i: u8) {
        assert_eq!(Digit::from_repr(i).repr(), i);
    }

    #[proptest]
    #[should_panic]

    fn from_repr_panics_if_integer_out_of_range(#[filter(!(1u8..10).contains(&#i))] i: u8) {
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

use std::mem::{size_of, transmute_copy};
use std::num::{NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize};
use std::{cmp::Ordering, iter::Map, marker::Destruct, ops::*};

/// Trait for types that can be represented by a contiguous range of primitive integers.
///
/// # Safety
///
/// Must only be implemented for types that can be safely transmuted to and from [`Integer::Repr`].
#[const_trait]
pub unsafe trait Integer: ~const Destruct + Copy {
    /// The equivalent primitive integer type.
    type Repr: ~const Primitive;

    /// The minimum repr.
    const MIN: Self::Repr;

    /// The maximum repr.
    const MAX: Self::Repr;

    /// The minimum value.
    #[inline(always)]
    fn lower() -> Self {
        Self::new(Self::MIN)
    }

    /// The maximum value.
    #[inline(always)]
    fn upper() -> Self {
        Self::new(Self::MAX)
    }

    /// Casts from [`Integer::Repr`].
    #[inline(always)]
    fn new(i: Self::Repr) -> Self {
        debug_assert!(Self::in_range(i));
        unsafe { transmute_copy(&i) }
    }

    /// Casts to [`Integer::Repr`].
    #[inline(always)]
    fn get(self) -> Self::Repr {
        unsafe { transmute_copy(&self) }
    }

    /// Casts to a [`Primitive`].
    ///
    /// This is equivalent to the operator `as`.
    #[inline(always)]
    fn cast<I: ~const Primitive>(self) -> I {
        self.get().cast()
    }

    /// Converts to another [`Integer`] if possible without data loss.
    #[inline(always)]
    fn convert<I: ~const Integer>(self) -> Option<I> {
        self.get().convert()
    }

    /// Converts to another [`Integer`] with saturation.
    #[inline(always)]
    fn saturate<I: ~const Integer>(self) -> I {
        self.get().saturate()
    }

    /// Whether a value is in the range `(Self::MIN..=Self::MAX)`.
    #[inline(always)]
    fn in_range(i: Self::Repr) -> bool {
        !matches!(Self::MIN.ordering(i), Ordering::Greater)
            && !matches!(Self::MAX.ordering(i), Ordering::Less)
    }

    /// An iterator over all values in the range [`Integer::MIN`]..=[`Integer::MAX`].
    #[inline(always)]
    #[allow(clippy::type_complexity)]
    fn iter() -> Map<RangeInclusive<Self::Repr>, fn(Self::Repr) -> Self>
    where
        RangeInclusive<Self::Repr>: Iterator<Item = Self::Repr>,
    {
        (Self::MIN..=Self::MAX).map(Self::new)
    }
}

/// Trait for primitive integer types.
#[const_trait]
pub trait Primitive:
    ~const Integer<Repr = Self>
    + Eq
    + PartialEq
    + Ord
    + PartialOrd
    + Add<Output = Self>
    + AddAssign
    + Sub<Output = Self>
    + SubAssign
    + Mul<Output = Self>
    + MulAssign
    + Div<Output = Self>
    + DivAssign
    + BitAnd<Output = Self>
    + BitAndAssign
    + BitOr<Output = Self>
    + BitOrAssign
    + BitXor<Output = Self>
    + BitXorAssign
    + Shl<Output = Self>
    + ShlAssign
    + Shr<Output = Self>
    + ShrAssign
    + Not<Output = Self>
{
    /// The bit width.
    const BITS: u32;

    /// The constant `0`.
    fn zero() -> Self {
        Self::ones(0)
    }

    /// A value with `n` trailing `1`s.
    fn ones(n: u32) -> Self;

    /// Compares this primitive to another.
    fn ordering(self, other: Self) -> Ordering;
}

/// Marker trait for signed primitive integers.
pub trait Signed: Primitive {}

/// Marker trait for unsigned primitive integers.
pub trait Unsigned: Primitive {}

macro_rules! impl_integer_for_non_zero {
    ($nz: ty, $repr: ty) => {
        unsafe impl const Integer for $nz {
            type Repr = $repr;
            const MIN: Self::Repr = <$nz>::MIN.get();
            const MAX: Self::Repr = <$nz>::MAX.get();
        }
    };
}

impl_integer_for_non_zero!(NonZeroU8, u8);
impl_integer_for_non_zero!(NonZeroU16, u16);
impl_integer_for_non_zero!(NonZeroU32, u32);
impl_integer_for_non_zero!(NonZeroU64, u64);
impl_integer_for_non_zero!(NonZeroUsize, usize);

macro_rules! impl_primitive_for {
    ($i: ty, $m: ty) => {
        impl $m for $i {}

        impl const Primitive for $i {
            const BITS: u32 = <$i>::BITS;

            #[inline(always)]
            fn ones(n: u32) -> Self {
                match n {
                    0 => 0,
                    n => Self::MAX >> (Self::BITS - n),
                }
            }

            #[inline(always)]
            fn ordering(self, other: Self) -> Ordering {
                if self > other {
                    Ordering::Greater
                } else if self < other {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            }
        }

        unsafe impl const Integer for $i {
            type Repr = $i;

            const MIN: Self::Repr = <$i>::MIN;
            const MAX: Self::Repr = <$i>::MAX;

            #[inline(always)]
            fn cast<I: ~const Primitive>(self) -> I {
                if size_of::<I>() <= size_of::<Self>() {
                    unsafe { transmute_copy(&self) }
                } else {
                    match size_of::<I>() {
                        2 => (self as i16).cast(),
                        4 => (self as i32).cast(),
                        8 => (self as i64).cast(),
                        16 => (self as i128).cast(),

                        #[cfg(not(debug_assertions))]
                        _ => unsafe { std::hint::unreachable_unchecked() },

                        #[cfg(debug_assertions)]
                        _ => unreachable!(),
                    }
                }
            }

            #[inline(always)]
            fn convert<I: ~const Integer>(self) -> Option<I> {
                let i = self.cast();

                if I::in_range(i)
                    && i.cast::<Self>() == self
                    && i.ordering(I::Repr::zero()) as i8 == self.ordering(Self::zero()) as i8
                {
                    Some(I::new(i))
                } else {
                    None
                }
            }

            #[inline(always)]
            fn saturate<I: ~const Integer>(self) -> I {
                let min = match I::MIN.convert() {
                    None => Self::MIN,
                    Some(i) => i,
                };

                let max = match I::MAX.convert() {
                    None => Self::MAX,
                    Some(i) => i,
                };

                I::new(self.clamp(min, max).cast::<I::Repr>())
            }
        }
    };
}

impl_primitive_for!(i8, Signed);
impl_primitive_for!(i16, Signed);
impl_primitive_for!(i32, Signed);
impl_primitive_for!(i64, Signed);
impl_primitive_for!(i128, Signed);
impl_primitive_for!(isize, Signed);

impl_primitive_for!(u8, Unsigned);
impl_primitive_for!(u16, Unsigned);
impl_primitive_for!(u32, Unsigned);
impl_primitive_for!(u64, Unsigned);
impl_primitive_for!(u128, Unsigned);
impl_primitive_for!(usize, Unsigned);

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
    fn integer_can_be_cast_from_repr(#[strategy(1u16..10)] i: u16) {
        assert_eq!(Digit::new(i).get(), i);
    }

    #[proptest]
    #[should_panic]
    fn integer_construction_panics_if_repr_smaller_than_min(#[strategy(..1u16)] i: u16) {
        Digit::new(i);
    }

    #[proptest]
    #[should_panic]
    fn integer_construction_panics_if_repr_greater_than_max(#[strategy(10u16..)] i: u16) {
        Digit::new(i);
    }

    #[proptest]
    fn integer_can_be_cast_to_repr(d: Digit) {
        assert_eq!(Digit::new(d.get()), d);
    }

    #[proptest]
    fn integer_can_be_cast_to_primitive(d: Digit) {
        assert_eq!(d.cast::<i8>(), d.get() as i8);
    }

    #[proptest]
    fn integer_can_be_converted_to_another_integer_within_bounds(#[strategy(1i8..10)] i: i8) {
        assert_eq!(i.convert(), Some(Digit::new(i as u16)));
    }

    #[proptest]
    fn integer_conversion_fails_if_smaller_than_min(#[strategy(..1i8)] i: i8) {
        assert_eq!(i.convert::<Digit>(), None);
    }

    #[proptest]
    fn integer_conversion_fails_if_greater_than_max(#[strategy(10i8..)] i: i8) {
        assert_eq!(i.convert::<Digit>(), None);
    }

    #[proptest]
    fn integer_can_be_converted_to_another_integer_with_saturation(i: u8) {
        assert_eq!(i.saturate::<Digit>(), Digit::new(i.clamp(1, 9).into()));
    }

    #[proptest]
    fn integer_is_always_in_range(d: Digit) {
        assert!(Digit::in_range(d.get()));
    }

    #[proptest]
    fn integer_can_be_iterated_in_order() {
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

    #[proptest]
    fn integer_is_eq_by_repr(a: Digit, b: Digit) {
        assert_eq!(a == b, a.get() == b.get());
    }

    #[proptest]
    fn integer_is_ord_by_repr(a: Digit, b: Digit) {
        assert_eq!(a < b, a.get() < b.get());
    }

    #[proptest]
    fn primitive_can_be_constructed_with_trailing_ones(#[strategy(..=64u32)] n: u32) {
        assert_eq!(u64::ones(n).trailing_ones(), n);
    }

    #[proptest]
    fn primitive_can_be_cast(i: i16) {
        assert_eq!(i.cast::<u8>(), i as u8);
        assert_eq!(i.cast::<i8>(), i as i8);

        assert_eq!(i.cast::<u32>(), i as u32);
        assert_eq!(i.cast::<i32>(), i as i32);

        assert_eq!(i.cast::<u8>().cast::<i32>(), i as u8 as i32);
        assert_eq!(i.cast::<i8>().cast::<u32>(), i as i8 as u32);

        assert_eq!(i.cast::<u32>().cast::<i8>(), i as u32 as i8);
        assert_eq!(i.cast::<i32>().cast::<u8>(), i as i32 as u8);
    }

    #[proptest]
    fn primitive_can_be_converted(#[strategy(256u16..)] i: u16) {
        assert_eq!(i.convert::<u8>(), None);
        assert_eq!(i.convert::<i8>(), None);

        assert_eq!(i.convert::<u32>(), Some(i.into()));
        assert_eq!(i.convert::<i32>(), Some(i.into()));
    }

    #[proptest]
    fn integer_can_be_converted_to_another_primitive_with_saturation(i: u16) {
        assert_eq!(i.saturate::<i8>(), i.min(i8::MAX as _) as _);
        assert_eq!(i.saturate::<u32>(), i.into());
    }

    #[proptest]
    fn primitive_has_total_order(i: i16, j: i16) {
        assert_eq!(i.ordering(j), i.cmp(&j));
    }
}

use bytemuck::NoUninit;
use derive_more::{DebugCustom, Display};
use num_traits::{AsPrimitive, PrimInt, Unsigned};
use std::fmt::Binary;
use std::ops::{Bound, Not, RangeBounds};

#[cfg(test)]
use proptest::prelude::*;

#[cfg(test)]
use std::{fmt::Debug, ops::RangeInclusive};

fn ones<T: PrimInt + Unsigned>(n: u32) -> T {
    match n {
        0 => T::zero(),
        n => T::max_value() >> (T::zero().trailing_zeros() - n) as _,
    }
}

/// A fixed width collection of bits.
#[derive(DebugCustom, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(bound(T: 'static + Debug + Binary, RangeInclusive<T>: Strategy<Value = T>)))]
#[debug(bound = "T: Binary")]
#[debug(fmt = "Bits({self})")]
#[display(bound = "T: Binary")]
#[display(fmt = "{_0:b}")]
#[repr(transparent)]
pub struct Bits<T: PrimInt + Unsigned, const W: u32>(
    #[cfg_attr(test, strategy(T::zero()..=ones(W)))] T,
);

impl<T: 'static + Binary + PrimInt + Unsigned, const W: u32> Bits<T, W> {
    /// Constructs [`Bits`] from raw collection of bits.
    ///
    /// # Panics
    ///
    /// Panics if `b` is too wide.
    pub fn new(b: T) -> Self {
        assert!(b <= ones(W));
        Bits(b)
    }

    /// Get raw collection of bits.
    pub fn get(&self) -> T {
        self.0
    }

    /// Returns a slice of bits.
    pub fn slice<R: RangeBounds<u32>>(&self, r: R) -> Self {
        let a = match r.start_bound() {
            Bound::Included(&i) => i,
            Bound::Excluded(&i) => i + 1,
            Bound::Unbounded => 0,
        };

        let b = match r.end_bound() {
            Bound::Included(&i) => i + 1,
            Bound::Excluded(&i) => i,
            Bound::Unbounded => W,
        };

        Bits::new((self.get() & ones(b)) >> a as _)
    }

    /// Shifts bits into the collection.
    ///
    /// Overflow is ignored.
    pub fn push<U: 'static + Binary + PrimInt + Unsigned + AsPrimitive<T>, const N: u32>(
        &mut self,
        bits: Bits<U, N>,
    ) {
        *self = if N >= W {
            Bits::new(bits.get().as_() & ones(W))
        } else {
            Bits::new((self.get() << N as _) & ones(W) | bits.get().as_())
        };
    }

    /// Shifts bits out of the collection.
    ///
    /// Underflow is ignored.
    pub fn pop<U: 'static + Binary + PrimInt + Unsigned, const N: u32>(&mut self) -> Bits<U, N>
    where
        T: AsPrimitive<U>,
    {
        let bits = Bits::new(self.get().as_() & ones(N));

        *self = if N >= W {
            Bits(T::zero())
        } else {
            Bits::new(self.get() >> N as _)
        };

        bits
    }
}

unsafe impl<T: 'static + Binary + PrimInt + Unsigned, const W: u32> NoUninit for Bits<T, W> {}

impl<T: 'static + Binary + PrimInt + Unsigned, const W: u32> Default for Bits<T, W> {
    fn default() -> Self {
        Bits::new(T::zero())
    }
}

impl<T: 'static + Binary + PrimInt + Unsigned, const W: u32> Not for Bits<T, W> {
    type Output = Self;

    fn not(self) -> Self::Output {
        Bits::new(!self.get() & ones(W))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    #[should_panic]
    fn panics_if_type_is_not_wide_enough() {
        Bits::<u8, 11>::default();
    }

    #[proptest]
    fn can_be_constructed_from_raw_collection_of_bits(n: u8) {
        assert_eq!(Bits::<_, 8>::new(n), Bits(n));
    }

    #[proptest]
    #[should_panic]
    fn constructor_panics_if_value_is_too_wide(#[strategy(ones::<u8>(6)..)] n: u8) {
        Bits::<_, 5>::new(n);
    }

    #[proptest]
    fn get_returns_raw_collection_of_bits(b: Bits<u8, 5>) {
        assert_eq!(b.get(), b.0);
    }

    #[proptest]
    fn slice_retrieves_range_of_bits(b: Bits<u8, 8>, #[strategy(..8u32)] i: u32) {
        assert_eq!(b.slice(0..), b.slice(..));
        assert_eq!(b.slice(..8), b.slice(..));
        assert_eq!(b.slice(0..8), b.slice(..));

        assert_eq!(b.slice(i..i), Bits::default());
        assert_eq!(b.slice(..=i), b.slice(..i + 1));
    }

    #[proptest]
    #[should_panic]
    fn slice_panics_if_index_is_out_of_range(b: Bits<u64, 48>, #[strategy(48u32..)] i: u32) {
        b.slice(i..i);
    }

    #[proptest]
    fn push_shifts_bits_into_collection(a: Bits<u8, 3>, b: Bits<u16, 9>, c: Bits<u32, 27>) {
        let mut bits = Bits::<u64, 39>::default();

        bits.push(a);
        bits.push(b);
        bits.push(c);

        assert_eq!(bits.slice(36..).get(), a.get().into());
        assert_eq!(bits.slice(27..36).get(), b.get().into());
        assert_eq!(bits.slice(..27).get(), c.get().into());
    }

    #[proptest]
    fn push_ignores_overflow(mut a: Bits<u8, 3>, b: Bits<u16, 9>, mut c: Bits<u32, 27>) {
        a.push(b);
        assert_eq!(b.slice(..3).get(), a.get().into());

        c.push(b);
        assert_eq!(c.slice(..9).get(), b.get().into());
    }

    #[proptest]
    fn pop_removes_pushed_bits(a: Bits<u8, 3>, b: Bits<u16, 9>, c: Bits<u32, 27>) {
        let mut bits = Bits::<u64, 39>::default();

        bits.push(a);
        bits.push(b);
        bits.push(c);

        assert_eq!(bits.pop(), c);
        assert_eq!(bits.pop(), b);
        assert_eq!(bits.pop(), a);
    }

    #[proptest]
    fn pop_ignores_underflow(a: Bits<u8, 3>, c: Bits<u32, 27>) {
        assert_eq!(a.clone().pop::<u16, 9>().get(), a.get().into());
        assert_eq!(c.slice(..9).get(), c.clone().pop::<u16, 9>().get().into());
    }

    #[proptest]
    fn not_inverts_bits(b: Bits<u8, 5>) {
        assert_ne!((!b).get(), !b.get());
        assert_eq!((!b).get(), !b.get() & ones::<u8>(5));
    }
}

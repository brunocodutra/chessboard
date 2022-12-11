use derive_more::{DebugCustom, Display};
use std::ops::{Bound, Not, RangeBounds};
use test_strategy::Arbitrary;

const fn ones(n: u32) -> u64 {
    match n {
        0 => 0,
        n => u64::MAX >> (64 - n),
    }
}

/// A fixed width collection of bits.
#[derive(DebugCustom, Display, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[debug(fmt = "Bits({})", self)]
#[display(fmt = "{:b}", _0)]
pub struct Bits<const W: u32>(#[strategy(0..=ones(W))] u64);

impl<const W: u32> Bits<W> {
    /// Constructs [`Bits`] from raw collection of bits.
    ///
    /// Overflown bits are discarded.
    #[inline(always)]
    pub fn new(b: u64) -> Self {
        Bits(b & ones(W))
    }

    /// Returns a slice of bits.
    #[inline(always)]
    pub fn get<R: RangeBounds<u32>>(&self, r: R) -> Self {
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

        Bits((self.0 & ones(b)) >> a)
    }

    /// Shifts bits into the collection.
    ///
    /// Overflown bits are discarded.
    ///
    /// # Panics
    ///
    /// Panics if `N` is greater than `W`.
    #[inline(always)]
    pub fn push<const N: u32>(&mut self, bits: Bits<N>) {
        assert!(W >= N);
        *self = Bits::new((self.0 << N) | bits.0)
    }

    /// Shifts bits out of the collection.
    ///
    /// # Panics
    ///
    /// Panics if `N` is greater than `W`.
    #[inline(always)]
    pub fn pop<const N: u32>(&mut self) -> Bits<N> {
        assert!(W >= N);
        let bits = Bits::new(self.0);
        self.0 >>= N;
        bits
    }
}

impl<const W: u32> Not for Bits<W> {
    type Output = Self;

    #[inline(always)]
    fn not(self) -> Self::Output {
        Bits::new(!self.0)
    }
}

macro_rules! impl_from_bits_for_integer {
    ( $type: ty ) => {
        impl<const W: u32> From<Bits<W>> for $type {
            #[inline(always)]
            fn from(b: Bits<W>) -> Self {
                b.0 as _
            }
        }
    };
}

impl_from_bits_for_integer!(usize);
impl_from_bits_for_integer!(u64);
impl_from_bits_for_integer!(u32);
impl_from_bits_for_integer!(u16);
impl_from_bits_for_integer!(u8);

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn can_be_constructed_from_raw_collection_of_bits(n: u64) {
        assert_eq!(Bits::<64>::new(n), Bits(n));
    }

    #[proptest]
    fn overflown_bits_are_discarded_by_constructor(#[strategy(ones(16)..)] n: u64) {
        assert_eq!(Bits::<16>::new(n), Bits(n & ones(16)));
    }

    #[proptest]
    fn get_slices_bits(b: Bits<48>, #[strategy(..48u32)] i: u32) {
        assert_eq!(b.get(0..), b.get(..));
        assert_eq!(b.get(..48), b.get(..));
        assert_eq!(b.get(0..48), b.get(..));

        assert_eq!(b.get(i..i), Bits::default());
        assert_eq!(b.get(..=i), b.get(..i + 1));
    }

    #[proptest]
    #[should_panic]
    fn get_panics_if_index_is_out_of_range(b: Bits<48>, #[strategy(48u32..)] i: u32) {
        b.get(..=i);
    }

    #[proptest]
    fn can_be_converted_into_usize(n: usize) {
        assert_eq!(n, Bits::<64>::new(n as _).into());
    }

    #[proptest]
    fn can_be_converted_into_u64(n: u64) {
        assert_eq!(n, Bits::<64>::new(n as _).into());
    }

    #[proptest]
    fn can_be_converted_into_u32(n: u32) {
        assert_eq!(n, Bits::<48>::new(n as _).into());
    }

    #[proptest]
    fn can_be_converted_into_u16(n: u16) {
        assert_eq!(n, Bits::<24>::new(n as _).into());
    }

    #[proptest]
    fn can_be_converted_into_u8(n: u8) {
        assert_eq!(n, Bits::<12>::new(n as _).into());
    }

    #[proptest]
    fn push_shifts_bits_into_collection(a: Bits<3>, b: Bits<9>, c: Bits<27>) {
        let mut bits = Bits::<39>::default();

        bits.push(a);
        bits.push(b);
        bits.push(c);

        assert_eq!(u64::from(a), bits.get(36..).into());
        assert_eq!(u64::from(b), bits.get(27..36).into());
        assert_eq!(u64::from(c), bits.get(..27).into());
    }

    #[proptest]
    fn push_discards_overflown_bits(mut a: Bits<16>, b: Bits<16>) {
        a.push(b);
        assert_eq!(a, b);
    }

    #[proptest]
    #[should_panic]
    fn push_panics_if_collection_is_too_small(mut a: Bits<12>, b: Bits<16>) {
        a.push(b);
    }

    #[proptest]
    fn pop_returns_pushed_bits(a: Bits<3>, b: Bits<9>, c: Bits<27>) {
        let mut bits = Bits::<39>::default();

        bits.push(a);
        bits.push(b);
        bits.push(c);

        assert_eq!(bits.pop(), c);
        assert_eq!(bits.pop(), b);
        assert_eq!(bits.pop(), a);
    }

    #[proptest]
    #[should_panic]
    fn pop_panics_if_collection_is_too_small(mut a: Bits<12>) {
        a.pop::<16>();
    }
}

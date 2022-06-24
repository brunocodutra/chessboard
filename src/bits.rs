use bitvec::{prelude::*, slice::BitSlice};
use derive_more::{DebugCustom, Display};
use std::ops::{Deref, DerefMut};

#[cfg(test)]
use proptest::prelude::*;

/// A fixed width collection of bits.
///
/// # Warning
///
/// Until [generic_const_exprs] is stabilized, `C` must be explicitly specified
/// such that `W` is **not** greater than `8 * C`.
///
/// [generic_const_exprs]: https://doc.rust-lang.org/beta/unstable-book/language-features/generic-const-exprs.html#generic_const_exprs
#[derive(DebugCustom, Display, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(args = u64))]
#[debug(fmt = "Bits({})", self)]
#[display(fmt = "{:b}", "self.deref()")]
pub struct Bits<const W: usize, const C: usize>(
    #[cfg_attr(test, strategy((*args..1u64.checked_shl(W as u32).unwrap_or(u64::MAX)).prop_map(|i| {
        let mut bits = BitArray::default();
        bits.store(i);
        bits
    })))]
    BitArray<[u8; C]>,
);

/// Constructs [`Bits`] from any [`BitSlice`].
///
/// # Panics
///
/// Panics if the [`BitSlice`] is narrower than `W`.
impl<T: BitStore, const W: usize, const C: usize> From<&BitSlice<T>> for Bits<W, C> {
    fn from(slice: &BitSlice<T>) -> Self {
        debug_assert!(slice[W..].not_any());
        let mut bits = Self::default();
        bits.clone_from_bitslice(&slice[..W]);
        bits
    }
}

impl<const W: usize, const C: usize> Deref for Bits<W, C> {
    type Target = BitSlice<u8>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        debug_assert!(self.0[W..].not_any());
        &self.0[..W]
    }
}

impl<const W: usize, const C: usize> DerefMut for Bits<W, C> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(self.0[W..].not_any());
        &mut self.0[..W]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn can_be_converted_from_wider_sequence_of_raw_bits(b: Bits<12, 4>) {
        assert_eq!(Bits::<12, 2>::from(&*b).load::<u32>(), b.load::<u32>());
    }

    #[proptest]
    #[should_panic]
    fn converting_from_narrower_sequence_of_raw_bits_panics(b: Bits<8, 1>) {
        let _: Bits<12, 2> = (&*b).into();
    }

    #[proptest]
    fn bits_has_fixed_width(b: Bits<12, 2>) {
        assert_eq!(b.len(), 12);
    }

    #[proptest]
    fn bits_can_be_mutated(mut b: Bits<12, 2>) {
        b.fill(true);
        assert_eq!(b.load::<u32>(), (1 << 12) - 1);
    }
}

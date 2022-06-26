use crate::Register;
use bitvec::{mem::BitRegister, prelude::*, slice::BitSlice};
use derive_more::{DebugCustom, Display};
use std::ops::{Deref, DerefMut};

#[cfg(test)]
use proptest::prelude::*;

/// A fixed width collection of bits.
#[derive(DebugCustom, Display, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(args = T, bound(std::ops::RangeInclusive<T>: Strategy<Value = T>)))]
#[debug(fmt = "Bits({})", self)]
#[display(fmt = "{:b}", "self.deref()")]
#[repr(transparent)]
pub struct Bits<T: BitStore + BitRegister, const W: usize>(
    #[cfg_attr(test, strategy(*args..=T::ALL >> (T::BITS as usize - W)))] T,
);

impl<T: BitStore + BitRegister, const W: usize> Register for Bits<T, W> {
    const WIDTH: usize = W;
}

/// Constructs [`Bits`] from any [`BitSlice`].
///
/// # Panics
///
/// Panics if the input is narrower than `W`.
impl<T: BitStore + BitRegister, U: BitStore, const W: usize> From<&BitSlice<U>> for Bits<T, W> {
    fn from(slice: &BitSlice<U>) -> Self {
        debug_assert!(slice[W..].not_any());
        Bits(slice.load())
    }
}

impl<T: BitStore + BitRegister, const W: usize> Deref for Bits<T, W> {
    type Target = BitSlice<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        debug_assert!(self.0.view_bits::<Lsb0>()[W..].not_any());
        &self.0.view_bits()[..W]
    }
}

impl<T: BitStore + BitRegister, const W: usize> DerefMut for Bits<T, W> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(self.0.view_bits::<Lsb0>()[W..].not_any());
        &mut self.0.view_bits_mut()[..W]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn can_be_converted_from_wider_sequence_of_raw_bits(b: Bits<u32, 12>) {
        assert_eq!(Bits::<u16, 12>::from(&*b).load::<u32>(), b.load::<u32>());
    }

    #[proptest]
    #[should_panic]
    fn converting_from_narrower_sequence_of_raw_bits_panics(b: Bits<u8, 8>) {
        let _: Bits<u16, 12> = (&*b).into();
    }

    #[proptest]
    fn bits_has_fixed_width(b: Bits<u16, 12>) {
        assert_eq!(b.len(), 12);
    }

    #[proptest]
    fn bits_can_be_mutated(mut b: Bits<u16, 12>) {
        b.fill(true);
        assert_eq!(b.load::<u32>(), (1 << 12) - 1);
    }
}

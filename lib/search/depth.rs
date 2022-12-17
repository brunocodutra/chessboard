use crate::util::{Binary, Bits};
use derive_more::{Display, Error};
use num_traits::{clamp, AsPrimitive};
use std::convert::Infallible;
use test_strategy::Arbitrary;

#[derive(Debug, Display, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary)]
#[display(fmt = "{}", _0)]
pub struct Depth(#[strategy(Self::MIN.get()..Self::MAX.get())] u8);

impl Depth {
    pub const ZERO: Self = Depth(0);
    pub const MIN: Self = Self::ZERO;

    #[cfg(not(test))]
    pub const MAX: Self = Depth(31);

    #[cfg(test)]
    pub const MAX: Self = Depth(3);

    /// Constructs [`Depth`] from a raw number.
    ///
    /// # Panics
    ///
    /// Panics if `d` is outside of the bounds.
    #[inline]
    pub fn new(d: u8) -> Self {
        d.try_into().unwrap()
    }

    /// Returns the raw depth.
    #[inline]
    pub fn get(&self) -> u8 {
        self.0
    }

    /// Safely constructs [`Depth`] from a raw number through saturation.
    #[inline]
    pub fn saturate<T: AsPrimitive<u8> + From<u8> + PartialOrd>(i: T) -> Self {
        Depth(clamp(i, Self::MIN.get().into(), Self::MAX.get().into()).as_())
    }
}

/// The reason why converting [`Depth`] from an integer failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(
    fmt = "expected integer in the range `({}..={})`",
    Depth::MIN,
    Depth::MAX
)]
pub struct DepthOutOfRange;

impl TryFrom<u8> for Depth {
    type Error = DepthOutOfRange;

    #[inline]
    fn try_from(n: u8) -> Result<Self, Self::Error> {
        if (Self::MIN.get()..=Self::MAX.get()).contains(&n) {
            Ok(Depth(n))
        } else {
            Err(DepthOutOfRange)
        }
    }
}

impl Binary for Depth {
    type Bits = Bits<u8, 5>;
    type Error = Infallible;

    fn encode(&self) -> Self::Bits {
        Bits::new(self.get())
    }

    fn decode(bits: Self::Bits) -> Result<Self, Self::Error> {
        Ok(Depth::new(bits.get()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn new_accepts_numbers_within_bounds(#[strategy(Depth::MIN.get()..=Depth::MAX.get())] d: u8) {
        assert_eq!(Depth::new(d), Depth(d));
    }

    #[proptest]
    #[should_panic]
    fn new_panics_if_number_greater_than_max(#[strategy(Depth::MAX.get() + 1..)] d: u8) {
        Depth::new(d);
    }

    #[proptest]
    fn saturate_preserves_numbers_within_bounds(
        #[strategy(Depth::MIN.get()..=Depth::MAX.get())] n: u8,
    ) {
        assert_eq!(Depth::saturate(n), Depth(n));
    }

    #[proptest]
    fn saturate_caps_if_numbers_greater_than_max(#[strategy(Depth::MAX.get() + 1..)] n: u8) {
        assert_eq!(Depth::saturate(n), Depth::MAX);
    }

    #[proptest]
    fn get_returns_raw_depth(d: Depth) {
        assert_eq!(d.get(), d.0);
    }

    #[proptest]
    fn decoding_encoded_depth_is_an_identity(d: Depth) {
        assert_eq!(Binary::decode(d.encode()), Ok(d));
    }
}

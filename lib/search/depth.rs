use crate::util::{Binary, Bits, Bounds, Saturating};

pub struct DepthBounds;

impl Bounds for DepthBounds {
    type Integer = i8;

    const LOWER: Self::Integer = 0;

    #[cfg(not(test))]
    const UPPER: Self::Integer = 31;

    #[cfg(test)]
    const UPPER: Self::Integer = 3;
}

/// The search depth.
pub type Depth = Saturating<DepthBounds>;

impl Binary for Depth {
    type Bits = Bits<u8, 5>;

    #[inline(always)]
    fn encode(&self) -> Self::Bits {
        Bits::new(self.get() as _)
    }

    #[inline(always)]
    fn decode(bits: Self::Bits) -> Self {
        Depth::new(bits.get() as _)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn decoding_encoded_depth_is_an_identity(d: Depth) {
        assert_eq!(Depth::decode(d.encode()), d);
    }
}

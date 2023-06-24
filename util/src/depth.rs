use crate::{Binary, Bits, Bounds, Saturating};
use std::{convert::Infallible, fmt};

pub struct DepthBounds;

impl Bounds for DepthBounds {
    type Integer = u8;
    const LOWER: Self::Integer = 0;
    const UPPER: Self::Integer = 31;
}

/// The search depth.
pub type Depth = Saturating<DepthBounds>;

impl Binary for Depth {
    type Bits = Bits<u8, 5>;
    type Error = Infallible;

    #[inline]
    fn encode(&self) -> Self::Bits {
        Bits::new(self.get())
    }

    #[inline]
    fn decode(bits: Self::Bits) -> Result<Self, Self::Error> {
        Ok(Depth::new(bits.get()))
    }
}

impl fmt::Display for Depth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn decoding_encoded_depth_is_an_identity(d: Depth) {
        assert_eq!(Binary::decode(d.encode()), Ok(d));
    }
}

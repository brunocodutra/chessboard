use crate::util::{Binary, Bits, Bounds, Saturating};
use std::convert::Infallible;

pub struct DepthBounds;

impl Bounds<u8> for DepthBounds {
    const LOWER: u8 = 0;

    #[cfg(not(test))]
    const UPPER: u8 = 31;

    #[cfg(test)]
    const UPPER: u8 = 3;
}

pub type Depth = Saturating<u8, DepthBounds>;

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
    fn decoding_encoded_depth_is_an_identity(d: Depth) {
        assert_eq!(Binary::decode(d.encode()), Ok(d));
    }
}

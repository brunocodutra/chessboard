use crate::util::{Binary, Bits, Saturating};
use std::convert::Infallible;

#[cfg(not(test))]
pub type Depth = Saturating<u8, 0, 31>;

#[cfg(test)]
pub type Depth = Saturating<u8, 0, 3>;

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

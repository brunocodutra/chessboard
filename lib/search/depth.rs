use crate::util::{Binary, Bits, Saturate};
use derive_more::{Display, Error, Into};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use test_strategy::Arbitrary;

#[derive(
    Debug,
    Display,
    Default,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Arbitrary,
    Into,
    Serialize,
    Deserialize,
)]
#[serde(into = "u8", try_from = "u8")]
pub struct Depth(#[strategy(Self::MIN.get()..=Self::MAX.get())] u8);

impl Saturate for Depth {
    type Primitive = u8;

    const ZERO: Self = Depth(0);

    #[cfg(not(test))]
    const MAX: Self = Depth(31);

    #[cfg(test)]
    const MAX: Self = Depth(3);

    #[inline]
    fn new(i: Self::Primitive) -> Self {
        i.try_into().unwrap()
    }

    #[inline]
    fn get(&self) -> Self::Primitive {
        self.0
    }
}

#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(fmt = "expected integer in the range `({}..={})`", Depth::MIN.get(), Depth::MAX.get())]
pub struct DepthOutOfRange;

impl TryFrom<u8> for Depth {
    type Error = DepthOutOfRange;

    #[inline]
    fn try_from(i: u8) -> Result<Self, Self::Error> {
        if (Self::MIN.get()..=Self::MAX.get()).contains(&i) {
            Ok(Depth(i))
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
    fn serialization_is_transparent(d: Depth) {
        assert_eq!(ron::ser::to_string(&d), ron::ser::to_string(&d.get()));
    }

    #[proptest]
    fn deserializing_succeeds_if_within_bounds(d: Depth) {
        assert_eq!(ron::de::from_str(&d.to_string()), Ok(d));
    }

    #[proptest]
    fn deserializing_fails_if_greater_than_max(#[strategy(Depth::MAX.get() as i32 + 1..)] i: i32) {
        assert!(ron::de::from_str::<Depth>(&i.to_string()).is_err());
    }

    #[proptest]
    fn deserializing_fails_if_smaller_than_max(#[strategy(..-1)] i: i32) {
        assert!(ron::de::from_str::<Depth>(&i.to_string()).is_err());
    }

    #[proptest]
    fn decoding_encoded_depth_is_an_identity(d: Depth) {
        assert_eq!(Binary::decode(d.encode()), Ok(d));
    }
}

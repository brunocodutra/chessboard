use crate::util::{Binary, Bits, Bounds, Saturating};
use derive_more::{Display, Error};
use test_strategy::Arbitrary;

pub struct ValueBounds;

impl Bounds for ValueBounds {
    type Integer = i16;
    const LOWER: Self::Integer = -Self::UPPER;
    const UPPER: Self::Integer = 4095;
}

pub type Value = Saturating<ValueBounds>;

/// The reason why decoding [`Value`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "not a valid Value")]
pub struct DecodeValueError;

impl Binary for Value {
    type Bits = Bits<u16, 13>;
    type Error = DecodeValueError;

    fn encode(&self) -> Self::Bits {
        Bits::new((self.get() - ValueBounds::LOWER) as _)
    }

    fn decode(bits: Self::Bits) -> Result<Self, Self::Error> {
        if bits == !Bits::default() {
            Err(DecodeValueError)
        } else {
            Ok(Value::new(bits.get() as i16 + ValueBounds::LOWER))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn decoding_encoded_value_is_an_identity(v: Value) {
        assert_eq!(Binary::decode(v.encode()), Ok(v));
    }

    #[proptest]
    fn decoding_value_fails_for_invalid_bits() {
        let b = !<Value as Binary>::Bits::default();
        assert_eq!(Value::decode(b), Err(DecodeValueError));
    }
}

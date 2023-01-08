use crate::util::{Binary, Bits, Saturating};
use derive_more::{Display, Error};
use test_strategy::Arbitrary;

pub type Value = Saturating<i16, -4095, 4095>;

/// The reason why decoding [`Value`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "not a valid Value")]
pub struct DecodeValueError;

impl Binary for Value {
    type Bits = Bits<u16, 13>;
    type Error = DecodeValueError;

    fn encode(&self) -> Self::Bits {
        Bits::new((self.get() - Self::lower().get()) as _)
    }

    fn decode(bits: Self::Bits) -> Result<Self, Self::Error> {
        if bits == !Bits::default() {
            Err(DecodeValueError)
        } else {
            Ok(Value::new(bits.get() as i16 + Self::lower().get()))
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

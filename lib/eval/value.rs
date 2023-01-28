use crate::util::{Binary, Bits, Saturate};
use derive_more::{Display, Error, Neg};
use test_strategy::Arbitrary;

#[derive(
    Debug, Display, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary, Neg,
)]
pub struct Value(#[strategy(Self::MIN.get()..=Self::MAX.get())] i16);

impl Saturate for Value {
    type Primitive = i16;

    const ZERO: Self = Value(0);
    const MIN: Self = Value(-4095);
    const MAX: Self = Value(4095);

    #[inline]
    fn new(i: Self::Primitive) -> Self {
        assert!((Self::MIN.get()..=Self::MAX.get()).contains(&i));
        Value(i)
    }

    #[inline]
    fn get(&self) -> Self::Primitive {
        self.0
    }
}

/// The reason why decoding [`Value`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "not a valid Value")]
pub struct DecodeValueError;

impl Binary for Value {
    type Bits = Bits<u16, 13>;
    type Error = DecodeValueError;

    fn encode(&self) -> Self::Bits {
        Bits::new((self.get() - Self::MIN.get()) as _)
    }

    fn decode(bits: Self::Bits) -> Result<Self, Self::Error> {
        if bits == !Bits::default() {
            Err(DecodeValueError)
        } else {
            Ok(Value::new(bits.get() as i16 + Self::MIN.get()))
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

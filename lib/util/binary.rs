use crate::util::Bits;
use num_traits::{PrimInt, Unsigned};
use std::{convert::Infallible, fmt::Debug};

/// Trait for types that can be encoded to binary.
pub trait Binary: Sized {
    /// A fixed width collection of bits.
    type Bits;

    /// The reason why decoding failed.
    type Error;

    /// Encodes `Self` to its binary representation.
    fn encode(&self) -> Self::Bits;

    /// Decodes `Self` from its binary representation.
    fn decode(bits: Self::Bits) -> Result<Self, Self::Error>;
}

impl<T: PrimInt + Unsigned, const W: u32> Binary for Bits<T, W> {
    type Bits = Self;
    type Error = Infallible;

    fn encode(&self) -> Self::Bits {
        *self
    }

    fn decode(bits: Self::Bits) -> Result<Self, Self::Error> {
        Ok(bits)
    }
}

impl<T> Binary for Option<T>
where
    T: Binary,
    T::Bits: Default + Debug + Eq + PartialEq,
{
    type Bits = T::Bits;
    type Error = T::Error;

    fn encode(&self) -> Self::Bits {
        match self {
            None => T::Bits::default(),
            Some(t) => {
                let bits = t.encode();
                debug_assert_ne!(bits, T::Bits::default());
                bits
            }
        }
    }

    fn decode(bits: Self::Bits) -> Result<Self, Self::Error> {
        if bits == T::Bits::default() {
            Ok(None)
        } else {
            Ok(Some(T::decode(bits)?))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn encoding_bits_is_an_identity(b: Bits<u8, 6>) {
        assert_eq!(b.encode(), b);
    }

    #[proptest]
    fn decoding_bits_is_an_identity(b: Bits<u8, 6>) {
        assert_eq!(Binary::decode(b), Ok(b));
    }

    #[proptest]
    fn decoding_encoded_optional_is_an_identity(
        #[filter(#o != Some(Bits::default()))] o: Option<Bits<u8, 6>>,
    ) {
        assert_eq!(Binary::decode(o.encode()), Ok(o));
    }

    #[proptest]
    #[should_panic]
    fn encoding_panics_on_aliasing() {
        Some(Bits::<u8, 6>::default()).encode();
    }
}

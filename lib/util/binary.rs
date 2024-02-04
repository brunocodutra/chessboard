use crate::util::Bits;
use num_traits::{PrimInt, Unsigned};
use std::fmt::Debug;

/// Trait for types that can be encoded to binary.
pub trait Binary: Sized {
    /// A fixed width collection of bits.
    type Bits;

    /// Encodes `Self` to its binary representation.
    fn encode(&self) -> Self::Bits;

    /// Decodes `Self` from its binary representation.
    fn decode(bits: Self::Bits) -> Self;
}

impl<T: PrimInt + Unsigned, const W: u32> Binary for Bits<T, W> {
    type Bits = Self;

    #[inline(always)]
    fn encode(&self) -> Self::Bits {
        *self
    }

    #[inline(always)]
    fn decode(bits: Self::Bits) -> Self {
        bits
    }
}

impl<T> Binary for Option<T>
where
    T: Binary,
    T::Bits: Default + Debug + Eq + PartialEq,
{
    type Bits = T::Bits;

    #[inline(always)]
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

    #[inline(always)]
    fn decode(bits: Self::Bits) -> Self {
        if bits == T::Bits::default() {
            None
        } else {
            Some(T::decode(bits))
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
        assert_eq!(Bits::decode(b), b);
    }

    #[proptest]
    fn decoding_encoded_optional_is_an_identity(
        #[filter(#o != Some(Bits::default()))] o: Option<Bits<u8, 6>>,
    ) {
        assert_eq!(Option::decode(o.encode()), o);
    }

    #[proptest]
    #[should_panic]
    fn encoding_panics_on_aliasing() {
        Some(Bits::<u8, 6>::default()).encode();
    }
}

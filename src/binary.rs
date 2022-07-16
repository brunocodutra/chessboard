use crate::Register;
use std::convert::Infallible;

/// Trait for types that can be encoded to binary.
pub trait Binary: Sized {
    /// A fixed width collection of bits.
    type Register: Register;

    /// The reason why decoding failed.
    type Error;

    /// Encodes `Self` to its binary representation.
    fn encode(&self) -> Self::Register;

    /// Decodes `Self` from its binary representation.
    fn decode(register: Self::Register) -> Result<Self, Self::Error>;
}

impl<T: Register> Binary for T {
    type Register = Self;
    type Error = Infallible;

    fn encode(&self) -> Self::Register {
        *self
    }

    fn decode(register: Self::Register) -> Result<Self, Self::Error> {
        Ok(register)
    }
}

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

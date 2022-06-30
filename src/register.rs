use std::mem::size_of;

/// Trait for fixed width collection of bits.
pub trait Register: Copy {
    /// How many bits this register contains.
    const WIDTH: usize = Self::SIZE << 3;

    /// How many bytes this register contains.
    const SIZE: usize = size_of::<Self>();
}

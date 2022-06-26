/// Trait for fixed width collection of bits.
pub trait Register {
    /// How many bits this register contains.
    const WIDTH: usize;
}

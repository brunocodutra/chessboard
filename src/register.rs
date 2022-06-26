/// Trait for fixed width collection of bits.
pub trait Register: Copy {
    /// How many bits this register contains.
    const WIDTH: usize;
}

use bitvec::{mem::BitRegister, store::BitStore};

/// Trait for fixed width collection of bits.
pub trait Register: Copy {
    /// How many bits this register contains.
    const WIDTH: usize;
}

impl<T: BitStore + BitRegister> Register for T {
    const WIDTH: usize = T::BITS as usize;
}

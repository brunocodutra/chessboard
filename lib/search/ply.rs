use crate::util::{Bounds, Saturating};

pub struct PlyBounds;

impl Bounds for PlyBounds {
    type Integer = i8;
    const LOWER: Self::Integer = -Self::UPPER;

    #[cfg(not(test))]
    const UPPER: Self::Integer = 95;

    #[cfg(test)]
    const UPPER: Self::Integer = 3;
}

/// The number of half-moves played.
pub type Ply = Saturating<PlyBounds>;

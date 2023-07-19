use crate::util::{Bounds, Saturating};

pub struct PlyBounds;

impl Bounds for PlyBounds {
    type Integer = i8;
    const LOWER: Self::Integer = -Self::UPPER;
    const UPPER: Self::Integer = 127;
}

/// The number of half-moves played.
pub type Ply = Saturating<PlyBounds>;

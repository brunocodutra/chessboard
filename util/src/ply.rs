use crate::{Bounds, Saturating};

pub struct PlyBounds;

impl Bounds for PlyBounds {
    type Integer = i16;
    const LOWER: Self::Integer = -Self::UPPER;
    const UPPER: Self::Integer = 383;
}

/// The number of half-moves played.
pub type Ply = Saturating<PlyBounds>;

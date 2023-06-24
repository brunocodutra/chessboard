use super::{Bounds, PlyBounds, Saturating, ScoreBounds};

pub struct ValueBounds;

impl Bounds for ValueBounds {
    type Integer = i16;
    const LOWER: Self::Integer = -Self::UPPER;
    const UPPER: Self::Integer = ScoreBounds::UPPER - PlyBounds::UPPER as i16 - 1;
}

/// A [`Position`]'s static evaluation.
pub type Value = Saturating<ValueBounds>;

use crate::{Bounds, PlyBounds, Saturating, Score, ScoreBounds};
use std::fmt;

pub struct ValueBounds;

impl Bounds for ValueBounds {
    type Integer = i16;
    const LOWER: Self::Integer = -Self::UPPER;
    const UPPER: Self::Integer = ScoreBounds::UPPER - PlyBounds::UPPER - 1;
}

/// A position's static evaluation.
pub type Value = Saturating<ValueBounds>;

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&Score::saturate(self.get()), f)
    }
}

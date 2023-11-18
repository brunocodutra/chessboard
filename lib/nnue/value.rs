use crate::util::{Bounds, Saturating};

pub struct ValueBounds;

impl Bounds for ValueBounds {
    type Integer = i16;
    const LOWER: Self::Integer = -Self::UPPER;
    const UPPER: Self::Integer = 8000;
}

/// A position's static evaluation.
pub type Value = Saturating<ValueBounds>;

use crate::search::Score;
use crate::util::{Bounds, Saturating};
use std::fmt;

pub struct ValueBounds;

impl Bounds for ValueBounds {
    type Integer = i16;
    const LOWER: Self::Integer = -Self::UPPER;
    const UPPER: Self::Integer = 8000;
}

/// A position's static evaluation.
pub type Value = Saturating<ValueBounds>;

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Score as fmt::Display>::fmt(&self.cast(), f)
    }
}

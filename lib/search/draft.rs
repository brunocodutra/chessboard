use crate::util::{Bounds, Saturating};

pub struct DraftBounds;

impl Bounds for DraftBounds {
    type Integer = i8;
    const LOWER: Self::Integer = -Self::UPPER;

    #[cfg(not(test))]
    const UPPER: Self::Integer = 31;

    #[cfg(test)]
    const UPPER: Self::Integer = 3;
}

pub type Draft = Saturating<DraftBounds>;

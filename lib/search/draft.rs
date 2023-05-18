use crate::util::{Bounds, Saturating};

pub struct DraftBounds;

impl Bounds<i8> for DraftBounds {
    const LOWER: i8 = -Self::UPPER;

    #[cfg(not(test))]
    const UPPER: i8 = 31;

    #[cfg(test)]
    const UPPER: i8 = 3;
}

pub type Draft = Saturating<i8, DraftBounds>;

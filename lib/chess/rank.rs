use crate::util::Enum;
use cozy_chess as cc;
use derive_more::Display;
use std::ops::{RangeInclusive, Sub};

/// A row on the chess board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(u8)]
pub enum Rank {
    #[display("1")]
    First,
    #[display("2")]
    Second,
    #[display("3")]
    Third,
    #[display("4")]
    Fourth,
    #[display("5")]
    Fifth,
    #[display("6")]
    Sixth,
    #[display("7")]
    Seventh,
    #[display("8")]
    Eighth,
}

unsafe impl Enum for Rank {
    const RANGE: RangeInclusive<Self> = Rank::First..=Rank::Eighth;

    #[inline(always)]
    fn repr(&self) -> u8 {
        *self as _
    }
}

impl Sub for Rank {
    type Output = i8;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        self as i8 - rhs as i8
    }
}

#[doc(hidden)]
impl From<cc::Rank> for Rank {
    #[inline(always)]
    fn from(r: cc::Rank) -> Self {
        Rank::from_repr(r as _)
    }
}

#[doc(hidden)]
impl From<Rank> for cc::Rank {
    #[inline(always)]
    fn from(r: Rank) -> Self {
        cc::Rank::index_const(r as _)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;
    use test_strategy::proptest;

    #[proptest]
    fn rank_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<Rank>>(), size_of::<Rank>());
    }

    #[proptest]
    fn subtracting_ranks_gives_distance(a: Rank, b: Rank) {
        assert_eq!(a - b, a as i8 - b as i8);
    }

    #[proptest]
    fn rank_has_an_equivalent_cozy_chess_representation(r: Rank) {
        assert_eq!(Rank::from(cc::Rank::from(r)), r);
    }
}

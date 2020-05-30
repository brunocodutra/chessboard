use crate::foreign;
use derive_more::Display;

/// A row of the board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Rank {
    #[display(fmt = "1")]
    First,
    #[display(fmt = "2")]
    Second,
    #[display(fmt = "3")]
    Third,
    #[display(fmt = "4")]
    Fourth,
    #[display(fmt = "5")]
    Fifth,
    #[display(fmt = "6")]
    Sixth,
    #[display(fmt = "7")]
    Seventh,
    #[display(fmt = "8")]
    Eighth,
}

impl Rank {
    pub const VARIANTS: &'static [Rank] = &[
        Rank::First,
        Rank::Second,
        Rank::Third,
        Rank::Fourth,
        Rank::Fifth,
        Rank::Sixth,
        Rank::Seventh,
        Rank::Eighth,
    ];
}

impl From<foreign::Rank> for Rank {
    fn from(r: foreign::Rank) -> Self {
        Rank::VARIANTS[r.to_index()]
    }
}

impl Into<foreign::Rank> for Rank {
    fn into(self: Self) -> foreign::Rank {
        foreign::Rank::from_index(self as usize)
    }
}

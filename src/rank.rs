use crate::foreign;

/// A row of the board.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Rank {
    First,
    Second,
    Third,
    Fourth,
    Fifth,
    Sixth,
    Seventh,
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

    pub fn to_str(self) -> &'static str {
        use Rank::*;
        match self {
            First => &"1",
            Second => &"2",
            Third => &"3",
            Fourth => &"4",
            Fifth => &"5",
            Sixth => &"6",
            Seventh => &"7",
            Eighth => &"8",
        }
    }
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

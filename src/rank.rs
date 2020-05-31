use crate::foreign;
use derive_more::{Display, Error};
use std::str::FromStr;

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

/// The reason why a player action was rejected.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Error)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(
    fmt = "unable to parse rank, expected a sigle digit in the range [{}-{}]",
    "Rank::First",
    "Rank::Eighth"
)]
pub struct ParseRankError;

impl FromStr for Rank {
    type Err = ParseRankError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" => Ok(Rank::First),
            "2" => Ok(Rank::Second),
            "3" => Ok(Rank::Third),
            "4" => Ok(Rank::Fourth),
            "5" => Ok(Rank::Fifth),
            "6" => Ok(Rank::Sixth),
            "7" => Ok(Rank::Seventh),
            "8" => Ok(Rank::Eighth),
            _ => Err(ParseRankError),
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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn parsing_printed_rank_is_an_identity(r: Rank) {
            assert_eq!(r.to_string().parse(), Ok(r));
        }

        #[test]
        fn parsing_rank_fails_except_for_single_digits_between_1_and_8(r in "[^1-8]*") {
            assert_eq!(r.parse::<Rank>(), Err(ParseRankError));
        }
    }
}

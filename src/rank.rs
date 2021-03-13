use crate::foreign;
use derive_more::{Display, Error};
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;
use tracing::instrument;

/// Denotes a row on the chess board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[repr(u8)]
pub enum Rank {
    #[display(fmt = "1")]
    First = 1,
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

/// The reason why parsing [`Rank`] failed.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Error)]
#[display(
    fmt = "unable to parse rank; expected digit in the range `[{}-{}]`",
    Rank::First,
    Rank::Eighth
)]
pub struct ParseRankError;

impl FromStr for Rank {
    type Err = ParseRankError;

    #[instrument(err)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u32>().map_err(|_| ParseRankError)?.try_into()
    }
}

impl TryFrom<u32> for Rank {
    type Error = ParseRankError;

    #[instrument(err)]
    fn try_from(n: u32) -> Result<Self, Self::Error> {
        match n {
            1 => Ok(Rank::First),
            2 => Ok(Rank::Second),
            3 => Ok(Rank::Third),
            4 => Ok(Rank::Fourth),
            5 => Ok(Rank::Fifth),
            6 => Ok(Rank::Sixth),
            7 => Ok(Rank::Seventh),
            8 => Ok(Rank::Eighth),
            _ => Err(ParseRankError),
        }
    }
}

impl From<Rank> for u32 {
    fn from(r: Rank) -> Self {
        r as u32
    }
}

impl From<foreign::Rank> for Rank {
    fn from(r: foreign::Rank) -> Self {
        Rank::VARIANTS[r.to_index()]
    }
}

impl Into<foreign::Rank> for Rank {
    fn into(self) -> foreign::Rank {
        foreign::Rank::from_index(self as usize - Self::First as usize)
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
        fn parsing_rank_succeeds_for_digit_between_1_and_8(n in 1..=8u32) {
            assert_eq!(n.to_string().parse::<Rank>(), n.try_into());
        }

        #[test]
        fn parsing_rank_fails_except_for_digit_between_1_and_8(s in "[^1-8]*|[1-8]{2,}") {
            assert_eq!(s.parse::<Rank>(), Err(ParseRankError));
        }

        #[test]
        fn rank_can_be_converted_into_u32(r: Rank) {
            assert_eq!(u32::from(r).try_into(), Ok(r));
        }
    }
}

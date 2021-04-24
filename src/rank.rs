use derive_more::{Display, Error, From};
use shakmaty as sm;
use std::convert::{TryFrom, TryInto};
use std::{num::ParseIntError, str::FromStr};
use tracing::instrument;

/// Denotes a row on the chess board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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

    /// This rank's index in the range (0..=7).
    pub fn index(&self) -> usize {
        (*self).into()
    }
}

/// The reason why parsing [`Rank`] failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error, From)]
#[display(fmt = "unable to parse rank")]
pub enum ParseRankError {
    ParseIntError(ParseIntError),
    OutOfRange(RankOutOfRange),
}

impl FromStr for Rank {
    type Err = ParseRankError;

    #[instrument(level = "trace", err)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse::<u32>()?.try_into()?)
    }
}

/// The reason why converting [`Rank`] from index failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(
    fmt = "expected digit in the range `({}..={})`",
    Rank::First,
    Rank::Eighth
)]
pub struct RankOutOfRange;

impl TryFrom<u32> for Rank {
    type Error = RankOutOfRange;

    #[instrument(level = "trace", err)]
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
            _ => Err(RankOutOfRange),
        }
    }
}

impl From<Rank> for u32 {
    fn from(r: Rank) -> Self {
        r as u32
    }
}

/// The reason why converting [`Rank`] from index failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(fmt = "expected integer in the range `(0..=7)`")]
pub struct RankIndexOutOfRange;

impl TryFrom<usize> for Rank {
    type Error = RankIndexOutOfRange;

    #[instrument(level = "trace", err)]
    fn try_from(i: usize) -> Result<Self, Self::Error> {
        Self::VARIANTS.get(i).copied().ok_or(RankIndexOutOfRange)
    }
}

impl From<Rank> for usize {
    fn from(f: Rank) -> usize {
        f as usize - Rank::First as usize
    }
}

#[doc(hidden)]
impl From<sm::Rank> for Rank {
    fn from(r: sm::Rank) -> Self {
        usize::from(r).try_into().unwrap()
    }
}

#[doc(hidden)]
impl From<Rank> for sm::Rank {
    fn from(r: Rank) -> Self {
        sm::Rank::new(r.index() as u32)
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
            assert_eq!(n.to_string().parse::<Rank>(), Ok(n.try_into().unwrap()));
        }

        #[test]
        fn parsing_rank_fails_for_strings_representing_invalid_integers(s in "[^0-9]*") {
            use ParseRankError::*;
            assert_eq!(s.parse::<Rank>(), Err(ParseIntError(s.parse::<u32>().unwrap_err())));
        }

        #[test]
        fn parsing_rank_fails_for_integers_out_of_range(n: u32) {
            prop_assume!(!(1..=8).contains(&n));
            use ParseRankError::*;
            assert_eq!(n.to_string().parse::<Rank>(), Err(OutOfRange(Rank::try_from(n).unwrap_err())));
        }

        #[test]
        fn rank_can_be_converted_into_u32(r: Rank) {
            assert_eq!(u32::from(r).try_into(), Ok(r));
        }

        #[test]
        fn rank_has_an_index(f: Rank) {
            assert_eq!(f.index().try_into(), Ok(f));
        }

        #[test]
        fn converting_rank_from_index_out_of_range_fails(i in 8usize..) {
            assert_eq!(Rank::try_from(i), Err(RankIndexOutOfRange));
        }

        #[test]
        fn rank_is_ordered_by_index(a: Rank, b: Rank) {
            assert_eq!(a < b, a.index() < b.index());
        }

        #[test]
        fn rank_has_an_equivalent_shakmaty_representation(r: Rank) {
            assert_eq!(Rank::from(sm::Rank::from(r)), r);
        }
    }
}

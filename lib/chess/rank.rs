use crate::chess::{Bitboard, Perspective};
use crate::util::Integer;
use derive_more::{Display, Error};
use std::{ops::Sub, str::FromStr};

/// A row on the chess board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(i8)]
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

impl Rank {
    /// Returns a [`Bitboard`] that only contains this rank.
    #[inline(always)]
    pub fn bitboard(self) -> Bitboard {
        Bitboard::new(0x000000000000FF << (self.get() * 8))
    }
}

unsafe impl Integer for Rank {
    type Repr = i8;
    const MIN: Self::Repr = Rank::First as _;
    const MAX: Self::Repr = Rank::Eighth as _;
}

impl Perspective for Rank {
    /// This rank from the opponent's perspective.
    #[inline(always)]
    fn flip(&self) -> Self {
        Self::new(self.get() ^ Self::MAX)
    }
}

impl Sub for Rank {
    type Output = i8;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        self.get() - rhs.get()
    }
}

/// The reason why parsing [`Rank`] failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(
    "failed to parse rank, expected digit in the range `({}..={})`",
    Rank::First,
    Rank::Eighth
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::{File, Square};
    use std::mem::size_of;
    use test_strategy::proptest;

    #[test]
    fn rank_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<Rank>>(), size_of::<Rank>());
    }

    #[proptest]
    fn subtracting_ranks_returns_distance(a: Rank, b: Rank) {
        assert_eq!(a - b, a.get() - b.get());
    }

    #[proptest]
    fn flipping_rank_returns_its_complement(r: Rank) {
        assert_eq!(r.flip().get(), Rank::MAX - r.get());
    }

    #[proptest]
    fn rank_has_an_equivalent_bitboard(r: Rank) {
        assert_eq!(
            Vec::from_iter(r.bitboard()),
            Vec::from_iter(File::iter().map(|f| Square::new(f, r)))
        );
    }

    #[proptest]
    fn parsing_printed_rank_is_an_identity(r: Rank) {
        assert_eq!(r.to_string().parse(), Ok(r));
    }

    #[proptest]
    fn parsing_rank_fails_if_not_digit_between_1_and_8(
        #[filter(!('1'..='8').contains(&#c))] c: char,
    ) {
        assert_eq!(c.to_string().parse::<Rank>(), Err(ParseRankError));
    }

    #[proptest]
    fn parsing_rank_fails_if_length_not_one(#[filter(#s.len() != 1)] s: String) {
        assert_eq!(s.parse::<Rank>(), Err(ParseRankError));
    }
}

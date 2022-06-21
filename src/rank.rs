use derive_more::{DebugCustom, Display, Error, From};
use shakmaty as sm;
use std::convert::{TryFrom, TryInto};
use std::{char::ParseCharError, num::TryFromIntError, ops::Sub, str::FromStr};

#[cfg(test)]
use proptest::sample::select;

/// Denotes a row on the chess board.
#[derive(DebugCustom, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug(fmt = "{}", self)]
#[display(fmt = "{}", _0)]
pub struct Rank(#[cfg_attr(test, strategy(select(sm::Rank::ALL.as_ref())))] sm::Rank);

impl Rank {
    /// Constructs [`Rank`] from index.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not in the range (0..=7).
    pub fn from_index(i: u8) -> Self {
        i.try_into().unwrap()
    }

    /// This rank's index in the range (0..=7).
    pub fn index(&self) -> u8 {
        (*self).into()
    }

    /// Returns an iterator over [`Rank`]s ordered by [index][`Rank::index`].
    pub fn iter() -> impl DoubleEndedIterator<Item = Self> + ExactSizeIterator {
        sm::Rank::ALL.into_iter().map(Rank)
    }
}

impl Sub for Rank {
    type Output = i8;

    fn sub(self, rhs: Self) -> Self::Output {
        self.index() as i8 - rhs.index() as i8
    }
}

/// The reason why parsing [`Rank`] failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error, From)]
#[display(fmt = "failed to parse rank")]
pub enum ParseRankError {
    ParseCharError(ParseCharError),
    InvalidRank(InvalidRank),
}

impl FromStr for Rank {
    type Err = ParseRankError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse::<char>()?.try_into()?)
    }
}

/// The reason why converting [`Rank`] from index failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(fmt = "expected digit in the range `('1'..='8')`")]
pub struct InvalidRank;

impl TryFrom<char> for Rank {
    type Error = InvalidRank;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        sm::Rank::from_char(c).map(Rank).ok_or(InvalidRank)
    }
}

impl From<Rank> for char {
    fn from(r: Rank) -> Self {
        r.0.char()
    }
}

/// The reason why converting [`Rank`] from index failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(fmt = "expected integer in the range `(0..=7)`")]
pub struct RankOutOfRange;

impl From<TryFromIntError> for RankOutOfRange {
    fn from(_: TryFromIntError) -> Self {
        RankOutOfRange
    }
}

impl TryFrom<u8> for Rank {
    type Error = RankOutOfRange;

    fn try_from(i: u8) -> Result<Self, Self::Error> {
        Ok(Rank(i.try_into()?))
    }
}

impl From<Rank> for u8 {
    fn from(f: Rank) -> u8 {
        f.0.into()
    }
}

#[doc(hidden)]
impl From<sm::Rank> for Rank {
    fn from(r: sm::Rank) -> Self {
        Rank(r)
    }
}

#[doc(hidden)]
impl From<Rank> for sm::Rank {
    fn from(r: Rank) -> Self {
        r.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn iter_returns_iterator_over_ranks_in_order() {
        assert_eq!(
            Rank::iter().collect::<Vec<_>>(),
            (0..=7).map(Rank::from_index).collect::<Vec<_>>()
        );
    }

    #[proptest]
    fn iter_returns_double_ended_iterator() {
        assert_eq!(
            Rank::iter().rev().collect::<Vec<_>>(),
            (0..=7).rev().map(Rank::from_index).collect::<Vec<_>>()
        );
    }

    #[proptest]
    fn iter_returns_iterator_of_exact_size() {
        assert_eq!(Rank::iter().len(), 8);
    }

    #[proptest]
    fn parsing_printed_rank_is_an_identity(r: Rank) {
        assert_eq!(r.to_string().parse(), Ok(r));
    }

    #[proptest]
    fn parsing_rank_succeeds_for_digit_between_1_and_8(#[strategy(b'1'..=b'8')] c: u8) {
        let c = char::from(c);
        assert_eq!(c.to_string().parse::<Rank>(), Ok(c.try_into()?));
    }

    #[proptest]
    fn parsing_rank_fails_for_strings_of_length_not_one(#[strategy(".{2,}?")] s: String) {
        assert_eq!(
            s.parse::<Rank>().err(),
            s.parse::<char>().err().map(Into::into)
        );
    }

    #[proptest]
    fn parsing_rank_fails_for_digits_out_of_range(#[filter(!('1'..='8').contains(&#c))] c: char) {
        assert_eq!(
            c.to_string().parse::<Rank>().err(),
            Rank::try_from(c).err().map(Into::into)
        );
    }

    #[proptest]
    fn rank_can_be_converted_to_char(r: Rank) {
        assert_eq!(char::from(r).try_into(), Ok(r));
    }

    #[proptest]
    fn converting_rank_from_digit_out_of_range_fails(#[filter(!('1'..='8').contains(&#c))] c: char) {
        assert_eq!(Rank::try_from(c), Err(InvalidRank));
    }

    #[proptest]
    fn rank_has_an_index(f: Rank) {
        assert_eq!(f.index().try_into(), Ok(f));
    }

    #[proptest]
    fn subtracting_ranks_gives_distance(a: Rank, b: Rank) {
        assert_eq!(a - b, a.index() as i8 - b.index() as i8);
    }

    #[proptest]
    fn from_index_constructs_rank_by_index(#[strategy(0u8..8)] i: u8) {
        assert_eq!(Rank::from_index(i).index(), i);
    }

    #[proptest]
    #[should_panic]
    fn from_index_panics_if_index_out_of_range(#[strategy(8u8..)] i: u8) {
        Rank::from_index(i);
    }

    #[proptest]
    fn converting_rank_from_index_out_of_range_fails(#[strategy(8u8..)] i: u8) {
        assert_eq!(Rank::try_from(i), Err(RankOutOfRange));
    }

    #[proptest]
    fn rank_is_ordered_by_index(a: Rank, b: Rank) {
        assert_eq!(a < b, a.index() < b.index());
    }

    #[proptest]
    fn rank_has_an_equivalent_shakmaty_representation(r: Rank) {
        assert_eq!(Rank::from(sm::Rank::from(r)), r);
    }
}

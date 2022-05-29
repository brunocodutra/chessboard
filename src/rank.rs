use derive_more::{DebugCustom, Display, Error, From};
use shakmaty as sm;
use std::convert::{TryFrom, TryInto};
use std::{iter::FusedIterator, num::ParseIntError, ops::Sub, str::FromStr};

/// Denotes a row on the chess board.
#[derive(DebugCustom, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(u8)]
pub enum Rank {
    #[debug(fmt = "1")]
    #[display(fmt = "1")]
    First = 1,
    #[debug(fmt = "2")]
    #[display(fmt = "2")]
    Second,
    #[debug(fmt = "3")]
    #[display(fmt = "3")]
    Third,
    #[debug(fmt = "4")]
    #[display(fmt = "4")]
    Fourth,
    #[debug(fmt = "5")]
    #[display(fmt = "5")]
    Fifth,
    #[debug(fmt = "6")]
    #[display(fmt = "6")]
    Sixth,
    #[debug(fmt = "7")]
    #[display(fmt = "7")]
    Seventh,
    #[debug(fmt = "8")]
    #[display(fmt = "8")]
    Eighth,
}

impl Rank {
    /// Constructs [`Rank`] from index.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not in the range (0..=7).
    pub fn new(i: usize) -> Self {
        i.try_into().unwrap()
    }

    /// This rank's index in the range (0..=7).
    pub fn index(&self) -> usize {
        (*self).into()
    }

    /// Returns an iterator over [`Rank`]s ordered by [index][`Rank::index`].
    pub fn iter() -> impl DoubleEndedIterator<Item = Self> + ExactSizeIterator + FusedIterator {
        (0usize..8).map(Rank::new)
    }
}

/// The number of squares between two [`Rank`]s.
impl Sub for Rank {
    type Output = isize;

    fn sub(self, rhs: Self) -> Self::Output {
        self as isize - rhs as isize
    }
}

/// The reason why parsing [`Rank`] failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error, From)]
#[display(fmt = "failed to parse rank")]
pub enum ParseRankError {
    ParseIntError(ParseIntError),
    OutOfRange(RankOutOfRange),
}

impl FromStr for Rank {
    type Err = ParseRankError;

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

    fn try_from(n: u32) -> Result<Self, Self::Error> {
        Self::iter()
            .find(|&f| u32::from(f) == n)
            .ok_or(RankOutOfRange)
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

    fn try_from(i: usize) -> Result<Self, Self::Error> {
        use Rank::*;

        [First, Second, Third, Fourth, Fifth, Sixth, Seventh, Eighth]
            .get(i)
            .copied()
            .ok_or(RankIndexOutOfRange)
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
        Rank::new(usize::from(r))
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
    use test_strategy::proptest;

    #[proptest]
    fn iter_returns_iterator_over_ranks_in_order() {
        use Rank::*;
        assert_eq!(
            Rank::iter().collect::<Vec<_>>(),
            vec![First, Second, Third, Fourth, Fifth, Sixth, Seventh, Eighth]
        );
    }

    #[proptest]
    fn iter_returns_double_ended_iterator() {
        use Rank::*;
        assert_eq!(
            Rank::iter().rev().collect::<Vec<_>>(),
            vec![Eighth, Seventh, Sixth, Fifth, Fourth, Third, Second, First]
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
    fn parsing_rank_succeeds_for_digit_between_1_and_8(#[strategy(1u32..=8)] n: u32) {
        assert_eq!(n.to_string().parse::<Rank>(), Ok(n.try_into()?));
    }

    #[proptest]
    fn parsing_rank_fails_for_strings_representing_invalid_integers(
        #[strategy("[^0-9]*")] s: String,
    ) {
        assert_eq!(
            s.parse::<Rank>().err(),
            s.parse::<u32>().err().map(Into::into)
        );
    }

    #[proptest]
    fn parsing_rank_fails_for_integers_out_of_range(#[filter(!(1..=8).contains(&#n))] n: u32) {
        assert_eq!(
            n.to_string().parse::<Rank>().err(),
            Rank::try_from(n).err().map(Into::into)
        );
    }

    #[proptest]
    fn rank_can_be_converted_into_u32(r: Rank) {
        assert_eq!(u32::from(r).try_into(), Ok(r));
    }

    #[proptest]
    fn rank_has_an_index(f: Rank) {
        assert_eq!(f.index().try_into(), Ok(f));
    }

    #[proptest]
    fn subtracting_ranks_gives_distance(a: Rank, b: Rank) {
        assert_eq!(a - b, a.index() as isize - b.index() as isize);
    }

    #[proptest]
    fn new_constructs_rank_by_index(#[strategy(0usize..=7)] i: usize) {
        assert_eq!(Rank::new(i).index(), i);
    }

    #[proptest]
    #[should_panic]
    fn new_panics_if_index_out_of_range(#[strategy(8usize..)] i: usize) {
        Rank::new(i);
    }

    #[proptest]
    fn converting_rank_from_index_out_of_range_fails(#[strategy(8usize..)] i: usize) {
        assert_eq!(Rank::try_from(i), Err(RankIndexOutOfRange));
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

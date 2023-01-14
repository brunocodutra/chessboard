use derive_more::{DebugCustom, Display, Error};
use proptest::sample::select;
use shakmaty as sm;
use std::convert::{TryFrom, TryInto};
use std::ops::Sub;
use test_strategy::Arbitrary;

/// Denotes a row on the chess board.
#[derive(DebugCustom, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary)]
#[debug(fmt = "{self}")]
#[display(fmt = "{_0}")]
pub struct Rank(#[strategy(select(sm::Rank::ALL.as_ref()))] sm::Rank);

impl Rank {
    /// Constructs [`Rank`] from index.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not in the range (0..=7).
    pub fn from_index(i: u8) -> Self {
        Rank(i.try_into().unwrap())
    }

    /// This rank's index in the range (0..=7).
    pub fn index(&self) -> u8 {
        self.0.into()
    }

    /// Returns an iterator over [`Rank`]s ordered by [index][`Rank::index`].
    pub fn iter() -> impl DoubleEndedIterator<Item = Self> + ExactSizeIterator {
        sm::Rank::ALL.into_iter().map(Rank)
    }

    /// Mirrors this rank.
    pub fn mirror(&self) -> Self {
        self.0.flip_vertical().into()
    }
}

impl Sub for Rank {
    type Output = i8;

    fn sub(self, rhs: Self) -> Self::Output {
        self.index() as i8 - rhs.index() as i8
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
    use std::mem::size_of;
    use test_strategy::proptest;

    #[proptest]
    fn rank_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<Rank>>(), size_of::<Rank>());
    }

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
    fn rank_can_be_converted_to_char(r: Rank) {
        assert_eq!(char::from(r).try_into(), Ok(r));
    }

    #[proptest]
    fn converting_rank_from_digit_out_of_range_fails(
        #[filter(!('1'..='8').contains(&#c))] c: char,
    ) {
        assert_eq!(Rank::try_from(c), Err(InvalidRank));
    }

    #[proptest]
    fn rank_has_an_index(r: Rank) {
        assert_eq!(Rank::from_index(r.index()), r);
    }

    #[proptest]
    fn rank_has_a_mirror(r: Rank) {
        assert_eq!(r.mirror().index(), 7 - r.index());
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
    fn rank_is_ordered_by_index(a: Rank, b: Rank) {
        assert_eq!(a < b, a.index() < b.index());
    }

    #[proptest]
    fn rank_has_an_equivalent_shakmaty_representation(r: Rank) {
        assert_eq!(Rank::from(sm::Rank::from(r)), r);
    }
}

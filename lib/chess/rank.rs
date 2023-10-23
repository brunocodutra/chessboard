use derive_more::Display;
use shakmaty as sm;
use std::ops::Sub;

/// Denotes a row on the chess board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(u8)]
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
    const FILES: [Self; 8] = [
        Rank::First,
        Rank::Second,
        Rank::Third,
        Rank::Fourth,
        Rank::Fifth,
        Rank::Sixth,
        Rank::Seventh,
        Rank::Eighth,
    ];

    /// Constructs [`Rank`] from index.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not in the range (0..=7).
    pub fn from_index(i: u8) -> Self {
        Self::FILES[i as usize]
    }

    /// This ranks's index in the range (0..=7).
    pub fn index(&self) -> u8 {
        *self as _
    }

    /// Returns an iterator over [`Rank`]s ordered by [index][`Rank::index`].
    pub fn iter() -> impl DoubleEndedIterator<Item = Self> + ExactSizeIterator {
        Self::FILES.into_iter()
    }

    /// Mirrors this rank.
    pub fn mirror(&self) -> Self {
        Self::from_index(Rank::Eighth as u8 - *self as u8)
    }
}

impl Sub for Rank {
    type Output = i8;

    fn sub(self, rhs: Self) -> Self::Output {
        self.index() as i8 - rhs.index() as i8
    }
}

#[doc(hidden)]
impl From<sm::Rank> for Rank {
    fn from(r: sm::Rank) -> Self {
        Rank::from_index(r as _)
    }
}

#[doc(hidden)]
impl From<Rank> for sm::Rank {
    fn from(r: Rank) -> Self {
        sm::Rank::new(r as _)
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

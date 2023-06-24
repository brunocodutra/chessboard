use arrayvec::ArrayVec;
use chess::Move;
use derive_more::{DebugCustom, Deref, Display, IntoIterator};
use proptest::{collection::vec, prelude::*};
use test_strategy::Arbitrary;

/// A sequence of moves.
#[derive(
    DebugCustom, Display, Default, Clone, Eq, PartialEq, Hash, Arbitrary, Deref, IntoIterator,
)]
#[debug(fmt = "Line({self})")]
#[display(
    fmt = "{}",
    "self.iter().map(Move::to_string).collect::<ArrayVec<_, N>>().join(\" \")"
)]
pub struct Line<const N: usize>(
    #[strategy(vec(any::<Move>(), 0..=N).prop_map(ArrayVec::from_iter))]
    #[deref(forward)]
    #[into_iterator(owned, ref, ref_mut)]
    ArrayVec<Move, N>,
);

impl<const N: usize> Line<N> {
    /// Returns an empty sequence.
    pub fn empty() -> Self {
        Self::default()
    }

    /// The number of moves in this sequence.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// If this sequence contains no move.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate over the moves in this sequence.
    #[inline]
    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

/// Create a [`Line`] from an iterator of [`Move`]s.
///
/// The sequence might be truncated if the number of moves exceeds the internal capacity.
impl<const N: usize> FromIterator<Move> for Line<N> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = Move>>(moves: I) -> Self {
        Line(moves.into_iter().take(N).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::sample::size_range;
    use test_strategy::proptest;

    #[proptest]
    fn len_returns_number_of_moves_in_the_sequence(l: Line<3>) {
        assert_eq!(l.len(), l.iter().len());
    }

    #[proptest]
    fn is_empty_returns_whether_there_are_no_moves_in_the_sequence(l: Line<3>) {
        assert_eq!(l.is_empty(), l.iter().count() == 0);
    }

    #[proptest]
    fn collects_truncated_sequence(#[any(size_range(0..=6).lift())] ms: Vec<Move>) {
        assert_eq!(
            Line::<3>::from_iter(ms.clone()),
            ms.into_iter().take(3).collect()
        );
    }
}

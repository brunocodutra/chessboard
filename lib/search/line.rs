use crate::chess::Move;
use arrayvec::ArrayVec;
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
    "self.iter().map(Move::to_string).collect::<ArrayVec<_, { Line::N }>>().join(\" \")"
)]
pub struct Line(
    #[strategy(vec(any::<Move>(), 0..=Self::N).prop_map(ArrayVec::from_iter))]
    #[deref(forward)]
    #[into_iterator(owned, ref, ref_mut)]
    ArrayVec<Move, { Line::N }>,
);

impl Line {
    #[cfg(not(test))]
    const N: usize = 16;

    #[cfg(test)]
    const N: usize = 4;

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
impl FromIterator<Move> for Line {
    #[inline]
    fn from_iter<I: IntoIterator<Item = Move>>(moves: I) -> Self {
        Line(moves.into_iter().take(Self::N).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::sample::size_range;
    use test_strategy::proptest;

    #[proptest]
    fn len_returns_number_of_moves_in_the_sequence(l: Line) {
        assert_eq!(l.len(), l.iter().len());
    }

    #[proptest]
    fn is_empty_returns_whether_there_are_no_moves_in_the_sequence(l: Line) {
        assert_eq!(l.is_empty(), l.iter().count() == 0);
    }

    #[proptest]
    fn collects_truncated_sequence(#[any(size_range(0..=2 * Line::N).lift())] ms: Vec<Move>) {
        assert_eq!(
            Line::from_iter(ms.clone()),
            ms.into_iter().take(Line::N).collect()
        );
    }
}

use crate::chess::{File, Rank, Square};
use derive_more::{BitAnd, BitAndAssign, BitOr, BitOrAssign, DebugCustom, Not};
use proptest::prelude::*;
use shakmaty as sm;
use test_strategy::Arbitrary;

/// A set of squares on a chess board.
#[derive(
    DebugCustom,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Arbitrary,
    Not,
    BitAnd,
    BitAndAssign,
    BitOr,
    BitOrAssign,
)]
#[debug(fmt = "{_0:?}")]
pub struct Bitboard(#[strategy(any::<u64>().prop_map(sm::Bitboard))] sm::Bitboard);

impl Bitboard {
    /// An empty board.
    pub const fn empty() -> Self {
        Bitboard(sm::Bitboard::EMPTY)
    }

    /// A full board.
    pub const fn full() -> Self {
        Bitboard(sm::Bitboard::FULL)
    }

    /// The number of [`Square`]s on the board.
    pub const fn len(&self) -> usize {
        self.0.count()
    }

    /// Whether the board is empty.
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether this [`Square`] is on the board.
    pub fn contains(&self, s: Square) -> bool {
        self.0.contains(s.into())
    }
}

impl From<File> for Bitboard {
    fn from(f: File) -> Self {
        Bitboard(<sm::Bitboard as From<sm::File>>::from(f.into()))
    }
}

impl From<Rank> for Bitboard {
    fn from(r: Rank) -> Self {
        Bitboard(<sm::Bitboard as From<sm::Rank>>::from(r.into()))
    }
}

impl From<Square> for Bitboard {
    fn from(s: Square) -> Self {
        Bitboard(<sm::Bitboard as From<sm::Square>>::from(s.into()))
    }
}

/// An iterator over the [`Square`]s in a [`Bitboard`].
pub struct BitboardIter(sm::bitboard::IntoIter);

impl Iterator for BitboardIter {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.0.next()?.into())
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = BitboardIter;

    fn into_iter(self) -> Self::IntoIter {
        BitboardIter(self.0.into_iter())
    }
}

#[doc(hidden)]
impl From<sm::Bitboard> for Bitboard {
    fn from(bb: sm::Bitboard) -> Self {
        Bitboard(bb)
    }
}

#[doc(hidden)]
impl From<Bitboard> for sm::Bitboard {
    fn from(bb: Bitboard) -> Self {
        bb.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn empty_constructs_board_with_no_squares() {
        assert_eq!(Bitboard::empty().into_iter().count(), 0);
    }

    #[proptest]
    fn full_constructs_board_with_all_squares() {
        assert_eq!(Bitboard::full().into_iter().count(), 64);
    }

    #[proptest]
    fn len_returns_number_of_squares_on_the_board(bb: Bitboard) {
        assert_eq!(bb.len(), bb.into_iter().count());
    }

    #[proptest]
    fn is_empty_returns_whether_there_are_squares_on_the_board(bb: Bitboard) {
        assert_eq!(bb.is_empty(), bb.into_iter().count() == 0);
    }

    #[proptest]
    fn contains_checks_whether_square_is_on_the_board(bb: Bitboard) {
        for s in bb {
            assert!(bb.contains(s));
        }
    }

    #[proptest]
    fn bitboard_can_be_created_from_file(f: File) {
        let bb = Bitboard::from(f);
        assert_eq!(bb.len(), 8);

        for s in bb {
            assert_eq!(s.file(), f);
        }
    }

    #[proptest]
    fn bitboard_can_be_created_from_rank(r: Rank) {
        let bb = Bitboard::from(r);
        assert_eq!(bb.len(), 8);

        for s in bb {
            assert_eq!(s.rank(), r);
        }
    }

    #[proptest]
    fn bitboard_can_be_created_from_square(s: Square) {
        let bb = Bitboard::from(s);
        assert!(bb.contains(s));
        assert_eq!(bb.len(), 1);
    }

    #[proptest]
    fn bitboard_has_an_equivalent_shakmaty_representation(bb: Bitboard) {
        assert_eq!(Bitboard::from(sm::Bitboard::from(bb)), bb);
    }
}

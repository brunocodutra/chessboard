use crate::chess::{File, Rank, Square};
use cozy_chess as cc;
use derive_more::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Debug, Not};

/// A set of squares on a chess board.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Hash, Not, BitAnd, BitAndAssign, BitOr, BitOrAssign,
)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug("{_0:#?}")]
pub struct Bitboard(#[cfg_attr(test, map(cc::BitBoard))] cc::BitBoard);

impl Bitboard {
    /// An empty board.
    pub const fn empty() -> Self {
        Bitboard(cc::BitBoard::EMPTY)
    }

    /// A full board.
    pub const fn full() -> Self {
        Bitboard(cc::BitBoard::FULL)
    }

    /// Light squares.
    pub const fn light() -> Self {
        Bitboard(cc::BitBoard::LIGHT_SQUARES)
    }

    /// Dark squares.
    pub const fn dark() -> Self {
        Bitboard(cc::BitBoard::DARK_SQUARES)
    }

    /// The number of [`Square`]s on the board.
    pub const fn len(&self) -> usize {
        self.0.len() as _
    }

    /// Whether the board is empty.
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether this [`Square`] is on the board.
    pub fn contains(&self, s: Square) -> bool {
        self.0.has(s.into())
    }
}

impl From<File> for Bitboard {
    fn from(f: File) -> Self {
        Bitboard(<cc::BitBoard as From<cc::File>>::from(f.into()))
    }
}

impl From<Rank> for Bitboard {
    fn from(r: Rank) -> Self {
        Bitboard(<cc::BitBoard as From<cc::Rank>>::from(r.into()))
    }
}

impl From<Square> for Bitboard {
    fn from(s: Square) -> Self {
        Bitboard(<cc::BitBoard as From<cc::Square>>::from(s.into()))
    }
}

/// An iterator over the [`Square`]s in a [`Bitboard`].
pub struct Iter(cc::BitBoardIter);

impl Iterator for Iter {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.0.next()?.into())
    }
}

impl ExactSizeIterator for Iter {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = Iter;

    fn into_iter(self) -> Self::IntoIter {
        Iter(self.0.into_iter())
    }
}

#[doc(hidden)]
impl From<cc::BitBoard> for Bitboard {
    #[inline(always)]
    fn from(bb: cc::BitBoard) -> Self {
        Bitboard(bb)
    }
}

#[doc(hidden)]
impl From<Bitboard> for cc::BitBoard {
    #[inline(always)]
    fn from(bb: Bitboard) -> Self {
        bb.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;
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
    fn light_and_dark_bitboards_are_complementary() {
        assert_eq!(Bitboard::light() | Bitboard::dark(), Bitboard::full());
        assert_eq!(Bitboard::light() & Bitboard::dark(), Bitboard::empty());
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
    fn bitboard_has_an_equivalent_cozy_chess_representation(bb: Bitboard) {
        assert_eq!(Bitboard::from(cc::BitBoard::from(bb)), bb);
    }
}

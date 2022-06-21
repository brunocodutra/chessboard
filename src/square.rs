use crate::{File, ParseFileError, ParseRankError, Rank};
use derive_more::{DebugCustom, Display, Error, From};
use shakmaty as sm;
use std::convert::{TryFrom, TryInto};
use std::{num::TryFromIntError, str::FromStr};
use vampirc_uci::UciSquare;

#[cfg(test)]
use proptest::sample::select;

/// Denotes a square on the chess board.
#[derive(DebugCustom, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug(fmt = "{}", self)]
#[display(fmt = "{}{}", "self.file()", "self.rank()")]
pub struct Square(#[cfg_attr(test, strategy(select(sm::Square::ALL.as_ref())))] sm::Square);

impl Square {
    /// Constructs [`Square`] from a pair of [`File`] and [`Rank`].
    pub fn new(f: File, r: Rank) -> Self {
        Square(sm::Square::from_coords(f.into(), r.into()))
    }

    /// Constructs [`Square`] from index.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not in the range (0..64).
    pub fn from_index(i: u8) -> Self {
        i.try_into().unwrap()
    }

    /// This squares's index in the range (0..64).
    ///
    /// Squares are ordered from a1 = 0 to h8 = 63, files then ranks, so b1 = 2 and a2 = 8.
    pub fn index(&self) -> u8 {
        (*self).into()
    }

    /// Returns an iterator over [`Square`]s ordered by [index][`Square::index`].
    pub fn iter() -> impl DoubleEndedIterator<Item = Self> + ExactSizeIterator {
        sm::Square::ALL.into_iter().map(Square)
    }

    /// This square's [`File`].
    pub fn file(&self) -> File {
        self.0.file().into()
    }

    /// This square's [`Rank`].
    pub fn rank(&self) -> Rank {
        self.0.rank().into()
    }
}

/// The reason why converting [`Square`] from index failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(fmt = "expected integer in the range `(0..64)`")]
pub struct SquareOutOfRange;

impl From<TryFromIntError> for SquareOutOfRange {
    fn from(_: TryFromIntError) -> Self {
        SquareOutOfRange
    }
}

impl TryFrom<u8> for Square {
    type Error = SquareOutOfRange;

    fn try_from(i: u8) -> Result<Self, Self::Error> {
        Ok(Square(i.try_into()?))
    }
}

impl From<Square> for u8 {
    fn from(s: Square) -> u8 {
        s.0.into()
    }
}

/// The reason why parsing [`Square`] failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error, From)]
#[display(fmt = "failed to parse square")]
pub enum ParseSquareError {
    InvalidFile(ParseFileError),
    InvalidRank(ParseRankError),
}

impl FromStr for Square {
    type Err = ParseSquareError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let i = s.char_indices().nth(1).map_or_else(|| s.len(), |(i, _)| i);
        Ok(Square::new(s[..i].parse()?, s[i..].parse()?))
    }
}

#[doc(hidden)]
impl From<Square> for UciSquare {
    fn from(s: Square) -> Self {
        UciSquare {
            file: s.file().into(),
            rank: s.rank().index() + 1,
        }
    }
}

#[doc(hidden)]
impl From<UciSquare> for Square {
    fn from(s: UciSquare) -> Self {
        Square::new(s.file.try_into().unwrap(), (s.rank - 1).try_into().unwrap())
    }
}

#[doc(hidden)]
impl From<sm::Square> for Square {
    fn from(s: sm::Square) -> Self {
        Square(s)
    }
}

#[doc(hidden)]
impl From<Square> for sm::Square {
    fn from(s: Square) -> Self {
        s.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn new_constructs_square_from_pair_of_file_and_rank(s: Square) {
        assert_eq!(Square::new(s.file(), s.rank()), s);
    }

    #[proptest]
    fn iter_returns_iterator_over_files_in_order() {
        assert_eq!(
            Square::iter().collect::<Vec<_>>(),
            (0..=63).map(Square::from_index).collect::<Vec<_>>()
        );
    }

    #[proptest]
    fn iter_returns_double_ended_iterator() {
        assert_eq!(
            Square::iter().rev().collect::<Vec<_>>(),
            (0..=63).rev().map(Square::from_index).collect::<Vec<_>>()
        );
    }

    #[proptest]
    fn iter_returns_iterator_of_exact_size() {
        assert_eq!(Square::iter().len(), 64);
    }

    #[proptest]
    fn parsing_printed_square_is_an_identity(s: Square) {
        assert_eq!(s.to_string().parse(), Ok(s));
    }

    #[proptest]
    fn parsing_square_fails_if_file_is_invalid(#[strategy("[^a-h]")] f: String, r: Rank) {
        let s = [f.clone(), r.to_string()].concat();
        assert_eq!(
            s.parse::<Square>().err(),
            f.parse::<File>().err().map(Into::into)
        );
    }

    #[proptest]
    fn parsing_square_fails_if_rank_is_invalid(f: File, #[strategy("[^1-8]*")] r: String) {
        let s = [f.to_string(), r.clone()].concat();
        assert_eq!(
            s.parse::<Square>().err(),
            r.parse::<Rank>().err().map(Into::into)
        );
    }

    #[proptest]
    fn square_has_an_index(s: Square) {
        assert_eq!(s.index().try_into(), Ok(s));
    }

    #[proptest]
    fn from_index_constructs_square_by_index(#[strategy(0u8..64)] i: u8) {
        assert_eq!(Square::from_index(i).index(), i);
    }

    #[proptest]
    #[should_panic]
    fn from_index_panics_if_index_out_of_range(#[strategy(64u8..)] i: u8) {
        Square::from_index(i);
    }

    #[proptest]
    fn converting_square_from_index_out_of_range_fails(#[strategy(64u8..)] i: u8) {
        assert_eq!(Square::try_from(i), Err(SquareOutOfRange));
    }

    #[proptest]
    fn square_is_ordered_by_index(a: Square, b: Square) {
        assert_eq!(a < b, a.index() < b.index());
    }

    #[proptest]
    fn square_has_an_equivalent_vampirc_uci_representation(s: Square) {
        assert_eq!(Square::from(<UciSquare as From<Square>>::from(s)), s);
    }

    #[proptest]
    fn square_has_an_equivalent_shakmaty_representation(s: Square) {
        assert_eq!(Square::from(sm::Square::from(s)), s);
    }
}

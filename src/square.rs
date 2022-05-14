use crate::{File, ParseFileError, ParseRankError, Rank};
use derive_more::{Display, Error, From};
use shakmaty as sm;
use std::convert::{TryFrom, TryInto};
use std::{cmp::Ordering, iter::FusedIterator, str::FromStr};
use vampirc_uci::UciSquare;

#[cfg(test)]
use test_strategy::Arbitrary;

/// Denotes a square on the chess board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(Arbitrary))]
#[display(fmt = "{}{}", _0, _1)]
pub struct Square(pub File, pub Rank);

impl Square {
    /// Constructs [`Square`] from index.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not in the range (0..=63).
    pub fn new(i: usize) -> Self {
        i.try_into().unwrap()
    }

    /// This squares's index in the range (0..=63).
    ///
    /// Squares are ordered from a1 = 0 to h8 = 63, files then ranks, so b1 = 2 and a2 = 8.
    pub fn index(&self) -> usize {
        (*self).into()
    }

    /// Returns an iterator over [`Square`]s ordered by [index][`Square::index`].
    pub fn iter() -> impl DoubleEndedIterator<Item = Self> + ExactSizeIterator + FusedIterator {
        (0usize..64).map(Square::new)
    }

    /// This square's [`File`].
    pub fn file(&self) -> File {
        self.0
    }

    /// This square's [`Rank`].
    pub fn rank(&self) -> Rank {
        self.1
    }
}

impl Ord for Square {
    fn cmp(&self, other: &Self) -> Ordering {
        self.index().cmp(&other.index())
    }
}

impl PartialOrd for Square {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.index().partial_cmp(&other.index())
    }
}

/// The reason why converting [`Square`] from index failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(fmt = "expected integer in the range `(0..=63)`")]
pub struct SquareIndexOutOfRange;

impl TryFrom<usize> for Square {
    type Error = SquareIndexOutOfRange;

    fn try_from(i: usize) -> Result<Self, Self::Error> {
        Ok(Square(
            (i % 8).try_into().map_err(|_| SquareIndexOutOfRange)?,
            (i / 8).try_into().map_err(|_| SquareIndexOutOfRange)?,
        ))
    }
}

impl From<Square> for usize {
    fn from(f: Square) -> usize {
        f.rank().index() * 8 + f.file().index()
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
        Ok(Square(s[..i].parse()?, s[i..].parse()?))
    }
}

#[doc(hidden)]
impl From<Square> for UciSquare {
    fn from(s: Square) -> Self {
        UciSquare {
            file: s.file().into(),
            rank: s.rank() as u8,
        }
    }
}

#[doc(hidden)]
impl From<UciSquare> for Square {
    fn from(s: UciSquare) -> Self {
        Square(
            s.file.try_into().unwrap(),
            (s.rank as u32).try_into().unwrap(),
        )
    }
}

#[doc(hidden)]
impl From<sm::Square> for Square {
    fn from(s: sm::Square) -> Self {
        Square::new(usize::from(s))
    }
}

#[doc(hidden)]
impl From<Square> for sm::Square {
    fn from(s: Square) -> Self {
        sm::Square::new(s.index() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn iter_returns_iterator_over_files_in_order() {
        let squares: Vec<_> = Rank::iter()
            .flat_map(|r| File::iter().map(move |f| Square(f, r)))
            .collect();

        assert_eq!(Square::iter().collect::<Vec<_>>(), squares);
    }

    #[proptest]
    fn iter_returns_double_ended_iterator() {
        let squares: Vec<_> = Rank::iter()
            .flat_map(|r| File::iter().map(move |f| Square(f, r)))
            .rev()
            .collect();

        assert_eq!(Square::iter().rev().collect::<Vec<_>>(), squares);
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
            s.parse::<Square>(),
            Err(f.parse::<File>().unwrap_err().into())
        );
    }

    #[proptest]
    fn parsing_square_fails_if_rank_is_invalid(f: File, #[strategy("[^1-8]*")] r: String) {
        let s = [f.to_string(), r.clone()].concat();
        assert_eq!(
            s.parse::<Square>(),
            Err(r.parse::<Rank>().unwrap_err().into())
        );
    }

    #[proptest]
    fn square_has_an_index(s: Square) {
        assert_eq!(s.index().try_into(), Ok(s));
    }

    #[proptest]
    fn new_constructs_square_by_index(#[strategy(0usize..=63)] i: usize) {
        assert_eq!(Square::new(i).index(), i);
    }

    #[proptest]
    #[should_panic]
    fn new_panics_if_index_out_of_range(#[strategy(64usize..)] i: usize) {
        Square::new(i);
    }

    #[proptest]
    fn converting_square_from_index_out_of_range_fails(#[strategy(64usize..)] i: usize) {
        assert_eq!(Square::try_from(i), Err(SquareIndexOutOfRange));
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

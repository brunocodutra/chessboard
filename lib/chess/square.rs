use crate::chess::{File, Rank};
use crate::util::{Binary, Bits};
use shakmaty as sm;
use std::{convert::Infallible, fmt};
use vampirc_uci::UciSquare;

/// Denotes a square on the chess board.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(u8)]
#[rustfmt::skip]
pub enum Square {
    A1, B1, C1, D1, E1, F1, G1, H1,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A8, B8, C8, D8, E8, F8, G8, H8,
}

impl Square {
    #[rustfmt::skip]
    const SQUARES: [Self; 64] = [
        Square::A1, Square::B1, Square::C1, Square::D1, Square::E1, Square::F1, Square::G1, Square::H1,
        Square::A2, Square::B2, Square::C2, Square::D2, Square::E2, Square::F2, Square::G2, Square::H2,
        Square::A3, Square::B3, Square::C3, Square::D3, Square::E3, Square::F3, Square::G3, Square::H3,
        Square::A4, Square::B4, Square::C4, Square::D4, Square::E4, Square::F4, Square::G4, Square::H4,
        Square::A5, Square::B5, Square::C5, Square::D5, Square::E5, Square::F5, Square::G5, Square::H5,
        Square::A6, Square::B6, Square::C6, Square::D6, Square::E6, Square::F6, Square::G6, Square::H6,
        Square::A7, Square::B7, Square::C7, Square::D7, Square::E7, Square::F7, Square::G7, Square::H7,
        Square::A8, Square::B8, Square::C8, Square::D8, Square::E8, Square::F8, Square::G8, Square::H8,
    ];

    /// Constructs [`Square`] from a pair of [`File`] and [`Rank`].
    pub fn new(f: File, r: Rank) -> Self {
        Self::from_index(f.index() + r.index() * 8)
    }

    /// Constructs [`Square`] from index.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not in the range (0..64).
    pub fn from_index(i: u8) -> Self {
        Self::SQUARES[i as usize]
    }

    /// This squares's index in the range (0..64).
    ///
    /// Squares are ordered from a1 = 0 to h8 = 63, files then ranks, so b1 = 2 and a2 = 8.
    pub fn index(&self) -> u8 {
        *self as _
    }

    /// Returns an iterator over [`Square`]s ordered by [index][`Square::index`].
    pub fn iter() -> impl DoubleEndedIterator<Item = Self> + ExactSizeIterator {
        Self::SQUARES.into_iter()
    }

    /// This square's [`File`].
    pub fn file(&self) -> File {
        File::from_index(self.index() % 8)
    }

    /// This square's [`Rank`].
    pub fn rank(&self) -> Rank {
        Rank::from_index(self.index() / 8)
    }

    /// Mirrors this square's [`Rank`].
    pub fn mirror(&self) -> Self {
        Self::from_index(self.index() ^ Square::A8.index())
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.file(), self.rank())
    }
}

impl Binary for Square {
    type Bits = Bits<u8, 6>;
    type Error = Infallible;

    fn encode(&self) -> Self::Bits {
        Bits::new(*self as _)
    }

    fn decode(bits: Self::Bits) -> Result<Self, Self::Error> {
        Ok(Square::from_index(bits.get()))
    }
}

#[doc(hidden)]
impl From<Square> for UciSquare {
    fn from(s: Square) -> Self {
        UciSquare {
            file: (s.file().index() + b'a').into(),
            rank: s.rank().index() + 1,
        }
    }
}

#[doc(hidden)]
impl From<UciSquare> for Square {
    fn from(s: UciSquare) -> Self {
        Square::new(
            File::from_index((u32::from(s.file) - u32::from('a')) as _),
            Rank::from_index(s.rank - 1),
        )
    }
}

#[doc(hidden)]
impl From<sm::Square> for Square {
    fn from(s: sm::Square) -> Self {
        Square::from_index(s as _)
    }
}

#[doc(hidden)]
impl From<Square> for sm::Square {
    fn from(s: Square) -> Self {
        sm::Square::new(s as _)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;
    use test_strategy::proptest;

    #[proptest]
    fn square_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<Square>>(), size_of::<Square>());
    }

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
    fn decoding_encoded_square_is_an_identity(s: Square) {
        assert_eq!(Square::decode(s.encode()), Ok(s));
    }

    #[proptest]
    fn decoding_square_never_fails(r: <Square as Binary>::Bits) {
        assert!(Square::decode(r).is_ok());
    }

    #[proptest]
    fn square_has_an_index(s: Square) {
        assert_eq!(Square::from_index(s.index()), s);
    }

    #[proptest]
    fn square_has_a_mirror_on_the_same_file(s: Square) {
        assert_eq!(s.mirror(), Square::new(s.file(), s.rank().mirror()));
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
    fn square_is_ordered_by_index(a: Square, b: Square) {
        assert_eq!(a < b, a.index() < b.index());
    }

    #[proptest]
    fn square_has_an_equivalent_uci_representation(s: Square) {
        assert_eq!(Square::from(<UciSquare as From<Square>>::from(s)), s);
    }

    #[proptest]
    fn square_has_an_equivalent_shakmaty_representation(s: Square) {
        assert_eq!(Square::from(sm::Square::from(s)), s);
    }
}

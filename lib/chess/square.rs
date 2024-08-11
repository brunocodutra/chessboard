use crate::chess::{Bitboard, File, Mirror, ParseFileError, ParseRankError, Perspective, Rank};
use crate::util::{Assume, Binary, Bits, Integer};
use derive_more::{Display, Error, From};
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::{fmt, str::FromStr};

/// A square on the chess board.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(i8)]
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
    /// Constructs [`Square`] from a pair of [`File`] and [`Rank`].
    #[inline(always)]
    pub fn new(f: File, r: Rank) -> Self {
        <Self as Integer>::new(f.get() | r.get() << 3)
    }

    /// This square's [`File`].
    #[inline(always)]
    pub fn file(&self) -> File {
        File::new(self.get() & 0b111)
    }

    /// This square's [`Rank`].
    #[inline(always)]
    pub fn rank(&self) -> Rank {
        Rank::new(self.get() >> 3)
    }

    /// Returns a [`Bitboard`] that only contains this square.
    #[inline(always)]
    pub fn bitboard(self) -> Bitboard {
        Bitboard::new(1 << self.get())
    }
}

unsafe impl Integer for Square {
    type Repr = i8;
    const MIN: Self::Repr = Square::A1 as _;
    const MAX: Self::Repr = Square::H8 as _;
}

impl Mirror for Square {
    /// Horizontally mirrors this square.
    #[inline(always)]
    fn mirror(&self) -> Self {
        <Self as Integer>::new(self.get() ^ Square::H1.get())
    }
}

impl Perspective for Square {
    /// Flips this square's [`Rank`].
    #[inline(always)]
    fn flip(&self) -> Self {
        <Self as Integer>::new(self.get() ^ Square::A8.get())
    }
}

impl Binary for Square {
    type Bits = Bits<u8, 6>;

    #[inline(always)]
    fn encode(&self) -> Self::Bits {
        self.convert().assume()
    }

    #[inline(always)]
    fn decode(bits: Self::Bits) -> Self {
        bits.convert().assume()
    }
}

impl Sub for Square {
    type Output = i8;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        self.get() - rhs.get()
    }
}

impl Sub<i8> for Square {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: i8) -> Self::Output {
        <Self as Integer>::new(self.get() - rhs)
    }
}

impl Add<i8> for Square {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: i8) -> Self::Output {
        <Self as Integer>::new(self.get() + rhs)
    }
}

impl SubAssign<i8> for Square {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: i8) {
        *self = *self - rhs
    }
}

impl AddAssign<i8> for Square {
    #[inline(always)]
    fn add_assign(&mut self, rhs: i8) {
        *self = *self + rhs
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.file(), f)?;
        fmt::Display::fmt(&self.rank(), f)?;
        Ok(())
    }
}

/// The reason why parsing [`Square`] failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error, From)]
pub enum ParseSquareError {
    #[display("failed to parse square")]
    InvalidFile(ParseFileError),
    #[display("failed to parse square")]
    InvalidRank(ParseRankError),
}

impl FromStr for Square {
    type Err = ParseSquareError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let i = s.char_indices().nth(1).map_or_else(|| s.len(), |(i, _)| i);
        Ok(Square::new(s[..i].parse()?, s[i..].parse()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;
    use test_strategy::proptest;

    #[test]
    fn square_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<Square>>(), size_of::<Square>());
    }

    #[proptest]
    fn new_constructs_square_from_pair_of_file_and_rank(sq: Square) {
        assert_eq!(Square::new(sq.file(), sq.rank()), sq);
    }

    #[proptest]
    fn square_has_an_equivalent_bitboard(sq: Square) {
        assert_eq!(Vec::from_iter(sq.bitboard()), vec![sq]);
    }

    #[proptest]
    fn decoding_encoded_square_is_an_identity(sq: Square) {
        assert_eq!(Square::decode(sq.encode()), sq);
    }

    #[proptest]
    fn mirroring_square_mirrors_its_file(sq: Square) {
        assert_eq!(sq.mirror(), Square::new(sq.file().mirror(), sq.rank()));
    }

    #[proptest]
    fn flipping_square_preserves_file_and_flips_rank(sq: Square) {
        assert_eq!(sq.flip(), Square::new(sq.file(), sq.rank().flip()));
    }

    #[proptest]
    fn subtracting_squares_returns_distance(a: Square, b: Square) {
        assert_eq!(b + (a - b), a);
        assert_eq!(a - (a - b), b);
    }

    #[proptest]
    fn square_can_be_incremented(mut sq: Square, #[strategy(-#sq.get()..64 - #sq.get())] i: i8) {
        assert_eq!(sq + i, {
            sq += i;
            sq
        });
    }

    #[proptest]
    fn square_can_be_decremented(mut sq: Square, #[strategy(#sq.get() - 63..=#sq.get())] i: i8) {
        assert_eq!(sq - i, {
            sq -= i;
            sq
        });
    }

    #[proptest]
    fn parsing_printed_square_is_an_identity(sq: Square) {
        assert_eq!(sq.to_string().parse(), Ok(sq));
    }

    #[proptest]
    fn parsing_square_fails_if_file_invalid(
        #[filter(!('a'..='h').contains(&#c))] c: char,
        r: Rank,
    ) {
        assert_eq!(
            [c.to_string(), r.to_string()].concat().parse::<Square>(),
            Err(ParseSquareError::InvalidFile(ParseFileError))
        );
    }

    #[proptest]
    fn parsing_square_fails_if_rank_invalid(
        f: File,
        #[filter(!('1'..='8').contains(&#c))] c: char,
    ) {
        assert_eq!(
            [f.to_string(), c.to_string()].concat().parse::<Square>(),
            Err(ParseSquareError::InvalidRank(ParseRankError))
        );
    }

    #[proptest]
    fn parsing_square_fails_if_length_not_two(#[filter(#s.len() != 2)] s: String) {
        assert_eq!(s.parse::<Square>().ok(), None);
    }
}

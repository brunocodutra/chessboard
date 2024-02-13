use crate::chess::{File, Rank};
use crate::util::{Binary, Bits, Integer};
use cozy_chess as cc;
use std::{fmt, ops::Sub};

/// A square on the chess board.
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
    /// Constructs [`Square`] from a pair of [`File`] and [`Rank`].
    #[inline(always)]
    pub const fn new(f: File, r: Rank) -> Self {
        Self::from_repr(f.repr() + r.repr() * 8)
    }

    /// This square's [`File`].
    #[inline(always)]
    pub const fn file(&self) -> File {
        File::from_repr(self.repr() % 8)
    }

    /// This square's [`Rank`].
    #[inline(always)]
    pub const fn rank(&self) -> Rank {
        Rank::from_repr(self.repr() / 8)
    }

    /// Mirrors this square's [`Rank`].
    #[inline(always)]
    pub const fn flip(&self) -> Self {
        Self::from_repr(self.repr() ^ Square::A8.repr())
    }
}

unsafe impl const Integer for Square {
    type Repr = u8;
    const MIN: Self::Repr = Square::A1 as _;
    const MAX: Self::Repr = Square::H8 as _;
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.file(), self.rank())
    }
}

impl Binary for Square {
    type Bits = Bits<u8, 6>;

    #[inline(always)]
    fn encode(&self) -> Self::Bits {
        Bits::new(*self as _)
    }

    #[inline(always)]
    fn decode(bits: Self::Bits) -> Self {
        Square::from_repr(bits.get())
    }
}

impl Sub for Square {
    type Output = i8;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        self as i8 - rhs as i8
    }
}

#[doc(hidden)]
impl From<cc::Square> for Square {
    #[inline(always)]
    fn from(s: cc::Square) -> Self {
        Square::from_repr(s as _)
    }
}

#[doc(hidden)]
impl From<Square> for cc::Square {
    #[inline(always)]
    fn from(s: Square) -> Self {
        cc::Square::index_const(s as _)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::Mirror;
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
    fn decoding_encoded_square_is_an_identity(s: Square) {
        assert_eq!(Square::decode(s.encode()), s);
    }

    #[proptest]
    fn flipping_square_mirrors_its_rank(s: Square) {
        assert_eq!(s.flip(), Square::new(s.file(), s.rank().mirror()));
    }

    #[proptest]
    fn subtracting_squares_gives_distance(a: Square, b: Square) {
        assert_eq!(a - b, a as i8 - b as i8);
    }

    #[proptest]
    fn square_has_an_equivalent_cozy_chess_representation(s: Square) {
        assert_eq!(Square::from(cc::Square::from(s)), s);
    }
}

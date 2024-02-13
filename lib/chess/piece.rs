use crate::chess::{Color, Role};
use crate::util::Integer;

/// A chess [piece][`Role`] of a certain [`Color`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(u8)]
pub enum Piece {
    WhitePawn,
    BlackPawn,
    WhiteKnight,
    BlackKnight,
    WhiteBishop,
    BlackBishop,
    WhiteRook,
    BlackRook,
    WhiteQueen,
    BlackQueen,
    WhiteKing,
    BlackKing,
}

impl Piece {
    /// Constructs [`Piece`] from a pair of [`Color`] and [`Role`].
    #[inline(always)]
    pub const fn new(r: Role, c: Color) -> Self {
        Self::from_repr(r.repr() * 2 + c.repr())
    }

    /// This piece's [`Role`].
    #[inline(always)]
    pub const fn role(&self) -> Role {
        Role::from_repr(self.repr() / 2)
    }

    /// This piece's [`Color`].
    #[inline(always)]
    pub const fn color(&self) -> Color {
        Color::from_repr(self.repr() % 2)
    }

    /// Mirrors this piece's [`Color`].
    #[inline(always)]
    pub const fn flip(&self) -> Self {
        Self::from_repr(self.repr() ^ Piece::BlackPawn.repr())
    }
}

unsafe impl const Integer for Piece {
    type Repr = u8;
    const MIN: Self::Repr = Piece::WhitePawn as _;
    const MAX: Self::Repr = Piece::BlackKing as _;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;
    use test_strategy::proptest;

    #[proptest]
    fn piece_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<Piece>>(), size_of::<Piece>());
    }

    #[proptest]
    fn piece_has_a_color(r: Role, c: Color) {
        assert_eq!(Piece::new(r, c).color(), c);
    }

    #[proptest]
    fn piece_has_a_role(r: Role, c: Color) {
        assert_eq!(Piece::new(r, c).role(), r);
    }

    #[proptest]
    fn piece_has_a_mirror_of_the_same_role_and_opposite_color(p: Piece) {
        assert_eq!(p.flip().role(), p.role());
        assert_eq!(p.flip().color(), !p.color());
    }
}

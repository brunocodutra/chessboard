use crate::chess::{Color, Perspective, Role};
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
        <Self as Integer>::new(c.get() | r.get() << 1)
    }

    /// This piece's [`Role`].
    #[inline(always)]
    pub const fn role(&self) -> Role {
        Role::new(self.get() >> 1)
    }

    /// This piece's [`Color`].
    #[inline(always)]
    pub const fn color(&self) -> Color {
        Color::new(self.get() & 0b1)
    }
}

unsafe impl const Integer for Piece {
    type Repr = u8;
    const MIN: Self::Repr = Piece::WhitePawn as _;
    const MAX: Self::Repr = Piece::BlackKing as _;
}

impl const Perspective for Piece {
    /// Mirrors this piece's [`Color`].
    #[inline(always)]
    fn flip(&self) -> Self {
        <Self as Integer>::new(self.get() ^ Piece::BlackPawn.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;
    use test_strategy::proptest;

    #[test]
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
    fn flipping_piece_preserves_role_and_mirrors_color(p: Piece) {
        assert_eq!(p.flip().role(), p.role());
        assert_eq!(p.flip().color(), !p.color());
    }
}

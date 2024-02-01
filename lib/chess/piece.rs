use crate::chess::{Color, Role};

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
    pub const ALL: [Self; 12] = [
        Piece::WhitePawn,
        Piece::BlackPawn,
        Piece::WhiteKnight,
        Piece::BlackKnight,
        Piece::WhiteBishop,
        Piece::BlackBishop,
        Piece::WhiteRook,
        Piece::BlackRook,
        Piece::WhiteQueen,
        Piece::BlackQueen,
        Piece::WhiteKing,
        Piece::BlackKing,
    ];

    /// Constructs [`Piece`] from a pair of [`Color`] and [`Role`].
    pub fn new(r: Role, c: Color) -> Self {
        Self::from_index(r.index() * 2 + c.index())
    }

    /// This piece's [`Role`].
    pub fn role(&self) -> Role {
        Role::from_index(self.index() / 2)
    }

    /// This piece's [`Color`].
    pub fn color(&self) -> Color {
        Color::from_index(self.index() % 2)
    }

    /// This piece's index in the range (0..12).
    pub fn index(&self) -> u8 {
        *self as _
    }

    /// Constructs [`Piece`] from index.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not in the range (0..=11).
    pub fn from_index(i: u8) -> Self {
        Self::ALL[i as usize]
    }

    /// This piece's mirror of the same [`Role`] and opposite [`Color`].
    pub fn mirror(&self) -> Self {
        Self::from_index(self.index() ^ Piece::BlackPawn.index())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Buffer;
    use test_strategy::proptest;

    #[proptest]
    fn piece_has_a_color(r: Role, c: Color) {
        assert_eq!(Piece::new(r, c).color(), c);
    }

    #[proptest]
    fn piece_has_a_role(r: Role, c: Color) {
        assert_eq!(Piece::new(r, c).role(), r);
    }

    #[proptest]
    fn piece_has_an_index(p: Piece) {
        assert_eq!(Piece::from_index(p.index()), p);
    }

    #[proptest]

    fn from_index_constructs_piece_by_index(#[strategy(0u8..12)] i: u8) {
        assert_eq!(Piece::from_index(i).index(), i);
    }

    #[proptest]
    #[should_panic]

    fn from_index_panics_if_index_out_of_range(#[strategy(6u8..)] i: u8) {
        Role::from_index(i);
    }

    #[proptest]
    fn piece_is_ordered_by_index(a: Piece, b: Piece) {
        assert_eq!(a < b, a.index() < b.index());
    }

    #[proptest]
    fn all_contains_pieces_in_order() {
        assert_eq!(
            Piece::ALL.into_iter().collect::<Buffer<_, 12>>(),
            (0..12).map(Piece::from_index).collect()
        );
    }

    #[proptest]
    fn piece_has_a_mirror_of_the_same_role_and_opposite_color(p: Piece) {
        assert_eq!(p.mirror().role(), p.role());
        assert_eq!(p.mirror().color(), !p.color());
    }
}

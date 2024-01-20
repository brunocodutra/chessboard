use crate::chess::{Color, Role};

/// A chess [piece][`Role`] of a certain [`Color`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Piece(pub Role, pub Color);

impl Piece {
    pub const ALL: [Self; 12] = [
        Piece(Role::Pawn, Color::White),
        Piece(Role::Pawn, Color::Black),
        Piece(Role::Knight, Color::White),
        Piece(Role::Knight, Color::Black),
        Piece(Role::Bishop, Color::White),
        Piece(Role::Bishop, Color::Black),
        Piece(Role::Rook, Color::White),
        Piece(Role::Rook, Color::Black),
        Piece(Role::Queen, Color::White),
        Piece(Role::Queen, Color::Black),
        Piece(Role::King, Color::White),
        Piece(Role::King, Color::Black),
    ];

    /// This piece's [`Role`].
    pub fn role(&self) -> Role {
        self.0
    }

    /// This piece's [`Color`].
    pub fn color(&self) -> Color {
        self.1
    }

    /// This piece's index in the range (0..12).
    pub fn index(&self) -> u8 {
        self.color() as u8 + self.role() as u8 * 2
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
        Piece(self.role(), !self.color())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Buffer;
    use test_strategy::proptest;

    #[proptest]
    fn piece_has_a_color(r: Role, c: Color) {
        assert_eq!(Piece(r, c).color(), c);
    }

    #[proptest]
    fn piece_has_a_role(r: Role, c: Color) {
        assert_eq!(Piece(r, c).role(), r);
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

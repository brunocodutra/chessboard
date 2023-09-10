use crate::chess::{Color, Role};
use shakmaty as sm;

/// A chess [piece][`Role`] of a certain [`Color`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Piece(pub Color, pub Role);

impl Piece {
    /// This piece's [`Color`].
    pub fn color(&self) -> Color {
        self.0
    }

    /// This piece's [`Role`].
    pub fn role(&self) -> Role {
        self.1
    }

    /// This piece's index in the range (0..12).
    pub fn index(&self) -> u8 {
        self.color() as u8 + self.role() as u8 * 2
    }

    /// This piece's mirror of the same [`Role`] and opposite [`Color`].
    pub fn mirror(&self) -> Self {
        Piece(!self.color(), self.role())
    }
}

#[doc(hidden)]
impl From<sm::Piece> for Piece {
    fn from(p: sm::Piece) -> Self {
        Piece(p.color.into(), p.role.into())
    }
}

#[doc(hidden)]
impl From<Piece> for sm::Piece {
    fn from(p: Piece) -> Self {
        sm::Piece {
            color: p.color().into(),
            role: p.role().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn piece_has_a_color(c: Color, r: Role) {
        assert_eq!(Piece(c, r).color(), c);
    }

    #[proptest]
    fn piece_has_a_role(c: Color, r: Role) {
        assert_eq!(Piece(c, r).role(), r);
    }

    #[proptest]
    fn piece_has_a_unique_index(p: Piece, #[filter(#p != #q)] q: Piece) {
        assert_ne!(p.index(), q.index());
    }

    #[proptest]
    fn piece_has_a_mirror_of_the_same_role_and_opposite_color(p: Piece) {
        assert_eq!(p.mirror().role(), p.role());
        assert_eq!(p.mirror().color(), !p.color());
    }

    #[proptest]
    fn piece_has_an_equivalent_shakmaty_representation(p: Piece) {
        assert_eq!(Piece::from(sm::Piece::from(p)), p);
    }
}

use crate::{Color, Role};
use shakmaty as sm;
use test_strategy::Arbitrary;

/// A chess [piece][`Role`] of a certain [`Color`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub struct Piece(pub Color, pub Role);

impl Piece {
    /// This piece's [`Color`].
    #[inline]
    pub fn color(&self) -> Color {
        self.0
    }

    /// This piece's [`Role`].
    #[inline]
    pub fn role(&self) -> Role {
        self.1
    }

    /// This piece's index in the range (0..12).
    #[inline]
    pub fn index(&self) -> u8 {
        self.color() as u8 + self.role() as u8 * 2
    }
}

#[doc(hidden)]
impl From<sm::Piece> for Piece {
    #[inline]
    fn from(p: sm::Piece) -> Self {
        Piece(p.color.into(), p.role.into())
    }
}

#[doc(hidden)]
impl From<Piece> for sm::Piece {
    #[inline]
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
    fn piece_has_an_equivalent_shakmaty_representation(p: Piece) {
        assert_eq!(Piece::from(sm::Piece::from(p)), p);
    }
}

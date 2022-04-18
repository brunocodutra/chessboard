use crate::{Color, Role};
use shakmaty as sm;
use std::fmt::{Display, Error as FmtError, Formatter, Write};

#[cfg(test)]
use test_strategy::Arbitrary;

/// A chess [piece][`Role`] of a certain [`Color`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(Arbitrary))]
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

    fn figurine(&self) -> char {
        match self {
            Piece(Color::White, Role::Pawn) => '♙',
            Piece(Color::White, Role::Knight) => '♘',
            Piece(Color::White, Role::Bishop) => '♗',
            Piece(Color::White, Role::Rook) => '♖',
            Piece(Color::White, Role::Queen) => '♕',
            Piece(Color::White, Role::King) => '♔',
            Piece(Color::Black, Role::Pawn) => '♟',
            Piece(Color::Black, Role::Knight) => '♞',
            Piece(Color::Black, Role::Bishop) => '♝',
            Piece(Color::Black, Role::Rook) => '♜',
            Piece(Color::Black, Role::Queen) => '♛',
            Piece(Color::Black, Role::King) => '♚',
        }
    }
}

impl From<Piece> for char {
    fn from(p: Piece) -> char {
        match p {
            Piece(Color::White, Role::Pawn) => 'P',
            Piece(Color::White, Role::Knight) => 'N',
            Piece(Color::White, Role::Bishop) => 'B',
            Piece(Color::White, Role::Rook) => 'R',
            Piece(Color::White, Role::Queen) => 'Q',
            Piece(Color::White, Role::King) => 'K',
            Piece(Color::Black, Role::Pawn) => 'p',
            Piece(Color::Black, Role::Knight) => 'n',
            Piece(Color::Black, Role::Bishop) => 'b',
            Piece(Color::Black, Role::Rook) => 'r',
            Piece(Color::Black, Role::Queen) => 'q',
            Piece(Color::Black, Role::King) => 'k',
        }
    }
}

impl Display for Piece {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let c = if f.alternate() {
            self.figurine()
        } else {
            (*self).into()
        };

        f.write_char(c)
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
    fn file_can_be_converted_into_char(p: Piece) {
        assert_eq!(char::from(p), sm::Piece::from(p).char());
    }

    #[proptest]
    fn piece_has_a_default_ascii_representation(p: Piece) {
        assert_eq!(char::from(p).to_string(), format!("{}", p));
    }

    #[proptest]
    fn piece_has_an_alternate_figurine_representation(p: Piece) {
        assert_eq!(p.figurine().to_string(), format!("{:#}", p));
    }

    #[proptest]
    fn piece_has_an_equivalent_shakmaty_representation(p: Piece) {
        assert_eq!(Piece::from(sm::Piece::from(p)), p);
    }
}

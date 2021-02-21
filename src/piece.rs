use crate::{Color, Role};
use derive_more::Display;

/// A chess piece of a certain color.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Piece {
    #[display(fmt = "{}", "char::from(*self)")]
    White(Role),
    #[display(fmt = "{}", "char::from(*self)")]
    Black(Role),
}

impl Piece {
    pub fn new(color: Color, role: Role) -> Self {
        use Piece::*;
        match color {
            Color::White => White(role),
            Color::Black => Black(role),
        }
    }

    pub fn color(&self) -> Color {
        use Piece::*;
        match *self {
            White(_) => Color::White,
            Black(_) => Color::Black,
        }
    }

    pub fn role(&self) -> Role {
        use Piece::*;
        match *self {
            White(p) | Black(p) => p,
        }
    }
}

impl From<Piece> for char {
    fn from(p: Piece) -> char {
        use Piece::*;
        use Role::*;
        match p {
            White(Pawn) => '♙',
            White(Knight) => '♘',
            White(Bishop) => '♗',
            White(Rook) => '♖',
            White(Queen) => '♕',
            White(King) => '♔',
            Black(Pawn) => '♟',
            Black(Knight) => '♞',
            Black(Bishop) => '♝',
            Black(Rook) => '♜',
            Black(Queen) => '♛',
            Black(King) => '♚',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn every_file_has_an_associated_character(p: Piece) {
            assert_eq!(char::from(p).to_string(), p.to_string());
        }

        #[test]
        fn every_piece_has_a_color(c: Color, r: Role) {
            assert_eq!(Piece::new(c, r).color(), c);
        }

        #[test]
        fn every_piece_has_a_role(c: Color, r: Role) {
            assert_eq!(Piece::new(c, r).role(), r);
        }
    }
}

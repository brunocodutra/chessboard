use crate::{Color, Role};
use std::fmt::{self, Write};

/// A chess [piece][`Role`] of a certain [`Color`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Piece {
    White(Role),
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

    fn figurine(&self) -> char {
        match self {
            Piece::White(Role::Pawn) => '♙',
            Piece::White(Role::Knight) => '♘',
            Piece::White(Role::Bishop) => '♗',
            Piece::White(Role::Rook) => '♖',
            Piece::White(Role::Queen) => '♕',
            Piece::White(Role::King) => '♔',
            Piece::Black(Role::Pawn) => '♟',
            Piece::Black(Role::Knight) => '♞',
            Piece::Black(Role::Bishop) => '♝',
            Piece::Black(Role::Rook) => '♜',
            Piece::Black(Role::Queen) => '♛',
            Piece::Black(Role::King) => '♚',
        }
    }
}

impl From<Piece> for char {
    fn from(p: Piece) -> char {
        match p {
            Piece::White(Role::Pawn) => 'P',
            Piece::White(Role::Knight) => 'N',
            Piece::White(Role::Bishop) => 'B',
            Piece::White(Role::Rook) => 'R',
            Piece::White(Role::Queen) => 'Q',
            Piece::White(Role::King) => 'K',
            Piece::Black(Role::Pawn) => 'p',
            Piece::Black(Role::Knight) => 'n',
            Piece::Black(Role::Bishop) => 'b',
            Piece::Black(Role::Rook) => 'r',
            Piece::Black(Role::Queen) => 'q',
            Piece::Black(Role::King) => 'k',
        }
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = if f.alternate() {
            self.figurine()
        } else {
            (*self).into()
        };

        f.write_char(c)
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
        fn piece_has_a_default_ascii_representation(p: Piece) {
            assert_eq!(char::from(p).to_string(), format!("{}", p));
        }

        #[test]
        fn piece_has_an_alternate_figurine_representation(p: Piece) {
            assert_eq!(p.figurine().to_string(), format!("{:#}", p));
        }

        #[test]
        fn every_piece_has_a_role(c: Color, r: Role) {
            assert_eq!(Piece::new(c, r).role(), r);
        }
    }
}

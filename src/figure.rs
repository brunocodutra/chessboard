use crate::{Color, Role};
use derive_more::Display;

/// A chess piece of a certain color.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Figure {
    #[display(fmt = "{}", "char::from(*self)")]
    White(Role),
    #[display(fmt = "{}", "char::from(*self)")]
    Black(Role),
}

impl Figure {
    pub fn new(color: Color, role: Role) -> Self {
        use Figure::*;
        match color {
            Color::White => White(role),
            Color::Black => Black(role),
        }
    }

    pub fn color(&self) -> Color {
        use Figure::*;
        match *self {
            White(_) => Color::White,
            Black(_) => Color::Black,
        }
    }

    pub fn role(&self) -> Role {
        use Figure::*;
        match *self {
            White(p) | Black(p) => p,
        }
    }
}

impl From<Figure> for char {
    fn from(f: Figure) -> char {
        use Figure::*;
        use Role::*;
        match f {
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

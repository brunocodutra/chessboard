use crate::{Color, Piece};
use derive_more::Display;

/// A chess piece of a certain color.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Figure {
    #[display(fmt = "{}", "char::from(*self)")]
    White(Piece),
    #[display(fmt = "{}", "char::from(*self)")]
    Black(Piece),
}

impl Figure {
    pub fn new(color: Color, piece: Piece) -> Self {
        use Figure::*;
        match color {
            Color::White => White(piece),
            Color::Black => Black(piece),
        }
    }

    pub fn color(&self) -> Color {
        use Figure::*;
        match *self {
            White(_) => Color::White,
            Black(_) => Color::Black,
        }
    }

    pub fn piece(&self) -> Piece {
        use Figure::*;
        match *self {
            White(p) | Black(p) => p,
        }
    }
}

impl From<Figure> for char {
    fn from(f: Figure) -> char {
        use Figure::*;
        use Piece::*;
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

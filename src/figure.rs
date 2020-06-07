use crate::{Color, Piece};
use std::fmt;

/// A chess piece of a certain color.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Figure {
    White(Piece),
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

    fn symbol(self) -> &'static str {
        use Figure::*;
        use Piece::*;
        match self {
            White(Pawn) => "♙",
            White(Knight) => "♘",
            White(Bishop) => "♗",
            White(Rook) => "♖",
            White(Queen) => "♕",
            White(King) => "♔",
            Black(Pawn) => "♟",
            Black(Knight) => "♞",
            Black(Bishop) => "♝",
            Black(Rook) => "♜",
            Black(Queen) => "♛",
            Black(King) => "♚",
        }
    }
}

impl fmt::Display for Figure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "{}", self.symbol())
        } else {
            write!(f, "{} {}", self.color(), self.piece())
        }
    }
}

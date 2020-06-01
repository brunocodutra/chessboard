use crate::{Color, Piece};
use std::fmt;

/// A chess piece of a certain color.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Figure {
    pub piece: Piece,
    pub color: Color,
}

impl Figure {
    fn symbol(self) -> &'static str {
        use Color::*;
        use Piece::*;
        match (self.piece, self.color) {
            (Pawn, White) => "♙",
            (Knight, White) => "♘",
            (Bishop, White) => "♗",
            (Rook, White) => "♖",
            (Queen, White) => "♕",
            (King, White) => "♔",
            (Pawn, Black) => "♟",
            (Knight, Black) => "♞",
            (Bishop, Black) => "♝",
            (Rook, Black) => "♜",
            (Queen, Black) => "♛",
            (King, Black) => "♚",
        }
    }
}

impl fmt::Display for Figure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "{}", self.symbol())
        } else {
            write!(f, "{} {}", self.color, self.piece)
        }
    }
}

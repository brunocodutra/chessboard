use crate::{color::*, piece::*};
use derive_more::Display;

/// A chess piece of a certain color.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(fmt = "{}", "self.to_str()")]
pub struct Figure {
    pub piece: Piece,
    pub color: Color,
}

impl Figure {
    fn to_str(self) -> &'static str {
        use Color::*;
        use Piece::*;
        match (self.piece, self.color) {
            (Pawn, White) => &"♙",
            (Knight, White) => &"♘",
            (Bishop, White) => &"♗",
            (Rook, White) => &"♖",
            (Queen, White) => &"♕",
            (King, White) => &"♔",
            (Pawn, Black) => &"♟",
            (Knight, Black) => &"♞",
            (Bishop, Black) => &"♝",
            (Rook, Black) => &"♜",
            (Queen, Black) => &"♛",
            (King, Black) => &"♚",
        }
    }
}

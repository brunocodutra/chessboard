use crate::{foreign, piece::*};
use derive_more::Display;

/// A chess piece.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Promotion {
    #[display(fmt = "n")]
    Knight,
    #[display(fmt = "b")]
    Bishop,
    #[display(fmt = "r")]
    Rook,
    #[display(fmt = "q")]
    Queen,
}

impl Into<Piece> for Promotion {
    fn into(self: Self) -> Piece {
        use Promotion::*;
        match self {
            Knight => Piece::Knight,
            Bishop => Piece::Bishop,
            Rook => Piece::Rook,
            Queen => Piece::Queen,
        }
    }
}

impl Into<foreign::Piece> for Promotion {
    fn into(self: Self) -> foreign::Piece {
        use Promotion::*;
        match self {
            Knight => foreign::Piece::Knight,
            Bishop => foreign::Piece::Bishop,
            Rook => foreign::Piece::Rook,
            Queen => foreign::Piece::Queen,
        }
    }
}

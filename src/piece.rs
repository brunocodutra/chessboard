use crate::{foreign, Promotion};
use derive_more::Display;

/// A chess piece.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Piece {
    #[display(fmt = "pawn")]
    Pawn,
    #[display(fmt = "knight")]
    Knight,
    #[display(fmt = "bishop")]
    Bishop,
    #[display(fmt = "rook")]
    Rook,
    #[display(fmt = "queen")]
    Queen,
    #[display(fmt = "king")]
    King,
}

impl From<Promotion> for Piece {
    fn from(p: Promotion) -> Self {
        use Piece::*;
        match p {
            Promotion::Knight => Knight,
            Promotion::Bishop => Bishop,
            Promotion::Rook => Rook,
            Promotion::Queen => Queen,
        }
    }
}

impl From<foreign::Piece> for Piece {
    fn from(p: foreign::Piece) -> Self {
        use Piece::*;
        match p {
            foreign::Piece::Pawn => Pawn,
            foreign::Piece::Knight => Knight,
            foreign::Piece::Bishop => Bishop,
            foreign::Piece::Rook => Rook,
            foreign::Piece::Queen => Queen,
            foreign::Piece::King => King,
        }
    }
}

impl Into<foreign::Piece> for Piece {
    fn into(self: Self) -> foreign::Piece {
        use Piece::*;
        match self {
            Pawn => foreign::Piece::Pawn,
            Knight => foreign::Piece::Knight,
            Bishop => foreign::Piece::Bishop,
            Rook => foreign::Piece::Rook,
            Queen => foreign::Piece::Queen,
            King => foreign::Piece::King,
        }
    }
}

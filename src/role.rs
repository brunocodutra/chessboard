use crate::{foreign, Promotion};
use derive_more::Display;

/// The chess piece type.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Role {
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

impl From<Promotion> for Role {
    fn from(p: Promotion) -> Self {
        use Role::*;
        match p {
            Promotion::Knight => Knight,
            Promotion::Bishop => Bishop,
            Promotion::Rook => Rook,
            Promotion::Queen => Queen,
        }
    }
}

impl From<foreign::Piece> for Role {
    fn from(p: foreign::Piece) -> Self {
        use Role::*;
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

impl Into<foreign::Piece> for Role {
    fn into(self) -> foreign::Piece {
        use Role::*;
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

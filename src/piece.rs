use crate::foreign;

/// A chess piece.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl Piece {
    pub fn to_str(self) -> &'static str {
        use Piece::*;
        match self {
            Pawn => &"pawn",
            Knight => &"knight",
            Bishop => &"bishop",
            Rook => &"rook",
            Queen => &"queen",
            King => &"king",
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

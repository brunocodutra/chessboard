use crate::foreign;
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

impl From<Role> for &'static str {
    fn from(r: Role) -> Self {
        match r {
            Role::Pawn => "pawn",
            Role::Knight => "knight",
            Role::Bishop => "bishop",
            Role::Rook => "rook",
            Role::Queen => "queen",
            Role::King => "king",
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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn every_role_has_an_associated_static_str(r: Role) {
            assert_eq!(<&str>::from(r), r.to_string());
        }
    }
}

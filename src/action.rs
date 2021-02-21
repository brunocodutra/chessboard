use crate::*;
use derive_more::{Display, Error};
use std::str::{self, FromStr};

/// The move of a piece on the board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(
    fmt = "{}{}{}",
    "from",
    "to",
    "promotion.map_or_else(String::new, |p| p.to_string())"
)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    /// If the move of a pawn triggers a promotion, the target piece should be specified.
    pub promotion: Option<Promotion>,
}

/// The reason why parsin a [`Move`] failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Hash, Error)]
pub enum ParseMoveError {
    #[display(fmt = "unable to parse move from `{}`; invalid 'from' square", _0)]
    InvalidFromSquare(String, #[error(source)] ParseSquareError),

    #[display(fmt = "unable to parse move from `{}`; invalid 'to' square", _0)]
    InvalidToSquare(String, #[error(source)] ParseSquareError),

    #[display(fmt = "unable to parse move from `{}`; invalid promotion", _0)]
    InvalidPromotion(String, #[error(source)] ParsePromotionError),
}

impl FromStr for Move {
    type Err = ParseMoveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ParseMoveError::*;

        let i = s.char_indices().nth(2).map_or_else(|| s.len(), |(i, _)| i);
        let j = s.char_indices().nth(4).map_or_else(|| s.len(), |(i, _)| i);

        Ok(Move {
            from: s[..i].parse().map_err(|e| InvalidFromSquare(s.into(), e))?,
            to: s[i..j].parse().map_err(|e| InvalidToSquare(s.into(), e))?,
            promotion: match &s[j..] {
                "" => None,
                p => Some(p.parse().map_err(|e| InvalidPromotion(s.into(), e))?),
            },
        })
    }
}

impl From<foreign::ChessMove> for Move {
    fn from(m: foreign::ChessMove) -> Self {
        Move {
            from: m.get_source().into(),
            to: m.get_dest().into(),
            promotion: match m.get_promotion() {
                Some(foreign::Piece::Knight) => Some(Promotion::Knight),
                Some(foreign::Piece::Bishop) => Some(Promotion::Bishop),
                Some(foreign::Piece::Rook) => Some(Promotion::Rook),
                Some(foreign::Piece::Queen) => Some(Promotion::Queen),
                _ => None,
            },
        }
    }
}

impl Into<foreign::ChessMove> for Move {
    fn into(self) -> foreign::ChessMove {
        foreign::ChessMove::new(
            self.from.into(),
            self.to.into(),
            self.promotion.map(Into::into),
        )
    }
}

/// The possible actions a player can take.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum PlayerAction {
    /// Move a piece on the board.
    #[display(fmt = "move {}", _0)]
    MakeMove(Move),

    /// Resign the match in favor of the opponent.
    #[display(fmt = "resign")]
    Resign,
}

/// The reason why a player action was rejected.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Error)]
#[error(ignore)]
pub enum InvalidPlayerAction {
    #[display(fmt = "the game has ended in a {}", _0)]
    GameHasEnded(Outcome),

    #[display(
        fmt = "the {} player is not allowed to move a {} {} from {} to {} with {} promotion",
        "_0",
        "_1.color()",
        "_1.role()",
        "_2.from",
        "_2.to",
        "_2.promotion.map_or_else(|| \"no\".into(), |p| Role::from(p).to_string())"
    )]
    IllegalMove(Color, Piece, Move),

    #[display(
        fmt = "the {} player attempted to move a nonexistent piece from {} to {}",
        "_0",
        "_1.from",
        "_1.to"
    )]
    InvalidMove(Color, Move),
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn parsing_printed_move_is_an_identity(m: Move) {
            assert_eq!(m.to_string().parse(), Ok(m));
        }

        #[test]
        fn parsing_move_fails_if_from_square_is_invalid(f in "[^a-h1-8]{2}", t: Square, p: Promotion) {
            use ParseMoveError::*;
            let s = [f.clone(), t.to_string(), p.to_string()].concat();
            assert_eq!(s.parse::<Move>(), Err(InvalidFromSquare(s, f.parse::<Square>().unwrap_err())));
        }

        #[test]
        fn parsing_move_fails_if_to_square_is_invalid(f: Square, t in "[^a-h1-8]{2}", p: Promotion) {
            use ParseMoveError::*;
            let s = [f.to_string(), t.clone(), p.to_string()].concat();
            assert_eq!(s.parse::<Move>(), Err(InvalidToSquare(s, t.parse::<Square>().unwrap_err())));
        }

        #[test]
        fn parsing_move_fails_if_promotion_is_invalid(f: Square, t: Square, p in "[^nbrq]+") {
            use ParseMoveError::*;
            let s = [f.to_string(), t.to_string(), p.clone()].concat();
            assert_eq!(s.parse::<Move>(), Err(InvalidPromotion(s, p.parse::<Promotion>().unwrap_err())));
        }
    }
}

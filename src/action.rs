use crate::*;
use derive_more::{Display, Error};
use std::str::{self, FromStr};

/// The move of a piece on the board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(
    fmt = "{}{}{}",
    "self.from",
    "self.to",
    "self.promotion.map(|p| p.to_string()).unwrap_or_else(String::new)"
)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    /// If the move of a pawn triggers a promotion, the target piece should be specified.
    pub promotion: Option<Promotion>,
}

/// The reason parsing a [`Move`] failed.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Error)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(fmt = "unable to parse move, {}")]
pub enum ParseMoveError {
    #[display(fmt = "invalid square at the 'from' position")]
    InvalidFromSquare(ParseSquareError),
    #[display(fmt = "invalid square at the 'to' position")]
    InvalidToSquare(ParseSquareError),
    #[display(fmt = "invalid promotion specifier")]
    InvalidPromotion(ParsePromotionError),
}

impl FromStr for Move {
    type Err = ParseMoveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ParseMoveError::*;

        let terminator = (s.len(), '\0');
        let (i, _) = s.char_indices().nth(2).unwrap_or(terminator);
        let (j, _) = s.char_indices().nth(4).unwrap_or(terminator);

        Ok(Move {
            from: s[..i].parse().map_err(InvalidFromSquare)?,
            to: s[i..j].parse().map_err(InvalidToSquare)?,
            promotion: match &s[j..] {
                "" => None,
                p => Some(p.parse().map_err(InvalidPromotion)?),
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
    fn into(self: Self) -> foreign::ChessMove {
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
    #[display(fmt = "move {}", "_0")]
    MakeMove(Move),

    /// Resign the match in favor of the opponent.
    #[display(fmt = "resign")]
    Resign,
}

/// The reason why a player action was rejected.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Error)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary), proptest(no_params))]
#[error(ignore)]
pub enum InvalidPlayerAction {
    #[display(fmt = "the game has ended in a {}", "_0")]
    GameHasEnded(Outcome),

    #[display(
        fmt = "the {} player is not allowed move the {} from {} to {} with {} promotion",
        "_0",
        "_1",
        "_2.from",
        "_2.to",
        "_2.promotion.map(|p| Piece::from(p).to_string()).unwrap_or_else(|| \"no\".into())"
    )]
    IllegalMove(Color, Figure, Move),

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
        fn parsing_move_fails_if_from_square_file_is_invalid(m in "[^a-h][1-8][a-h][1-8][nbrq]*") {
            use ParseMoveError::*;
            use ParseSquareError::*;
            assert_eq!(m.parse::<Move>(), Err(InvalidFromSquare(InvalidFile(ParseFileError))));
        }

        #[test]
        fn parsing_move_fails_if_from_square_rank_is_invalid(m in "[a-h][^1-8][a-h][1-8][nbrq]*") {
            use ParseMoveError::*;
            use ParseSquareError::*;
            assert_eq!(m.parse::<Move>(), Err(InvalidFromSquare(InvalidRank(ParseRankError))));
        }

        #[test]
        fn parsing_move_fails_if_to_square_file_is_invalid(m in "[a-h][1-8][^a-h][1-8][nbrq]*") {
            use ParseMoveError::*;
            use ParseSquareError::*;
            assert_eq!(m.parse::<Move>(), Err(InvalidToSquare(InvalidFile(ParseFileError))));
        }

        #[test]
        fn parsing_move_fails_if_to_square_rank_is_invalid(m in "[a-h][1-8][a-h][^1-8][nbrq]*") {
            use ParseMoveError::*;
            use ParseSquareError::*;
            assert_eq!(m.parse::<Move>(), Err(InvalidToSquare(InvalidRank(ParseRankError))));
        }

        #[test]
        fn parsing_move_fails_if_promotion_is_invalid(m in "[a-h][1-8][a-h][1-8][^nbrq]+") {
            use ParseMoveError::*;
            assert_eq!(m.parse::<Move>(), Err(InvalidPromotion(ParsePromotionError)));
        }
    }
}

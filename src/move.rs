use crate::{foreign, ParsePromotionError, ParseSquareError, Promotion, Square};
use derive_more::{Display, Error};
use std::str::{self, FromStr};
use tracing::instrument;

/// The move of a piece on the board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(fmt = "{}{}{}", from, to, promotion)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    pub promotion: Promotion,
}

/// The reason why parsing [`Move`] failed.
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

    #[instrument(err)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ParseMoveError::*;

        let i = s.char_indices().nth(2).map_or_else(|| s.len(), |(i, _)| i);
        let j = s.char_indices().nth(4).map_or_else(|| s.len(), |(i, _)| i);

        Ok(Move {
            from: s[..i].parse().map_err(|e| InvalidFromSquare(s.into(), e))?,
            to: s[i..j].parse().map_err(|e| InvalidToSquare(s.into(), e))?,
            promotion: s[j..].parse().map_err(|e| InvalidPromotion(s.into(), e))?,
        })
    }
}

impl From<foreign::ChessMove> for Move {
    fn from(m: foreign::ChessMove) -> Self {
        Move {
            from: m.get_source().into(),
            to: m.get_dest().into(),
            promotion: match m.get_promotion() {
                Some(foreign::Piece::Knight) => Promotion::Knight,
                Some(foreign::Piece::Bishop) => Promotion::Bishop,
                Some(foreign::Piece::Rook) => Promotion::Rook,
                Some(foreign::Piece::Queen) => Promotion::Queen,
                _ => Promotion::None,
            },
        }
    }
}

impl Into<foreign::ChessMove> for Move {
    fn into(self) -> foreign::ChessMove {
        foreign::ChessMove::new(self.from.into(), self.to.into(), self.promotion.into())
    }
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

use crate::{ParsePromotionError, ParseSquareError, Position, Promotion, Square};
use derive_more::{Display, Error, From};
use shakmaty as sm;
use std::str::FromStr;
use tracing::instrument;
use vampirc_uci::UciMove;

/// A chess move.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(fmt = "{}{}{}", _0, _1, _2)]
pub struct Move(Square, Square, Promotion);

impl Move {
    /// The source [`Square`].
    pub fn whence(&self) -> Square {
        self.0
    }

    /// The destination [`Square`].
    pub fn whither(&self) -> Square {
        self.1
    }

    /// The [`Promotion`] specifier.
    pub fn promotion(&self) -> Promotion {
        self.2
    }
}

/// Represents an illegal [`Move`] in a given [`Position`].
#[derive(Debug, Display, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(fmt = "move `{}` is illegal in position `{}`", _0, _1)]
pub struct IllegalMove(pub Move, pub Position);

/// The reason why parsing [`Move`] failed.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Error, From)]
#[display(fmt = "unable to parse move; {}")]
pub enum ParseMoveError {
    #[display(fmt = "invalid 'from' square")]
    #[from(ignore)]
    InvalidFromSquare(ParseSquareError),

    #[display(fmt = "invalid 'to' square")]
    #[from(ignore)]
    InvalidToSquare(ParseSquareError),

    #[display(fmt = "invalid promotion")]
    InvalidPromotion(ParsePromotionError),
}

impl FromStr for Move {
    type Err = ParseMoveError;

    #[instrument(level = "trace", err)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ParseMoveError::*;

        let i = s.char_indices().nth(2).map_or_else(|| s.len(), |(i, _)| i);
        let j = s.char_indices().nth(4).map_or_else(|| s.len(), |(i, _)| i);

        Ok(Move(
            s[..i].parse().map_err(InvalidFromSquare)?,
            s[i..j].parse().map_err(InvalidToSquare)?,
            s[j..].parse()?,
        ))
    }
}

#[doc(hidden)]
impl From<Move> for UciMove {
    fn from(m: Move) -> Self {
        UciMove {
            from: m.whence().into(),
            to: m.whither().into(),
            promotion: m.promotion().into(),
        }
    }
}

#[doc(hidden)]
impl From<UciMove> for Move {
    fn from(m: UciMove) -> Self {
        Move(m.from.into(), m.to.into(), m.promotion.into())
    }
}

#[doc(hidden)]
impl From<sm::uci::Uci> for Move {
    fn from(m: sm::uci::Uci) -> Self {
        match m {
            sm::uci::Uci::Normal {
                from,
                to,
                promotion,
            } => Move(from.into(), to.into(), promotion.into()),

            v => panic!("unexpected {:?}", v),
        }
    }
}

#[doc(hidden)]
impl From<Move> for sm::uci::Uci {
    fn from(m: Move) -> Self {
        sm::uci::Uci::Normal {
            from: m.whence().into(),
            to: m.whither().into(),
            promotion: m.promotion().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn move_serializes_to_pure_coordinate_notation(m: Move) {
            assert_eq!(m.to_string(), UciMove::from(m).to_string());
        }

        #[test]
        fn parsing_printed_move_is_an_identity(m: Move) {
            assert_eq!(m.to_string().parse(), Ok(m));
        }

        #[test]
        fn parsing_move_fails_if_from_square_is_invalid(f in "[^a-h]{2}|[^1-8]{2}", t: Square, p: Promotion) {
            use ParseMoveError::*;
            let s = [f.clone(), t.to_string(), p.to_string()].concat();
            assert_eq!(s.parse::<Move>(), Err(InvalidFromSquare(f.parse::<Square>().unwrap_err())));
        }

        #[test]
        fn parsing_move_fails_if_to_square_is_invalid(f: Square, t in "[^a-h]{2}|[^1-8]{2}", p: Promotion) {
            use ParseMoveError::*;
            let s = [f.to_string(), t.clone(), p.to_string()].concat();
            assert_eq!(s.parse::<Move>(), Err(InvalidToSquare(t.parse::<Square>().unwrap_err())));
        }

        #[test]
        fn parsing_move_fails_if_promotion_is_invalid(f: Square, t: Square, p in "[^nbrq]+") {
            use ParseMoveError::*;
            let s = [f.to_string(), t.to_string(), p.clone()].concat();
            assert_eq!(s.parse::<Move>(), Err(InvalidPromotion(p.parse::<Promotion>().unwrap_err())));
        }

        #[test]
        fn move_has_an_equivalent_vampirc_uci_representation(m: Move) {
            assert_eq!(Move::from(UciMove::from(m)), m);
        }

        #[test]
        fn move_has_an_equivalent_shakmaty_representation(m: Move) {
            assert_eq!(Move::from(sm::uci::Uci::from(m)), m);
        }
    }
}

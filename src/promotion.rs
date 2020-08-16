use crate::foreign;
use derive_more::{Display, Error};
use std::str::FromStr;

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

/// The reason parsing a [`Promotion`] failed.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Error)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(
    fmt = "unable to parse promotion, expected one of four characters: '{}', '{}', '{}', '{}'",
    "Promotion::Knight",
    "Promotion::Bishop",
    "Promotion::Rook",
    "Promotion::Queen"
)]
pub struct ParsePromotionError;

impl FromStr for Promotion {
    type Err = ParsePromotionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "n" => Ok(Promotion::Knight),
            "b" => Ok(Promotion::Bishop),
            "r" => Ok(Promotion::Rook),
            "q" => Ok(Promotion::Queen),
            _ => Err(ParsePromotionError),
        }
    }
}

impl Into<foreign::Piece> for Promotion {
    fn into(self) -> foreign::Piece {
        use Promotion::*;
        match self {
            Knight => foreign::Piece::Knight,
            Bishop => foreign::Piece::Bishop,
            Rook => foreign::Piece::Rook,
            Queen => foreign::Piece::Queen,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn parsing_printed_promotion_is_an_identity(p: Promotion) {
            assert_eq!(p.to_string().parse(), Ok(p));
        }

        #[test]
        fn parsing_promotion_fails_except_for_one_of_four_letters(p in "[^nbrq]*") {
            assert_eq!(p.parse::<Promotion>(), Err(ParsePromotionError));
        }
    }
}

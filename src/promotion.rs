use crate::{foreign, Role};
use derive_more::{Display, Error, From};
use std::str::FromStr;

/// A promotion specifier.
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
    #[display(fmt = "")]
    None,
}

/// The reason parsing [`Promotion`] failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Hash, Error, From)]
#[display(
    fmt = "unable to parse promotion from `{}`; expected either one of four characters `[{}{}{}{}]` or the empty string",
    _0,
    Promotion::Knight,
    Promotion::Bishop,
    Promotion::Rook,
    Promotion::Queen
)]
#[from(forward)]
pub struct ParsePromotionError(#[error(not(source))] pub String);

impl FromStr for Promotion {
    type Err = ParsePromotionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "n" => Ok(Promotion::Knight),
            "b" => Ok(Promotion::Bishop),
            "r" => Ok(Promotion::Rook),
            "q" => Ok(Promotion::Queen),
            "" => Ok(Promotion::None),
            _ => Err(s.into()),
        }
    }
}

impl From<Promotion> for &'static str {
    fn from(p: Promotion) -> Self {
        match p {
            Promotion::Knight => "n",
            Promotion::Bishop => "b",
            Promotion::Rook => "r",
            Promotion::Queen => "q",
            Promotion::None => "",
        }
    }
}

impl From<Promotion> for Option<Role> {
    fn from(p: Promotion) -> Self {
        match p {
            Promotion::Knight => Some(Role::Knight),
            Promotion::Bishop => Some(Role::Bishop),
            Promotion::Rook => Some(Role::Rook),
            Promotion::Queen => Some(Role::Queen),
            Promotion::None => None,
        }
    }
}

impl From<Promotion> for Option<foreign::Piece> {
    fn from(p: Promotion) -> Self {
        match p {
            Promotion::Knight => Some(foreign::Piece::Knight),
            Promotion::Bishop => Some(foreign::Piece::Bishop),
            Promotion::Rook => Some(foreign::Piece::Rook),
            Promotion::Queen => Some(foreign::Piece::Queen),
            Promotion::None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn every_promotion_has_an_associated_static_str(p: Promotion) {
            assert_eq!(<&str>::from(p), p.to_string());
        }

        #[test]
        fn parsing_printed_promotion_is_an_identity(p: Promotion) {
            assert_eq!(p.to_string().parse(), Ok(p));
        }

        #[test]
        fn parsing_promotion_fails_except_for_one_of_four_letters(s in "[^nbrq]+") {
            assert_eq!(s.parse::<Promotion>(), Err(ParsePromotionError(s)));
        }
    }
}

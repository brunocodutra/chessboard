use derive_more::{Display, Error};
use shakmaty as sm;
use std::str::FromStr;
use vampirc_uci::UciPiece;

#[cfg(test)]
use test_strategy::Arbitrary;

/// A promotion specifier.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(Arbitrary))]
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
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(
    fmt = "expected either one of four characters `[{}{}{}{}]` or the empty string",
    Promotion::Knight,
    Promotion::Bishop,
    Promotion::Rook,
    Promotion::Queen
)]
pub struct ParsePromotionError;

impl FromStr for Promotion {
    type Err = ParsePromotionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "" => Ok(Promotion::None),
            "n" => Ok(Promotion::Knight),
            "b" => Ok(Promotion::Bishop),
            "r" => Ok(Promotion::Rook),
            "q" => Ok(Promotion::Queen),
            _ => Err(ParsePromotionError),
        }
    }
}

#[doc(hidden)]
impl From<Promotion> for Option<UciPiece> {
    fn from(p: Promotion) -> Self {
        match p {
            Promotion::None => None,
            Promotion::Knight => Some(UciPiece::Knight),
            Promotion::Bishop => Some(UciPiece::Bishop),
            Promotion::Rook => Some(UciPiece::Rook),
            Promotion::Queen => Some(UciPiece::Queen),
        }
    }
}

#[doc(hidden)]
impl From<Option<UciPiece>> for Promotion {
    fn from(p: Option<UciPiece>) -> Self {
        match p {
            None => Promotion::None,
            Some(UciPiece::Knight) => Promotion::Knight,
            Some(UciPiece::Bishop) => Promotion::Bishop,
            Some(UciPiece::Rook) => Promotion::Rook,
            Some(UciPiece::Queen) => Promotion::Queen,
            Some(v) => panic!("unexpected {:?}", v),
        }
    }
}

#[doc(hidden)]
impl From<Option<sm::Role>> for Promotion {
    fn from(p: Option<sm::Role>) -> Self {
        match p {
            None => Promotion::None,
            Some(sm::Role::Knight) => Promotion::Knight,
            Some(sm::Role::Bishop) => Promotion::Bishop,
            Some(sm::Role::Rook) => Promotion::Rook,
            Some(sm::Role::Queen) => Promotion::Queen,
            Some(v) => panic!("unexpected {:?}", v),
        }
    }
}

#[doc(hidden)]
impl From<Promotion> for Option<sm::Role> {
    fn from(p: Promotion) -> Self {
        match p {
            Promotion::None => None,
            Promotion::Knight => Some(sm::Role::Knight),
            Promotion::Bishop => Some(sm::Role::Bishop),
            Promotion::Rook => Some(sm::Role::Rook),
            Promotion::Queen => Some(sm::Role::Queen),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn parsing_printed_promotion_is_an_identity(p: Promotion) {
        assert_eq!(p.to_string().parse(), Ok(p));
    }

    #[proptest]
    fn parsing_promotion_fails_for_upper_case_letter(#[strategy("[A-Z]")] s: String) {
        assert_eq!(s.parse::<Promotion>(), Err(ParsePromotionError));
    }

    #[proptest]
    fn parsing_promotion_fails_except_for_one_of_four_letters(#[strategy("[^nbrq]+")] s: String) {
        assert_eq!(s.parse::<Promotion>(), Err(ParsePromotionError));
    }

    #[proptest]
    fn promotion_has_an_equivalent_vampirc_uci_representation(p: Promotion) {
        assert_eq!(Promotion::from(Option::<UciPiece>::from(p)), p);
    }

    #[proptest]
    fn promotion_has_an_equivalent_shakmaty_representation(p: Promotion) {
        assert_eq!(Promotion::from(Option::<sm::Role>::from(p)), p);
    }
}

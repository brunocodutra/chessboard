use crate::chess::Role;
use crate::util::{Binary, Bits};
use derive_more::{Display, Error};
use shakmaty as sm;
use test_strategy::Arbitrary;
use vampirc_uci::UciPiece;

/// A promotion specifier.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary)]
pub enum Promotion {
    #[display(fmt = "")]
    None,
    #[display(fmt = "n")]
    Knight,
    #[display(fmt = "b")]
    Bishop,
    #[display(fmt = "r")]
    Rook,
    #[display(fmt = "q")]
    Queen,
}

/// The reason why decoding [`Promotion`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "not a valid promotion")]
pub struct DecodePromotionError;

impl Binary for Promotion {
    type Bits = Bits<u8, 3>;
    type Error = DecodePromotionError;

    fn encode(&self) -> Self::Bits {
        Bits::new(*self as _)
    }

    fn decode(bits: Self::Bits) -> Result<Self, Self::Error> {
        use Promotion::*;
        [None, Knight, Bishop, Rook, Queen]
            .into_iter()
            .nth(bits.get() as _)
            .ok_or(DecodePromotionError)
    }
}

impl From<Promotion> for Option<Role> {
    fn from(p: Promotion) -> Self {
        match p {
            Promotion::None => None,
            Promotion::Knight => Some(Role::Knight),
            Promotion::Bishop => Some(Role::Bishop),
            Promotion::Rook => Some(Role::Rook),
            Promotion::Queen => Some(Role::Queen),
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
            Some(v) => panic!("unexpected {v:?}"),
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
            Some(v) => panic!("unexpected {v:?}"),
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
    fn decoding_encoded_promotion_is_an_identity(p: Promotion) {
        assert_eq!(Promotion::decode(p.encode()), Ok(p));
    }

    #[proptest]
    fn decoding_promotion_fails_for_invalid_bits(#[strategy(5u8..8)] n: u8) {
        let b = <Promotion as Binary>::Bits::new(n as _);
        assert_eq!(Promotion::decode(b), Err(DecodePromotionError));
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

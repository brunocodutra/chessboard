use crate::util::{Binary, Bits};
use derive_more::{Display, Error};
use shakmaty as sm;
use vampirc_uci::UciPiece;

/// Denotes the type of a chess [`Piece`][`crate::Piece`].
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(u8)]
pub enum Role {
    #[display(fmt = "p")]
    Pawn,
    #[display(fmt = "n")]
    Knight,
    #[display(fmt = "b")]
    Bishop,
    #[display(fmt = "r")]
    Rook,
    #[display(fmt = "q")]
    Queen,
    #[display(fmt = "k")]
    King,
}

/// The reason why decoding [`Role`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "not a valid role")]
pub struct DecodeRoleError;

impl Binary for Role {
    type Bits = Bits<u8, 3>;
    type Error = DecodeRoleError;

    fn encode(&self) -> Self::Bits {
        Bits::new(*self as _)
    }

    fn decode(bits: Self::Bits) -> Result<Self, Self::Error> {
        use Role::*;
        [Pawn, Knight, Bishop, Rook, Queen, King]
            .into_iter()
            .nth(bits.get() as _)
            .ok_or(DecodeRoleError)
    }
}

#[doc(hidden)]
impl From<Role> for UciPiece {
    fn from(r: Role) -> Self {
        match r {
            Role::Pawn => UciPiece::Pawn,
            Role::Knight => UciPiece::Knight,
            Role::Bishop => UciPiece::Bishop,
            Role::Rook => UciPiece::Rook,
            Role::Queen => UciPiece::Queen,
            Role::King => UciPiece::King,
        }
    }
}

#[doc(hidden)]
impl From<UciPiece> for Role {
    fn from(r: UciPiece) -> Self {
        match r {
            UciPiece::Pawn => Role::Pawn,
            UciPiece::Knight => Role::Knight,
            UciPiece::Bishop => Role::Bishop,
            UciPiece::Rook => Role::Rook,
            UciPiece::Queen => Role::Queen,
            UciPiece::King => Role::King,
        }
    }
}

#[doc(hidden)]
impl From<Role> for sm::Role {
    fn from(r: Role) -> Self {
        match r {
            Role::Pawn => sm::Role::Pawn,
            Role::Knight => sm::Role::Knight,
            Role::Bishop => sm::Role::Bishop,
            Role::Rook => sm::Role::Rook,
            Role::Queen => sm::Role::Queen,
            Role::King => sm::Role::King,
        }
    }
}

#[doc(hidden)]
impl From<sm::Role> for Role {
    fn from(r: sm::Role) -> Self {
        match r {
            sm::Role::Pawn => Role::Pawn,
            sm::Role::Knight => Role::Knight,
            sm::Role::Bishop => Role::Bishop,
            sm::Role::Rook => Role::Rook,
            sm::Role::Queen => Role::Queen,
            sm::Role::King => Role::King,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn decoding_encoded_role_is_an_identity(r: Role) {
        assert_eq!(Role::decode(r.encode()), Ok(r));
    }

    #[proptest]
    fn decoding_role_fails_for_invalid_bits(#[strategy(6u8..8)] n: u8) {
        let b = <Role as Binary>::Bits::new(n as _);
        assert_eq!(Role::decode(b), Err(DecodeRoleError));
    }

    #[proptest]
    fn role_has_an_equivalent_shakmaty_representation(r: Role) {
        assert_eq!(Role::from(sm::Role::from(r)), r);
    }
}

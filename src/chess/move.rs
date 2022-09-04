use super::{Promotion, Square};
use crate::util::{Binary, Bits, Register};
use derive_more::{Display, Error};
use shakmaty as sm;
use vampirc_uci::UciMove;

/// A chess move.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, filter(#self.0 != #self.1))]
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

/// The reason why decoding [`Move`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "`{}` is not a valid Move", _0)]
pub struct DecodeMoveError(#[error(not(source))] <Move as Binary>::Register);

impl Binary for Move {
    type Register = Bits<u16, 15>;
    type Error = DecodeMoveError;

    fn encode(&self) -> Self::Register {
        let mut register = Bits::default();
        let (whence, rest) = register.split_at_mut(<Square as Binary>::Register::WIDTH);
        let (whither, rest) = rest.split_at_mut(<Square as Binary>::Register::WIDTH);
        let (promotion, _) = rest.split_at_mut(<Promotion as Binary>::Register::WIDTH);

        whence.clone_from_bitslice(&self.whence().encode());
        whither.clone_from_bitslice(&self.whither().encode());
        promotion.clone_from_bitslice(&self.promotion().encode());

        register
    }

    fn decode(register: Self::Register) -> Result<Self, Self::Error> {
        let (whence, rest) = register.split_at(<Square as Binary>::Register::WIDTH);
        let (whither, rest) = rest.split_at(<Square as Binary>::Register::WIDTH);
        let (promotion, _) = rest.split_at(<Promotion as Binary>::Register::WIDTH);

        Ok(Move(
            Square::decode(whence.into()).map_err(|_| DecodeMoveError(register))?,
            Square::decode(whither.into()).map_err(|_| DecodeMoveError(register))?,
            Promotion::decode(promotion.into()).map_err(|_| DecodeMoveError(register))?,
        ))
    }
}

#[doc(hidden)]
impl From<UciMove> for Move {
    fn from(m: UciMove) -> Self {
        Move(m.from.into(), m.to.into(), m.promotion.into())
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
    use std::mem::size_of;
    use test_strategy::proptest;

    #[proptest]
    fn move_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<Move>>(), size_of::<Move>());
    }

    #[proptest]
    fn decoding_encoded_move_is_an_identity(m: Move) {
        assert_eq!(Move::decode(m.encode()), Ok(m));
    }

    #[proptest]
    fn decoding_move_fails_for_invalid_register(#[any(64 * 64 * 5)] b: Bits<u16, 15>) {
        assert_eq!(Move::decode(b), Err(DecodeMoveError(b)));
    }

    #[proptest]
    fn move_serializes_to_pure_coordinate_notation(m: Move) {
        assert_eq!(m.to_string(), UciMove::from(m).to_string());
    }

    #[proptest]
    fn move_has_an_equivalent_vampirc_uci_representation(m: Move) {
        assert_eq!(Move::from(UciMove::from(m)), m);
    }

    #[proptest]
    fn move_has_an_equivalent_shakmaty_representation(m: Move) {
        assert_eq!(Move::from(sm::uci::Uci::from(m)), m);
    }
}

use super::{Promotion, Square};
use crate::util::{Binary, Bits};
use derive_more::{DebugCustom, Display, Error};
use shakmaty as sm;
use test_strategy::Arbitrary;
use vampirc_uci::UciMove;

/// A chess move.
#[derive(DebugCustom, Display, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[filter(#self.0 != #self.1)]
#[debug(fmt = "Move({self})")]
#[display(fmt = "{_0}{_1}{_2}")]
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
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "not a valid Move")]
pub struct DecodeMoveError;

impl From<<Square as Binary>::Error> for DecodeMoveError {
    fn from(_: <Square as Binary>::Error) -> Self {
        DecodeMoveError
    }
}

impl From<<Promotion as Binary>::Error> for DecodeMoveError {
    fn from(_: <Promotion as Binary>::Error) -> Self {
        DecodeMoveError
    }
}

impl Binary for Move {
    type Bits = Bits<u16, 15>;
    type Error = DecodeMoveError;

    fn encode(&self) -> Self::Bits {
        let mut bits = Bits::default();
        bits.push(self.promotion().encode());
        bits.push(self.whither().encode());
        bits.push(self.whence().encode());
        bits
    }

    fn decode(mut bits: Self::Bits) -> Result<Self, Self::Error> {
        Ok(Move(
            Square::decode(bits.pop())?,
            Square::decode(bits.pop())?,
            Promotion::decode(bits.pop())?,
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

            v => panic!("unexpected {v:?}"),
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
    fn decoding_move_fails_for_invalid_bits(#[strategy(20480u16..32768)] n: u16) {
        let b = <Move as Binary>::Bits::new(n as _);
        assert_eq!(Move::decode(b), Err(DecodeMoveError));
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

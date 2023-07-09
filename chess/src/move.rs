use crate::{Promotion, Role, Square};
use bitflags::bitflags;
use derive_more::{DebugCustom, Deref, Display, Error};
use shakmaty as sm;
use std::str::FromStr;
use test_strategy::Arbitrary;
use util::{Binary, Bits};
use vampirc_uci::UciMove;

bitflags! {
    /// Characteristics of a [`Move`] in the context of a [`Position`][`crate::Position`].
    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
    pub struct MoveKind: u8 {
        const ANY =         0b00000001;
        const CASTLE =      0b00000010;
        const PROMOTION =   0b00000100;
        const CAPTURE =     0b00001000;
        const EN_PASSANT =  0b00010000;
    }
}

#[doc(hidden)]
impl From<&sm::Move> for MoveKind {
    #[inline]
    fn from(m: &sm::Move) -> Self {
        let mut kind = Self::ANY;

        if m.is_castle() {
            kind |= MoveKind::CASTLE
        }

        if m.is_promotion() {
            kind |= MoveKind::PROMOTION
        }

        if m.is_capture() {
            kind |= MoveKind::CAPTURE;
        }

        if m.is_en_passant() {
            kind |= MoveKind::EN_PASSANT;
        }

        kind
    }
}

#[doc(hidden)]
impl From<&mut sm::Move> for MoveKind {
    #[inline]
    fn from(m: &mut sm::Move) -> Self {
        (&*m).into()
    }
}

/// A chess move.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary, Deref)]
pub struct MoveContext(#[deref] pub Move, pub Role, pub Option<(Role, Square)>);

impl MoveContext {
    /// The [`Role`] of the piece moved.
    #[inline]
    pub fn role(&self) -> Role {
        self.1
    }

    /// The [`Role`] of the piece captured.
    #[inline]
    pub fn capture(&self) -> Option<(Role, Square)> {
        self.2
    }
}

/// A chess move in [pure coordinate notation].
///
/// [pure coordinate notation]: https://www.chessprogramming.org/Algebraic_Chess_Notation#Pure_coordinate_notation
#[derive(DebugCustom, Display, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[filter(#self.0 != #self.1)]
#[debug(fmt = "Move({self})")]
#[display(fmt = "{_0}{_1}{_2}")]
pub struct Move(pub Square, pub Square, pub Promotion);

impl Move {
    /// The source [`Square`].
    #[inline]
    pub fn whence(&self) -> Square {
        self.0
    }

    /// The destination [`Square`].
    #[inline]
    pub fn whither(&self) -> Square {
        self.1
    }

    /// The [`Promotion`] specifier.
    #[inline]
    pub fn promotion(&self) -> Promotion {
        self.2
    }
}

/// The reason why the string is not valid move.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(fmt = "failed to parse move")]

pub struct ParseMoveError;

impl FromStr for Move {
    type Err = ParseMoveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<sm::uci::Uci>() {
            Ok(m) => Ok(m.into()),
            Err(_) => Err(ParseMoveError),
        }
    }
}

/// The reason why decoding [`Move`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "not a valid move")]
pub struct DecodeMoveError;

impl From<<Square as Binary>::Error> for DecodeMoveError {
    #[inline]
    fn from(_: <Square as Binary>::Error) -> Self {
        DecodeMoveError
    }
}

impl From<<Promotion as Binary>::Error> for DecodeMoveError {
    #[inline]
    fn from(_: <Promotion as Binary>::Error) -> Self {
        DecodeMoveError
    }
}

impl Binary for Move {
    type Bits = Bits<u16, 15>;
    type Error = DecodeMoveError;

    #[inline]
    fn encode(&self) -> Self::Bits {
        let mut bits = Bits::default();
        bits.push(self.promotion().encode());
        bits.push(self.whither().encode());
        bits.push(self.whence().encode());
        bits
    }

    #[inline]
    fn decode(mut bits: Self::Bits) -> Result<Self, Self::Error> {
        Ok(Move(
            Square::decode(bits.pop())?,
            Square::decode(bits.pop())?,
            Promotion::decode(bits.pop())?,
        ))
    }
}

#[doc(hidden)]
impl From<sm::Move> for MoveContext {
    #[inline]
    fn from(m: sm::Move) -> Self {
        match m {
            sm::Move::Normal {
                role,
                from,
                capture,
                to,
                promotion,
            } => MoveContext(
                Move(from.into(), to.into(), promotion.into()),
                role.into(),
                capture.map(|r| (r.into(), to.into())),
            ),

            sm::Move::EnPassant { from, to } => MoveContext(
                Move(from.into(), to.into(), Promotion::None),
                Role::Pawn,
                Some((
                    Role::Pawn,
                    Square::new(to.file().into(), from.rank().into()),
                )),
            ),

            m @ sm::Move::Castle { .. } => MoveContext(
                m.to_uci(sm::CastlingMode::Standard).into(),
                Role::King,
                None,
            ),

            v => panic!("unexpected {v:?}"),
        }
    }
}

#[doc(hidden)]
impl From<UciMove> for Move {
    #[inline]
    fn from(m: UciMove) -> Self {
        Move(m.from.into(), m.to.into(), m.promotion.into())
    }
}

#[doc(hidden)]
impl From<Move> for UciMove {
    #[inline]
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
    #[inline]
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
    #[inline]
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
    fn parsing_printed_move_is_an_identity(m: Move) {
        assert_eq!(m.to_string().parse(), Ok(m));
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

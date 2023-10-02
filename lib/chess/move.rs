use crate::chess::{Promotion, Role, Square};
use crate::util::{Binary, Bits};
use derive_more::{DebugCustom, Deref, Display, Error};
use shakmaty as sm;
use std::str::FromStr;
use vampirc_uci::UciMove;

/// The context of a chess move.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deref)]
pub struct MoveContext(#[deref] pub Move, pub Role, pub Option<(Role, Square)>);

impl MoveContext {
    /// The [`Role`] of the piece moved.
    pub fn role(&self) -> Role {
        self.1
    }

    /// Whether this is a promotion move.
    pub fn is_promotion(&self) -> bool {
        self.promotion() != Promotion::None
    }
    /// The [`Role`] of the piece captured.
    pub fn capture(&self) -> Option<(Role, Square)> {
        self.2
    }

    /// Whether this is a castling move.
    pub fn is_castling(&self) -> bool {
        self.role() == Role::King && (self.whence().file() - self.whither().file()).abs() > 1
    }

    /// Whether this is a capture move.
    pub fn is_capture(&self) -> bool {
        self.capture().is_some()
    }

    /// Whether this is an en passant capture move.
    pub fn is_en_passant(&self) -> bool {
        self.capture().is_some_and(|(_, s)| self.whither() != s)
    }

    /// Whether this move is neither a capture nor a promotion.
    pub fn is_quiet(&self) -> bool {
        !(self.is_capture() || self.is_promotion())
    }
}

#[doc(hidden)]
impl From<sm::Move> for MoveContext {
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

            ref m @ sm::Move::Castle { .. } => MoveContext(
                m.to_uci(sm::CastlingMode::Standard).into(),
                Role::King,
                None,
            ),

            v => panic!("unexpected {v:?}"),
        }
    }
}

#[doc(hidden)]
impl From<MoveContext> for sm::Move {
    fn from(mc @ MoveContext(m, role, capture): MoveContext) -> Self {
        if mc.is_castling() {
            sm::Move::Castle {
                king: m.whence().into(),
                rook: if m.whence() < m.whither() {
                    sm::Square::from_coords(sm::File::H, m.whither().rank().into())
                } else {
                    sm::Square::from_coords(sm::File::A, m.whither().rank().into())
                },
            }
        } else if mc.is_en_passant() {
            sm::Move::EnPassant {
                from: m.whence().into(),
                to: m.whither().into(),
            }
        } else {
            sm::Move::Normal {
                role: role.into(),
                from: m.whence().into(),
                capture: capture.map(|(r, _)| r.into()),
                to: m.whither().into(),
                promotion: m.promotion().into(),
            }
        }
    }
}

/// A chess move in [pure coordinate notation].
///
/// [pure coordinate notation]: https://www.chessprogramming.org/Algebraic_Chess_Notation#Pure_coordinate_notation
#[derive(DebugCustom, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, filter(#self.0 != #self.1))]
#[debug(fmt = "Move({self})")]
#[display(fmt = "{_0}{_1}{_2}")]
#[repr(align(4))]
pub struct Move(pub Square, pub Square, pub Promotion);

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
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "not a valid move")]
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
    use crate::chess::Position;
    use proptest::sample::Selector;
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

    #[proptest]
    fn move_context_has_an_equivalent_shakmaty_representation(
        #[filter(#_pos.outcome().is_none())] _pos: Position,
        #[map(|s: Selector| s.select(#_pos.moves()))] mc: MoveContext,
    ) {
        assert_eq!(MoveContext::from(<sm::Move as From<_>>::from(mc)), mc);
    }

    #[proptest]
    fn castling_moves_are_never_captures(
        #[filter(#_pos.outcome().is_none())] _pos: Position,
        #[map(|s: Selector| s.select(#_pos.moves()))] mc: MoveContext,
    ) {
        assert!(!mc.is_castling() || !mc.is_capture());
    }

    #[proptest]
    fn castling_moves_are_never_promotions(
        #[filter(#_pos.outcome().is_none())] _pos: Position,
        #[map(|s: Selector| s.select(#_pos.moves()))] mc: MoveContext,
    ) {
        assert!(!mc.is_castling() || !mc.is_promotion());
    }

    #[proptest]
    fn en_passant_moves_are_always_captures(
        #[filter(#_pos.outcome().is_none())] _pos: Position,
        #[map(|s: Selector| s.select(#_pos.moves()))] mc: MoveContext,
    ) {
        assert!(!mc.is_en_passant() || mc.is_capture());
    }

    #[proptest]
    fn captures_are_never_quiet(
        #[filter(#_pos.outcome().is_none())] _pos: Position,
        #[map(|s: Selector| s.select(#_pos.moves()))] mc: MoveContext,
    ) {
        assert!(!mc.is_capture() || !mc.is_quiet());
    }

    #[proptest]
    fn promotions_are_never_quiet(
        #[filter(#_pos.outcome().is_none())] _pos: Position,
        #[map(|s: Selector| s.select(#_pos.moves()))] mc: MoveContext,
    ) {
        assert!(!mc.is_promotion() || !mc.is_quiet());
    }
}

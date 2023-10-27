use crate::chess::{Role, Square};
use crate::util::{Assume, Binary, Bits};
use derive_more::{Display, Error};
use shakmaty as sm;
use std::{fmt, num::NonZeroU16, ops::RangeBounds};
use vampirc_uci::UciMove;

/// A chess move.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Move(NonZeroU16);

impl Move {
    fn bits<R: RangeBounds<u32>>(&self, r: R) -> Bits<u16, 16> {
        Bits::new(self.0.get()).slice(r)
    }

    /// The source [`Square`].
    pub fn whence(&self) -> Square {
        Square::decode(self.bits(10..).pop()).assume()
    }

    /// The destination [`Square`].
    pub fn whither(&self) -> Square {
        Square::decode(self.bits(4..).pop()).assume()
    }

    /// The promotion specifier.
    pub fn promotion(&self) -> Option<Role> {
        if self.is_promotion() {
            Some(Role::from_index(self.bits(..2).get() as u8 + 1))
        } else {
            None
        }
    }

    /// Whether this is a promotion move.
    pub fn is_promotion(&self) -> bool {
        self.bits(3..=3).get() != 0
    }

    /// Whether this is a castling move.
    pub fn is_castling(&self) -> bool {
        self.bits(..4).get() == 0b0001
    }

    /// Whether this is an en passant capture move.
    pub fn is_en_passant(&self) -> bool {
        self.bits(..4).get() == 0b0011
    }

    /// Whether this is a capture move.
    pub fn is_capture(&self) -> bool {
        self.bits(2..=2).get() != 0 || (self.bits(1..=1).get() != 0 && !self.is_promotion())
    }

    /// Whether this move is neither a capture nor a promotion.
    pub fn is_quiet(&self) -> bool {
        !(self.is_capture() || self.is_promotion())
    }
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.whence(), self.whither())?;

        if let Some(r) = self.promotion() {
            write!(f, "={}", r)?;
        }

        if self.is_en_passant() {
            write!(f, "^")?;
        } else if self.is_capture() {
            write!(f, "x")?;
        } else if self.is_castling() {
            write!(f, "~")?;
        }

        Ok(())
    }
}

// The reason why decoding [`Move`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "not a valid move")]
pub struct DecodeMoveError;

impl Binary for Move {
    type Bits = Bits<u16, 16>;
    type Error = DecodeMoveError;

    fn encode(&self) -> Self::Bits {
        self.bits(..)
    }

    fn decode(bits: Self::Bits) -> Result<Self, Self::Error> {
        NonZeroU16::new(bits.get()).map_or(Err(DecodeMoveError), |n| Ok(Move(n)))
    }
}

#[doc(hidden)]
impl From<sm::Move> for Move {
    fn from(m: sm::Move) -> Self {
        let mut bits = Bits::<_, 16>::default();
        bits.push(Square::from(m.from().assume()).encode());

        match m {
            sm::Move::Normal {
                to,
                capture: None,
                promotion: None,
                ..
            } => {
                bits.push(Square::from(to).encode());
                bits.push(Bits::<u8, 4>::new(0));
            }

            sm::Move::Normal {
                to,
                capture: Some(sm::Role::Pawn),
                promotion: None,
                ..
            } => {
                bits.push(Square::from(to).encode());
                bits.push(Bits::<u8, 4>::new(0b0010));
            }

            sm::Move::Normal {
                to,
                capture,
                promotion,
                ..
            } => {
                bits.push(Square::from(to).encode());
                bits.push(Bits::<u8, 1>::new(promotion.is_some() as _));
                bits.push(Bits::<u8, 1>::new(capture.is_some() as _));
                bits.push(Bits::<u8, 2>::new(promotion.map_or(0, |r| r as u8 - 2)));
            }

            sm::Move::EnPassant { to, .. } => {
                bits.push(Square::from(to).encode());
                bits.push(Bits::<u8, 4>::new(0b0011))
            }

            sm::Move::Castle { king, rook } => {
                if rook > king {
                    bits.push(Square::new(sm::File::G.into(), king.rank().into()).encode())
                } else {
                    bits.push(Square::new(sm::File::C.into(), king.rank().into()).encode())
                };

                bits.push(Bits::<u8, 4>::new(0b0001))
            }

            _ => unreachable!(),
        }

        Move(NonZeroU16::new(bits.get()).assume())
    }
}

#[doc(hidden)]
impl From<Move> for UciMove {
    fn from(m: Move) -> Self {
        UciMove {
            from: m.whence().into(),
            to: m.whither().into(),
            promotion: m.promotion().map(Role::into),
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
    fn decoding_move_fails_for_invalid_bits() {
        assert_eq!(Move::decode(Bits::default()), Err(DecodeMoveError));
    }

    #[proptest]
    fn castling_moves_are_never_captures(m: Move) {
        assert!(!m.is_castling() || !m.is_capture());
    }

    #[proptest]
    fn castling_moves_are_never_promotions(m: Move) {
        assert!(!m.is_castling() || !m.is_promotion());
    }

    #[proptest]
    fn en_passant_moves_are_always_captures(m: Move) {
        assert!(!m.is_en_passant() || m.is_capture());
    }

    #[proptest]
    fn captures_are_never_quiet(m: Move) {
        assert!(!m.is_capture() || !m.is_quiet());
    }

    #[proptest]
    fn promotions_are_never_quiet(m: Move) {
        assert!(!m.is_promotion() || !m.is_quiet());
    }
}

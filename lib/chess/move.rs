use crate::chess::{Role, Square};
use crate::util::{Assume, Binary, Bits};
use derive_more::{Display, Error};
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

    /// Constructs a castling move.
    pub fn castling(whence: Square, whither: Square) -> Self {
        let mut bits = Bits::<_, 16>::default();
        bits.push(whence.encode());
        bits.push(whither.encode());
        bits.push(Bits::<u8, 4>::new(0b0001));
        Move(NonZeroU16::new(bits.get()).assume())
    }

    /// Constructs an en passant move.
    pub fn en_passant(whence: Square, whither: Square) -> Self {
        let mut bits = Bits::<_, 16>::default();
        bits.push(whence.encode());
        bits.push(whither.encode());
        bits.push(Bits::<u8, 4>::new(0b0011));
        Move(NonZeroU16::new(bits.get()).assume())
    }

    /// Constructs a regular move.
    pub fn regular(
        whence: Square,
        whither: Square,
        promotion: Option<Role>,
        capture: Option<Role>,
    ) -> Self {
        let mut bits = Bits::<_, 16>::default();
        bits.push(whence.encode());
        bits.push(whither.encode());

        if capture == Some(Role::Pawn) {
            bits.push(Bits::<u8, 4>::new(0b0010))
        } else {
            bits.push(Bits::<u8, 1>::new(promotion.is_some() as _));
            bits.push(Bits::<u8, 1>::new(capture.is_some() as _));
            bits.push(Bits::<u8, 2>::new(
                promotion.map_or(0, |r| r.index().clamp(1, 4) - 1),
            ));
        }

        Move(NonZeroU16::new(bits.get()).assume())
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
    use crate::chess::{Bitboard, Position};
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
    fn decoding_move_fails_for_invalid_bits() {
        assert_eq!(Move::decode(Bits::default()), Err(DecodeMoveError));
    }

    #[proptest]
    fn move_can_be_constructed(
        #[by_ref]
        #[filter(#pos.outcome().is_none())]
        pos: Position,
        #[map(|s: Selector| s.select(#pos.moves(Bitboard::full())))] m: Move,
    ) {
        if m.is_castling() {
            assert_eq!(Move::castling(m.whence(), m.whither()), m);
        } else if m.is_en_passant() {
            assert_eq!(Move::en_passant(m.whence(), m.whither()), m);
        } else {
            let c = pos.role_on(m.whither());
            assert_eq!(Move::regular(m.whence(), m.whither(), m.promotion(), c), m);
        }
    }

    #[proptest]
    fn castling_moves_are_never_captures(wc: Square, wt: Square) {
        assert!(!Move::castling(wc, wt).is_capture());
    }

    #[proptest]
    fn castling_moves_are_never_promotions(wc: Square, wt: Square) {
        assert!(!Move::castling(wc, wt).is_promotion());
    }

    #[proptest]
    fn en_passant_moves_are_always_captures(wc: Square, wt: Square) {
        assert!(Move::en_passant(wc, wt).is_capture());
    }

    #[proptest]
    fn promotions_are_never_quiet(wc: Square, wt: Square, p: Role, c: Option<Role>) {
        assert!(!Move::regular(wc, wt, Some(p), c).is_quiet());
    }

    #[proptest]
    fn captures_are_never_quiet(wc: Square, wt: Square, p: Option<Role>, c: Role) {
        assert!(!Move::regular(wc, wt, p, Some(c)).is_quiet());
    }
}

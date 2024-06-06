use crate::chess::{Role, Square};
use crate::util::{Assume, Binary, Bits, Integer};
use std::{fmt, num::NonZeroU16, ops::RangeBounds};

/// A chess move.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Move(NonZeroU16);

impl Move {
    #[inline(always)]
    fn bits<R: RangeBounds<u32>>(&self, r: R) -> Bits<u16, 16> {
        self.encode().slice(r)
    }

    /// Constructs a castling move.
    #[inline(always)]
    pub fn castling(whence: Square, whither: Square) -> Self {
        let mut bits = Bits::<u16, 16>::default();
        bits.push(whence.encode());
        bits.push(whither.encode());
        bits.push(Bits::<u8, 4>::new(0b0001));
        Move(bits.convert().assume())
    }

    /// Constructs an en passant move.
    #[inline(always)]
    pub fn en_passant(whence: Square, whither: Square) -> Self {
        let mut bits = Bits::<u16, 16>::default();
        bits.push(whence.encode());
        bits.push(whither.encode());
        bits.push(Bits::<u8, 4>::new(0b0110));
        Move(bits.convert().assume())
    }

    /// Constructs a regular move.
    #[inline(always)]
    pub fn regular(whence: Square, whither: Square, promotion: Option<Role>) -> Self {
        let mut bits = Bits::<u16, 16>::default();
        bits.push(whence.encode());
        bits.push(whither.encode());

        match promotion {
            None => bits.push(Bits::<u8, 4>::new(0b0000)),
            Some(r) => {
                bits.push(Bits::<u8, 2>::new(0b10));
                bits.push(Bits::<u8, 2>::new(r.get() - 1));
            }
        }

        Move(bits.convert().assume())
    }

    /// Constructs a capture move.
    #[inline(always)]
    pub fn capture(whence: Square, whither: Square, promotion: Option<Role>) -> Self {
        let mut m = Self::regular(whence, whither, promotion);
        m.0 |= 0b100;
        m
    }

    /// The source [`Square`].
    #[inline(always)]
    pub fn whence(&self) -> Square {
        Square::decode(self.bits(10..).pop())
    }

    /// The destination [`Square`].
    #[inline(always)]
    pub fn whither(&self) -> Square {
        Square::decode(self.bits(4..).pop())
    }

    /// The promotion specifier.
    #[inline(always)]
    pub fn promotion(&self) -> Option<Role> {
        if self.is_promotion() {
            Some(Role::new(self.bits(..2).cast::<u8>() + 1))
        } else {
            None
        }
    }

    /// Whether this is a castling move.
    #[inline(always)]
    pub fn is_castling(&self) -> bool {
        self.bits(..4) == Bits::new(0b0001)
    }

    /// Whether this is an en passant capture move.
    #[inline(always)]
    pub fn is_en_passant(&self) -> bool {
        self.bits(..4) == Bits::new(0b0110)
    }

    /// Whether this is a capture move.
    #[inline(always)]
    pub fn is_capture(&self) -> bool {
        self.bits(2..=2) != Bits::new(0)
    }

    /// Whether this is a promotion move.
    #[inline(always)]
    pub fn is_promotion(&self) -> bool {
        self.bits(3..=3) != Bits::new(0)
    }

    /// Whether this move is neither a capture nor a promotion.
    #[inline(always)]
    pub fn is_quiet(&self) -> bool {
        self.bits(2..=3) == Bits::new(0)
    }
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")?;

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

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.whence(), self.whither())?;

        if let Some(r) = self.promotion() {
            write!(f, "{}", r)?;
        }

        Ok(())
    }
}

impl Binary for Move {
    type Bits = Bits<u16, 16>;

    #[inline(always)]
    fn encode(&self) -> Self::Bits {
        self.0.convert().assume()
    }

    #[inline(always)]
    fn decode(bits: Self::Bits) -> Self {
        Move(bits.convert().assume())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::sample::select;
    use std::mem::size_of;
    use test_strategy::proptest;

    #[test]
    fn move_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<Move>>(), size_of::<Move>());
    }

    #[proptest]
    fn decoding_encoded_move_is_an_identity(m: Move) {
        assert_eq!(Move::decode(m.encode()), m);
    }

    #[proptest]
    fn castling_move_can_be_constructed(wc: Square, #[filter(#wc != #wt)] wt: Square) {
        assert!(Move::castling(wc, wt).is_castling());
    }

    #[proptest]
    fn en_passant_move_can_be_constructed(wc: Square, #[filter(#wc != #wt)] wt: Square) {
        assert!(Move::en_passant(wc, wt).is_en_passant());
    }

    #[proptest]
    fn capture_move_can_be_constructed(
        wc: Square,
        #[filter(#wc != #wt)] wt: Square,
        #[strategy(select(&[Role::Knight, Role::Bishop, Role::Rook, Role::Queen]))] p: Role,
    ) {
        assert!(Move::capture(wc, wt, Some(p)).is_capture());
    }

    #[proptest]
    #[should_panic]
    fn constructing_capture_move_panics_with_invalid_promotion(
        wc: Square,
        #[filter(#wc != #wt)] wt: Square,
        #[strategy(select(&[Role::Pawn, Role::King]))] p: Role,
    ) {
        Move::capture(wc, wt, Some(p));
    }

    #[proptest]
    fn quiet_move_can_be_constructed(wc: Square, #[filter(#wc != #wt)] wt: Square) {
        assert!(Move::regular(wc, wt, None).is_quiet());
    }

    #[proptest]
    fn promotion_move_can_be_constructed(
        wc: Square,
        #[filter(#wc != #wt)] wt: Square,
        #[strategy(select(&[Role::Knight, Role::Bishop, Role::Rook, Role::Queen]))] p: Role,
    ) {
        assert!(Move::regular(wc, wt, Some(p)).is_promotion());
    }

    #[proptest]
    #[should_panic]
    fn constructing_promotion_move_panics_with_invalid_promotion(
        wc: Square,
        #[filter(#wc != #wt)] wt: Square,
        #[strategy(select(&[Role::Pawn, Role::King]))] p: Role,
    ) {
        Move::regular(wc, wt, Some(p));
    }

    #[proptest]
    fn castling_moves_are_never_captures(wc: Square, #[filter(#wc != #wt)] wt: Square) {
        assert!(!Move::castling(wc, wt).is_capture());
    }

    #[proptest]
    fn castling_moves_are_never_promotions(wc: Square, #[filter(#wc != #wt)] wt: Square) {
        assert!(!Move::castling(wc, wt).is_promotion());
    }

    #[proptest]
    fn castling_moves_are_always_quiet(wc: Square, #[filter(#wc != #wt)] wt: Square) {
        assert!(Move::castling(wc, wt).is_quiet());
    }

    #[proptest]
    fn en_passant_moves_are_always_captures(wc: Square, #[filter(#wc != #wt)] wt: Square) {
        assert!(Move::en_passant(wc, wt).is_capture());
    }

    #[proptest]
    fn en_passant_moves_are_never_promotions(wc: Square, #[filter(#wc != #wt)] wt: Square) {
        assert!(!Move::en_passant(wc, wt).is_promotion());
    }

    #[proptest]
    fn en_passant_moves_are_never_quiet(wc: Square, #[filter(#wc != #wt)] wt: Square) {
        assert!(!Move::en_passant(wc, wt).is_quiet());
    }

    #[proptest]
    fn promotions_are_never_quiet(
        wc: Square,
        #[filter(#wc != #wt)] wt: Square,
        #[strategy(select(&[Role::Knight, Role::Bishop, Role::Rook, Role::Queen]))] p: Role,
    ) {
        assert!(!Move::regular(wc, wt, Some(p)).is_quiet());
    }

    #[proptest]
    fn captures_are_never_quiet(
        wc: Square,
        #[filter(#wc != #wt)] wt: Square,
        #[strategy(select(&[Role::Knight, Role::Bishop, Role::Rook, Role::Queen]))] p: Role,
    ) {
        assert!(!Move::capture(wc, wt, None).is_quiet());
        assert!(!Move::capture(wc, wt, Some(p)).is_quiet());
    }
}

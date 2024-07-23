use crate::chess::{Bitboard, Perspective, Piece, Rank, Role, Square, Squares};
use crate::util::{Assume, Binary, Bits, Integer};
use std::fmt::{self, Write};
use std::{num::NonZeroU16, ops::RangeBounds};

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

    /// Updates the source [`Square`].
    #[inline(always)]
    pub fn set_whence(&mut self, whence: Square) {
        let bits = self.0.get() & 0b0000001111111111 | ((whence as u16) << 10);
        self.0 = NonZeroU16::new(bits).assume();
    }

    /// The destination [`Square`].
    #[inline(always)]
    pub fn whither(&self) -> Square {
        Square::decode(self.bits(4..).pop())
    }

    /// This move with a different destination [`Square`].
    #[inline(always)]
    pub fn set_whither(&mut self, whither: Square) {
        let bits = (self.0.get() & 0b1111110000001111) | ((whither as u16) << 4);
        self.0 = NonZeroU16::new(bits).assume();
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

    /// This move with a different promotion specifier.
    #[inline(always)]
    pub fn set_promotion(&mut self, promotion: Role) {
        debug_assert!(self.is_promotion());
        let bits = (self.0.get() & 0b1111111111111100) | (promotion as u16 - 1);
        self.0 = NonZeroU16::new(bits).assume();
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
    #[coverage(off)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)?;

        if self.is_en_passant() {
            f.write_char('^')?;
        } else if self.is_capture() {
            f.write_char('x')?;
        } else if self.is_castling() {
            f.write_char('~')?;
        }

        Ok(())
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.whence(), f)?;
        fmt::Display::fmt(&self.whither(), f)?;

        if let Some(r) = self.promotion() {
            fmt::Display::fmt(&r, f)?;
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

/// A subset of [`Move`]s originating from a given [`Square`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, filter(!#whither.contains(#base.whence())))]
pub struct MoveSet {
    base: Move,
    whither: Bitboard,
}

impl MoveSet {
    /// A set of castling moves.
    #[inline(always)]
    pub fn castling(whence: Square, whither: Bitboard) -> Self {
        let base = Move::castling(whence, whence.flip());
        MoveSet { base, whither }
    }

    /// A set of en passant moves.
    #[inline(always)]
    pub fn en_passant(whence: Square, whither: Bitboard) -> Self {
        let base = Move::en_passant(whence, whence.flip());
        MoveSet { base, whither }
    }

    /// A set of regular moves.
    #[inline(always)]
    pub fn regular(piece: Piece, whence: Square, whither: Bitboard) -> Self {
        use {Piece::*, Rank::*, Role::*};
        let base = match (piece, whence.rank()) {
            (WhitePawn, Seventh) => Move::regular(whence, whence.flip(), Some(Knight)),
            (BlackPawn, Second) => Move::regular(whence, whence.flip(), Some(Knight)),
            _ => Move::regular(whence, whence.flip(), None),
        };

        MoveSet { base, whither }
    }

    /// A set of capture moves.
    #[inline(always)]
    pub fn capture(piece: Piece, whence: Square, whither: Bitboard) -> Self {
        use {Piece::*, Rank::*, Role::*};
        let base = match (piece, whence.rank()) {
            (WhitePawn, Seventh) => Move::capture(whence, whence.flip(), Some(Knight)),
            (BlackPawn, Second) => Move::capture(whence, whence.flip(), Some(Knight)),
            _ => Move::capture(whence, whence.flip(), None),
        };

        MoveSet { base, whither }
    }

    /// The source [`Square`].
    #[inline(always)]
    pub fn whence(&self) -> Square {
        self.base.whence()
    }

    /// The destination [`Square`]s.
    #[inline(always)]
    pub fn whither(&self) -> Bitboard {
        self.whither
    }

    /// Whether the moves in this set are castling moves.
    #[inline(always)]
    pub fn is_castling(&self) -> bool {
        self.base.is_castling()
    }

    /// Whether the moves in this set are en passant captures.
    #[inline(always)]
    pub fn is_en_passant(&self) -> bool {
        self.base.is_en_passant()
    }

    /// Whether the moves in this set are captures.
    #[inline(always)]
    pub fn is_capture(&self) -> bool {
        self.base.is_capture()
    }

    /// Whether the moves in this set are promotions.
    #[inline(always)]
    pub fn is_promotion(&self) -> bool {
        self.base.is_promotion()
    }

    /// Whether the moves in this set are neither captures nor promotions.
    #[inline(always)]
    pub fn is_quiet(&self) -> bool {
        self.base.is_quiet()
    }

    /// An iterator over the [`Move`]s in this bitboard.
    #[inline(always)]
    pub fn iter(&self) -> Moves {
        Moves::new(*self)
    }
}

impl IntoIterator for MoveSet {
    type Item = Move;
    type IntoIter = Moves;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        Moves::new(self)
    }
}

/// An iterator over the [`Move`]s in a [`MoveSet`].
#[derive(Debug)]
pub struct Moves {
    base: Move,
    whither: Squares,
}

impl Moves {
    #[inline(always)]
    fn new(set: MoveSet) -> Self {
        Moves {
            base: set.base,
            whither: set.whither.iter(),
        }
    }
}

impl Iterator for Moves {
    type Item = Move;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(r @ (Role::Queen | Role::Rook | Role::Bishop)) = self.base.promotion() {
            self.base.set_promotion(Role::new(r.get() - 1));
            return Some(self.base);
        }

        self.base.set_whither(self.whither.next()?);
        if self.base.is_promotion() {
            self.base.set_promotion(Role::Queen);
        }

        Some(self.base)
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for Moves {
    #[inline(always)]
    fn len(&self) -> usize {
        match self.base.promotion() {
            None => self.whither.len(),
            Some(r) => 4 * self.whither.len() + (r.get() - Role::Knight.get()) as usize,
        }
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

    #[proptest]
    fn can_set_whence(mut m: Move, #[filter(#wc != #m.whither())] wc: Square) {
        m.set_whence(wc);
        assert_eq!(m.whence(), wc);
    }

    #[proptest]
    fn can_set_whither(mut m: Move, #[filter(#wt != #m.whence())] wt: Square) {
        m.set_whither(wt);
        assert_eq!(m.whither(), wt);
    }

    #[proptest]
    fn can_set_promotion(
        #[filter(#m.is_promotion())] mut m: Move,
        #[strategy(select(&[Role::Knight, Role::Bishop, Role::Rook, Role::Queen]))] p: Role,
    ) {
        m.set_promotion(p);
        assert_eq!(m.promotion(), Some(p));
    }

    #[proptest]
    #[should_panic]
    fn set_promotion_panics_if_not_promotion(#[filter(!#m.is_promotion())] mut m: Move, p: Role) {
        m.set_promotion(p)
    }

    #[proptest]
    fn can_iterate_moves_in_set(ml: MoveSet) {
        let v = Vec::from_iter(ml);
        assert_eq!(ml.iter().len(), v.len());
    }

    #[proptest]
    fn all_moves_in_set_are_of_the_same_type(ml: MoveSet) {
        for m in ml {
            assert_eq!(m.is_castling(), ml.is_castling());
            assert_eq!(m.is_en_passant(), ml.is_en_passant());
            assert_eq!(m.is_promotion(), ml.is_promotion());
            assert_eq!(m.is_capture(), ml.is_capture());
            assert_eq!(m.is_quiet(), ml.is_quiet());
        }
    }

    #[proptest]
    fn all_moves_in_set_are_of_the_same_source_square(ml: MoveSet) {
        for m in ml {
            assert_eq!(m.whence(), ml.whence());
        }
    }
}

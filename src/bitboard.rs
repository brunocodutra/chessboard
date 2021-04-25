use crate::Square;
use derive_more::{
    Binary, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, DebugCustom, Display,
    LowerHex, Not, Octal, Shl, ShlAssign, Shr, ShrAssign, UpperHex,
};
use shakmaty as sm;
use std::iter::{FromIterator, FusedIterator, Map};
use std::ops::Index;

#[cfg(test)]
use proptest::{arbitrary::any, strategy::Strategy};

/// A set of [`Square`]s represented by a bit array.
///
/// Bits are ordered by [`Square::index`].
#[derive(
    DebugCustom,
    Display,
    UpperHex,
    LowerHex,
    Octal,
    Binary,
    Default,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Not,
    BitAnd,
    BitAndAssign,
    BitOr,
    BitOrAssign,
    BitXor,
    BitXorAssign,
    Shr,
    ShrAssign,
    Shl,
    ShlAssign,
)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug(fmt = "Bitboard(\"{}\")", self)]
#[display(fmt = "{:#010o}", self)]
pub struct Bitboard(#[cfg_attr(test, strategy(any::<u64>().prop_map_into()))] sm::Bitboard);

impl Bitboard {
    /// Constructs an empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of [`Square`]s in the set.
    pub fn len(&self) -> usize {
        self.0.count()
    }

    /// Whether this [`Square`] is present.
    pub fn contains(&self, s: Square) -> bool {
        self.0.contains(s.into())
    }

    /// Add this [`Square`] to the set.
    ///
    /// Returns `false` if it was already present.
    pub fn insert(&mut self, s: Square) -> bool {
        let bb = *self;
        self.0.set(s.into(), true);
        *self != bb
    }

    /// Remove this [`Square`] from the set.
    ///
    /// Returns `false` if it was already absent.
    pub fn remove(&mut self, s: Square) -> bool {
        self.0.remove(s.into())
    }
}

impl Index<Square> for Bitboard {
    type Output = bool;

    fn index(&self, s: Square) -> &Self::Output {
        match self.contains(s) {
            true => &true,
            false => &false,
        }
    }
}

/// Constructs a unit set.
impl From<Square> for Bitboard {
    fn from(s: Square) -> Self {
        Bitboard(sm::Bitboard::from_square(s.into()))
    }
}

#[doc(hidden)]
impl From<sm::Bitboard> for Bitboard {
    fn from(bb: sm::Bitboard) -> Self {
        Bitboard(bb)
    }
}

#[doc(hidden)]
impl From<Bitboard> for sm::Bitboard {
    fn from(bb: Bitboard) -> Self {
        bb.0
    }
}

impl FromIterator<Square> for Bitboard {
    fn from_iter<T: IntoIterator<Item = Square>>(iter: T) -> Self {
        iter.into_iter()
            .map(Bitboard::from)
            .fold(Bitboard::new(), |a, b| a | b)
    }
}

impl Extend<Square> for Bitboard {
    fn extend<T: IntoIterator<Item = Square>>(&mut self, iter: T) {
        *self |= iter.into_iter().collect()
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = BitboardIterator;

    fn into_iter(self) -> Self::IntoIter {
        BitboardIterator(self.0.into_iter().map(Square::from))
    }
}

/// Iterator over the squares of a [`Bitboard`].
#[derive(Debug, Clone)]
pub struct BitboardIterator(Map<sm::bitboard::IntoIter, fn(sm::Square) -> Square>);

impl Iterator for BitboardIterator {
    type Item = Square;

    fn next(&mut self) -> Option<Square> {
        self.0.next()
    }

    fn count(self) -> usize {
        self.0.count()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    fn last(self) -> Option<Square> {
        self.0.last()
    }
}

impl ExactSizeIterator for BitboardIterator {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl DoubleEndedIterator for BitboardIterator {
    fn next_back(&mut self) -> Option<Square> {
        self.0.next_back()
    }
}

impl FusedIterator for BitboardIterator {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BinaryHeap;
    use test_strategy::proptest;

    #[proptest]
    fn len_returns_number_of_bits_set(bb: Bitboard) {
        assert_eq!(bb.len(), u64::from(bb.0).count_ones() as usize);
    }

    #[proptest]
    fn contains_checks_whether_square_is_in_bitboard(bb: Bitboard, s: Square) {
        assert_eq!(bb.contains(s), (bb & Bitboard::from(s)) != Bitboard::new());
    }

    #[proptest]
    fn insert_sets_bit_at_square_index_to_true(mut bb: Bitboard, s: Square) {
        let cc = bb | Bitboard::from(s);
        assert_eq!(!bb.contains(s), bb.insert(s));
        assert_eq!(bb, cc);
    }

    #[proptest]
    fn remove_sets_bit_at_square_index_to_false(mut bb: Bitboard, s: Square) {
        let cc = bb & !Bitboard::from(s);
        assert_eq!(bb.contains(s), bb.remove(s));
        assert_eq!(bb, cc);
    }

    #[proptest]
    fn bitboard_implements_index_operator(bb: Bitboard, s: Square) {
        assert_eq!(bb[s], bb.contains(s));
    }

    #[proptest]
    fn bitboard_can_be_turned_into_an_ordered_iterator_over_squares(bb: Bitboard) {
        assert_eq!(
            bb.into_iter().collect::<Vec<_>>(),
            bb.into_iter().collect::<BinaryHeap<_>>().into_sorted_vec()
        );
    }

    #[proptest]
    fn bitboard_can_be_turned_into_reversible_iterator(bb: Bitboard) {
        let mut ss = bb.into_iter().collect::<Vec<_>>();
        ss.reverse();
        assert_eq!(bb.into_iter().rev().collect::<Vec<_>>(), ss);
    }

    #[proptest]
    fn bitboard_can_be_turned_into_iterator_of_exact_size(bb: Bitboard) {
        assert_eq!(bb.into_iter().len(), bb.len());
    }

    #[proptest]
    fn bitboard_can_be_extended_from_iterator_over_squares(mut a: Bitboard, b: Bitboard) {
        let c = a | b;
        a.extend(b);
        assert_eq!(a, c);
    }

    #[proptest]
    fn iterator_over_squares_can_be_collected_to_bitboard(bb: Bitboard) {
        assert_eq!(bb.into_iter().collect::<Bitboard>(), bb);
    }

    #[proptest]
    fn bitboard_displays_as_prefixed_fixed_width_octal(bb: Bitboard) {
        assert_eq!(bb.to_string(), format!("{:#010o}", bb));
    }

    #[proptest]
    fn bitboard_can_be_constructed_from_square(s: Square) {
        assert_eq!(Bitboard::from(s), sm::Bitboard::from(1 << s.index()).into());
    }

    #[proptest]
    fn bitboard_has_an_equivalent_shakmaty_representation(bb: Bitboard) {
        assert_eq!(Bitboard::from(sm::Bitboard::from(bb)), bb);
    }
}

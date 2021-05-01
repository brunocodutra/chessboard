use crate::Square;
use derive_more::*;
use shakmaty as sm;
use std::fmt::{Binary, Error as FmtError, Formatter, LowerHex, Octal, UpperHex};
use std::iter::{FromIterator, FusedIterator, Map};
use std::ops::Index;

#[cfg(test)]
use proptest::{arbitrary::any, strategy::Strategy};

/// A set of [`Square`]s represented by a bit array.
///
/// Each bit corresponds to the [`Square`] with the same [index][`Square::index`] .
#[derive(
    DebugCustom,
    Display,
    // UpperHex,
    // LowerHex,
    // Octal,
    // Binary,
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
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[debug(fmt = "Bitboard(\"{}\")", self)]
#[display(fmt = "{:#010o}", self)]
pub struct Bitboard(
    #[cfg_attr(test, proptest(strategy = "any::<u64>().prop_map_into()"))] sm::Bitboard,
);

impl Bitboard {
    /// Reads the bit corresponding to the [`Square`].
    pub fn get(&self, s: Square) -> bool {
        self.0.contains(s.into())
    }

    /// Writes the value to the bit corresponding to the [`Square`].
    pub fn set(&mut self, s: Square, v: bool) {
        self.0.set(s.into(), v)
    }

    /// Flips the bit corresponding to the [`Square`].
    pub fn toggle(&mut self, s: Square) {
        self.0.toggle::<sm::Square>(s.into())
    }
}

/// Syntax sugar for [`Bitboard::get`].
impl Index<Square> for Bitboard {
    type Output = bool;

    fn index(&self, s: Square) -> &Self::Output {
        match self.get(s) {
            true => &true,
            false => &false,
        }
    }
}

// FIXME: https://github.com/niklasf/shakmaty/pull/42
impl UpperHex for Bitboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        UpperHex::fmt(&u64::from(self.0), f)
    }
}

// FIXME: https://github.com/niklasf/shakmaty/pull/42
impl LowerHex for Bitboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        LowerHex::fmt(&u64::from(self.0), f)
    }
}

// FIXME: https://github.com/niklasf/shakmaty/pull/42
impl Octal for Bitboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        Octal::fmt(&u64::from(self.0), f)
    }
}

// FIXME: https://github.com/niklasf/shakmaty/pull/42
impl Binary for Bitboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        Binary::fmt(&u64::from(self.0), f)
    }
}

impl From<u64> for Bitboard {
    fn from(bb: u64) -> Self {
        Bitboard(bb.into())
    }
}

impl From<Bitboard> for u64 {
    fn from(bb: Bitboard) -> Self {
        bb.0.into()
    }
}

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
            .fold(Bitboard::default(), |a, b| a | b)
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
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn bitboard_can_be_constructed_from_u64(n: u64) {
            assert_eq!(u64::from(Bitboard::from(n)), n);
        }

        #[test]
        fn bitboard_can_be_constructed_from_square(s: Square) {
            assert_eq!(u64::from(Bitboard::from(s)).trailing_zeros(), s.index() as u32);
        }

        #[test]
        fn get_returns_bit_corresponding_to_square(bb: Bitboard, s: Square) {
            assert_eq!(bb.get(s), bb & Bitboard::from(s) == Bitboard::from(s));
        }

        #[test]
        fn set_overwrites_bit_corresponding_to_square(mut bb: Bitboard, s: Square, v: bool) {
            bb.set(s, v);
            assert_eq!(bb.get(s), v);
        }

        #[test]
        fn toggle_flips_bit_corresponding_to_square(mut bb: Bitboard, s: Square) {
            let cc = bb;
            bb.toggle(s);
            assert_eq!(bb.get(s), !cc.get(s));
        }

        #[test]
        fn bitboard_implements_index_operator(bb: Bitboard, s: Square) {
            assert_eq!(bb[s], bb.get(s));
        }

        #[test]
        fn bitboard_can_be_turned_into_iterator_over_squares(bb: Bitboard) {
            assert_eq!(bb.into_iter().map(Bitboard::from).fold(Bitboard::default(), |a, b| a | b), bb);
        }

        #[test]
        fn bitboard_can_be_extended_from_iterator_over_squares(mut a: Bitboard, b: Bitboard) {
            let c = a | b;
            a.extend(b);
            assert_eq!(a, c);
        }

        #[test]
        fn iterator_over_squares_can_be_collected_to_bitboard(bb: Bitboard) {
            assert_eq!(bb.into_iter().collect::<Bitboard>(), bb);
        }

        #[test]
        fn bitboard_displays_as_prefixed_fixed_width_octal(bb: Bitboard) {
            assert_eq!(bb.to_string(), format!("{:#010o}", u64::from(bb)));
        }

        #[test]
        fn bitboard_has_an_equivalent_shakmaty_representation(bb: Bitboard) {
            assert_eq!(Bitboard::from(sm::Bitboard::from(bb)), bb);
        }

    }
}

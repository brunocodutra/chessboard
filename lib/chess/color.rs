use crate::util::Enum;
use cozy_chess as cc;
use derive_more::Display;
use std::ops::{Not, RangeInclusive};

/// The color of a chess [`Piece`][`crate::Piece`].
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(u8)]
pub enum Color {
    #[display("white")]
    White,
    #[display("black")]
    Black,
}

unsafe impl Enum for Color {
    const RANGE: RangeInclusive<Self> = Color::White..=Color::Black;

    #[inline(always)]
    fn repr(&self) -> u8 {
        *self as _
    }
}

impl Not for Color {
    type Output = Self;

    #[inline(always)]
    fn not(self) -> Self {
        self.mirror()
    }
}

#[doc(hidden)]
impl From<cc::Color> for Color {
    #[inline(always)]
    fn from(c: cc::Color) -> Self {
        match c {
            cc::Color::White => Color::White,
            cc::Color::Black => Color::Black,
        }
    }
}

#[doc(hidden)]
impl From<Color> for cc::Color {
    #[inline(always)]
    fn from(c: Color) -> Self {
        match c {
            Color::White => cc::Color::White,
            Color::Black => cc::Color::Black,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;
    use test_strategy::proptest;

    #[proptest]
    fn color_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<Color>>(), size_of::<Color>());
    }

    #[proptest]
    fn color_implements_not_operator(c: Color) {
        assert_eq!(!c, c.mirror());
    }

    #[proptest]
    fn color_has_an_equivalent_cozy_chess_representation(c: Color) {
        assert_eq!(Color::from(cc::Color::from(c)), c);
    }
}

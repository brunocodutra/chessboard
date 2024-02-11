use crate::{chess::Mirror, util::Integer};
use derive_more::Display;
use std::ops::Not;

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

unsafe impl const Integer for Color {
    type Repr = u8;
    const MIN: Self::Repr = Color::White as _;
    const MAX: Self::Repr = Color::Black as _;
}

impl const Mirror for Color {
    #[inline(always)]
    fn mirror(&self) -> Self {
        match *self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

impl Not for Color {
    type Output = Self;

    #[inline(always)]
    fn not(self) -> Self {
        self.mirror()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;
    use test_strategy::proptest;

    #[test]
    fn color_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<Color>>(), size_of::<Color>());
    }

    #[proptest]
    fn color_implements_not_operator(c: Color) {
        assert_eq!(!c, c.mirror());
    }
}

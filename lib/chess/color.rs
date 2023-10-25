use cozy_chess as cc;
use derive_more::Display;
use std::ops::Not;

/// The color of a chess [`Piece`][`crate::Piece`].
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(u8)]
pub enum Color {
    #[display(fmt = "white")]
    White,
    #[display(fmt = "black")]
    Black,
}

impl Not for Color {
    type Output = Self;

    fn not(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[doc(hidden)]
impl From<cc::Color> for Color {
    fn from(c: cc::Color) -> Self {
        match c {
            cc::Color::White => Color::White,
            cc::Color::Black => Color::Black,
        }
    }
}

#[doc(hidden)]
impl From<Color> for cc::Color {
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
    use test_strategy::proptest;

    #[proptest]
    fn color_implements_not_operator(c: Color) {
        assert_eq!(!!c, c);
    }

    #[proptest]
    fn color_has_an_equivalent_cozy_chess_representation(c: Color) {
        assert_eq!(Color::from(cc::Color::from(c)), c);
    }
}

use derive_more::Display;
use shakmaty as sm;
use std::ops::Not;
use test_strategy::Arbitrary;

/// Denotes the color of a chess [`Piece`][`super::Piece`].
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub enum Color {
    #[display(fmt = "white")]
    White,
    #[display(fmt = "black")]
    Black,
}

impl Not for Color {
    type Output = Color;

    #[inline]
    fn not(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[doc(hidden)]
impl From<sm::Color> for Color {
    #[inline]
    fn from(c: sm::Color) -> Self {
        match c {
            sm::Color::White => Color::White,
            sm::Color::Black => Color::Black,
        }
    }
}

#[doc(hidden)]
impl From<Color> for sm::Color {
    #[inline]
    fn from(c: Color) -> Self {
        match c {
            Color::White => sm::Color::White,
            Color::Black => sm::Color::Black,
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
    fn color_has_an_equivalent_shakmaty_representation(c: Color) {
        assert_eq!(Color::from(sm::Color::from(c)), c);
    }
}

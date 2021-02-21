use crate::foreign;
use derive_more::Display;

/// The color of a chess piece.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Color {
    #[display(fmt = "white")]
    White,
    #[display(fmt = "black")]
    Black,
}

impl From<Color> for &'static str {
    fn from(c: Color) -> Self {
        match c {
            Color::White => "white",
            Color::Black => "black",
        }
    }
}

impl From<foreign::Color> for Color {
    fn from(c: foreign::Color) -> Self {
        match c {
            foreign::Color::White => Color::White,
            foreign::Color::Black => Color::Black,
        }
    }
}

impl Into<foreign::Color> for Color {
    fn into(self) -> foreign::Color {
        match self {
            Color::White => foreign::Color::White,
            Color::Black => foreign::Color::Black,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn every_color_has_an_associated_static_str(c: Color) {
            assert_eq!(<&str>::from(c), c.to_string());
        }
    }
}

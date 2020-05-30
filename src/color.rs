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

impl From<foreign::Color> for Color {
    fn from(c: foreign::Color) -> Self {
        use Color::*;
        match c {
            foreign::Color::White => White,
            foreign::Color::Black => Black,
        }
    }
}

impl Into<foreign::Color> for Color {
    fn into(self) -> foreign::Color {
        use Color::*;
        match self {
            White => foreign::Color::White,
            Black => foreign::Color::Black,
        }
    }
}

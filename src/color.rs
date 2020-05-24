use crate::foreign;

/// The color of a chess piece.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn to_str(self) -> &'static str {
        use Color::*;
        match self {
            White => &"white",
            Black => &"black",
        }
    }
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

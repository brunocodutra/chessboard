use cozy_chess as cc;
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

impl Color {
    pub const ALL: [Self; 2] = [Color::White, Color::Black];

    /// Constructs [`Color`] from index.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not in the range (0..=1).
    pub fn from_index(i: u8) -> Self {
        Self::ALL[i as usize]
    }

    /// This colors's index in the range (0..=1).
    pub fn index(&self) -> u8 {
        *self as _
    }

    /// Mirrors this color.
    pub fn mirror(&self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

impl Not for Color {
    type Output = Self;

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
    use crate::util::Buffer;
    use std::mem::size_of;
    use test_strategy::proptest;

    #[proptest]
    fn color_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<Color>>(), size_of::<Color>());
    }

    #[proptest]
    fn color_has_an_index(c: Color) {
        assert_eq!(Color::from_index(c.index()), c);
    }

    #[proptest]

    fn from_index_constructs_color_by_index(#[strategy(0u8..2)] i: u8) {
        assert_eq!(Color::from_index(i).index(), i);
    }

    #[proptest]
    #[should_panic]

    fn from_index_panics_if_index_out_of_range(#[strategy(2u8..)] i: u8) {
        Color::from_index(i);
    }

    #[proptest]
    fn color_is_ordered_by_index(a: Color, b: Color) {
        assert_eq!(a < b, a.index() < b.index());
    }

    #[proptest]
    fn all_contains_colors_in_order() {
        assert_eq!(
            Color::ALL.into_iter().collect::<Buffer<_, 2>>(),
            (0..=1).map(Color::from_index).collect()
        );
    }

    #[proptest]
    fn color_has_a_mirror(c: Color) {
        assert_eq!(c.mirror().index(), 1 - c.index());
    }

    #[proptest]
    fn color_implements_not_operator(c: Color) {
        assert_eq!(!!c, c);
    }

    #[proptest]
    fn color_has_an_equivalent_cozy_chess_representation(c: Color) {
        assert_eq!(Color::from(cc::Color::from(c)), c);
    }
}

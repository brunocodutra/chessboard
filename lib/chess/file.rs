use crate::chess::{Bitboard, Mirror};
use crate::util::Integer;
use derive_more::{Display, Error};
use std::{ops::Sub, str::FromStr};

/// A column on the chess board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(i8)]
pub enum File {
    #[display("a")]
    A,
    #[display("b")]
    B,
    #[display("c")]
    C,
    #[display("d")]
    D,
    #[display("e")]
    E,
    #[display("f")]
    F,
    #[display("g")]
    G,
    #[display("h")]
    H,
}

impl File {
    /// Returns a [`Bitboard`] that only contains this file.
    #[inline(always)]
    pub fn bitboard(self) -> Bitboard {
        Bitboard::new(0x0101010101010101 << self.get())
    }
}

unsafe impl Integer for File {
    type Repr = i8;
    const MIN: Self::Repr = File::A as _;
    const MAX: Self::Repr = File::H as _;
}

impl Mirror for File {
    #[inline(always)]
    fn mirror(&self) -> Self {
        Self::new(self.get() ^ Self::MAX)
    }
}

impl Sub for File {
    type Output = i8;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        self.get() - rhs.get()
    }
}

/// The reason why parsing [`File`] failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(
    "failed to parse file, expected letter in the range `({}..={})`",
    File::A,
    File::H
)]
pub struct ParseFileError;

impl FromStr for File {
    type Err = ParseFileError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "a" => Ok(File::A),
            "b" => Ok(File::B),
            "c" => Ok(File::C),
            "d" => Ok(File::D),
            "e" => Ok(File::E),
            "f" => Ok(File::F),
            "g" => Ok(File::G),
            "h" => Ok(File::H),
            _ => Err(ParseFileError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::{Rank, Square};
    use std::mem::size_of;
    use test_strategy::proptest;

    #[test]
    fn file_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<File>>(), size_of::<File>());
    }

    #[proptest]
    fn mirroring_file_returns_complement(f: File) {
        assert_eq!(f.mirror().get(), File::MAX - f.get());
    }

    #[proptest]
    fn subtracting_files_returns_distance(a: File, b: File) {
        assert_eq!(a - b, a.get() - b.get());
    }

    #[proptest]
    fn file_has_an_equivalent_bitboard(f: File) {
        assert_eq!(
            Vec::from_iter(f.bitboard()),
            Vec::from_iter(Rank::iter().map(|r| Square::new(f, r)))
        );
    }

    #[proptest]
    fn parsing_printed_file_is_an_identity(f: File) {
        assert_eq!(f.to_string().parse(), Ok(f));
    }

    #[proptest]
    fn parsing_file_fails_if_not_lower_case_letter_between_a_and_h(
        #[filter(!('a'..='h').contains(&#c))] c: char,
    ) {
        assert_eq!(c.to_string().parse::<File>(), Err(ParseFileError));
    }

    #[proptest]
    fn parsing_file_fails_if_length_not_one(#[filter(#s.len() != 1)] s: String) {
        assert_eq!(s.parse::<File>(), Err(ParseFileError));
    }
}

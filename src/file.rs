use derive_more::{Display, Error, From};
use shakmaty as sm;
use std::convert::{TryFrom, TryInto};
use std::{char::ParseCharError, str::FromStr};
use tracing::instrument;

/// Denotes a column on the chess board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[repr(u8)]
pub enum File {
    #[display(fmt = "a")]
    A = b'a',
    #[display(fmt = "b")]
    B,
    #[display(fmt = "c")]
    C,
    #[display(fmt = "d")]
    D,
    #[display(fmt = "e")]
    E,
    #[display(fmt = "f")]
    F,
    #[display(fmt = "g")]
    G,
    #[display(fmt = "h")]
    H,
}

impl File {
    pub const VARIANTS: &'static [File] = &[
        File::A,
        File::B,
        File::C,
        File::D,
        File::E,
        File::F,
        File::G,
        File::H,
    ];

    /// This files's index in the range (0..=7).
    pub fn index(&self) -> usize {
        (*self).into()
    }
}

/// The reason why parsing [`File`] failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error, From)]
#[display(fmt = "unable to parse file")]
pub enum ParseFileError {
    ParseCharError(ParseCharError),
    OutOfRange(FileOutOfRange),
}

impl FromStr for File {
    type Err = ParseFileError;

    #[instrument(level = "trace", err)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse::<char>()?.try_into()?)
    }
}

/// The reason why converting [`File`] from index failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(
    fmt = "expected lower case letter in the range `({}..={})`",
    File::A,
    File::H
)]
pub struct FileOutOfRange;

impl TryFrom<char> for File {
    type Error = FileOutOfRange;

    #[instrument(level = "trace", err)]
    fn try_from(c: char) -> Result<Self, Self::Error> {
        match c {
            'a' => Ok(File::A),
            'b' => Ok(File::B),
            'c' => Ok(File::C),
            'd' => Ok(File::D),
            'e' => Ok(File::E),
            'f' => Ok(File::F),
            'g' => Ok(File::G),
            'h' => Ok(File::H),
            _ => Err(FileOutOfRange),
        }
    }
}

impl From<File> for char {
    fn from(f: File) -> char {
        (f as u8).into()
    }
}

/// The reason why converting [`File`] from index failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(fmt = "expected integer in the range `(0..=7)`")]
pub struct FileIndexOutOfRange;

impl TryFrom<usize> for File {
    type Error = FileIndexOutOfRange;

    #[instrument(level = "trace", err)]
    fn try_from(i: usize) -> Result<Self, Self::Error> {
        Self::VARIANTS.get(i).copied().ok_or(FileIndexOutOfRange)
    }
}

impl From<File> for usize {
    fn from(f: File) -> usize {
        f as usize - File::A as usize
    }
}

#[doc(hidden)]
impl From<sm::File> for File {
    fn from(f: sm::File) -> Self {
        usize::from(f).try_into().unwrap()
    }
}

#[doc(hidden)]
impl From<File> for sm::File {
    fn from(f: File) -> Self {
        sm::File::new(f.index() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn parsing_printed_file_is_an_identity(f: File) {
            assert_eq!(f.to_string().parse(), Ok(f));
        }

        #[test]
        fn parsing_file_succeeds_for_lower_case_letter_between_a_and_h(c in b'a'..=b'h') {
            let c = char::from(c);
            assert_eq!(c.to_string().parse::<File>(), Ok(c.try_into().unwrap()));
        }

        #[test]
        fn parsing_file_fails_for_upper_case_letter(s in "[A-Z]") {
            assert_eq!(s.parse::<File>(), Err(ParseFileError::OutOfRange(FileOutOfRange)));
        }

        #[test]
        fn parsing_file_fails_for_strings_of_length_not_one(s in ".{2,}?") {
            use ParseFileError::*;
            assert_eq!(s.parse::<File>(), Err(ParseCharError(s.parse::<char>().unwrap_err())));
        }

        #[test]
        fn parsing_file_fails_for_char_other_than_lower_case_letter_between_a_and_h(c: char) {
            prop_assume!(!('a'..='h').contains(&c));
            use ParseFileError::*;
            assert_eq!(c.to_string().parse::<File>(), Err(OutOfRange(File::try_from(c).unwrap_err())));
        }

        #[test]
        fn file_can_be_converted_into_char(f: File) {
            assert_eq!(char::from(f).try_into(), Ok(f));
        }

        #[test]
        fn converting_file_from_char_out_of_range_fails(c in b'i'..) {
            assert_eq!(File::try_from(char::from(c)), Err(FileOutOfRange));
        }

        #[test]
        fn file_has_an_index(f: File) {
            assert_eq!(f.index().try_into(), Ok(f));
        }

        #[test]
        fn converting_file_from_index_out_of_range_fails(i in 8usize..) {
            assert_eq!(File::try_from(i), Err(FileIndexOutOfRange));
        }

        #[test]
        fn file_is_ordered_by_index(a: File, b: File) {
            assert_eq!(a < b, a.index() < b.index());
        }

        #[test]
        fn file_has_an_equivalent_shakmaty_representation(f: File) {
            assert_eq!(File::from(sm::File::from(f)), f);
        }
    }
}

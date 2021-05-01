use derive_more::{Display, Error, From};
use shakmaty as sm;
use std::convert::{TryFrom, TryInto};
use std::{char::ParseCharError, iter::FusedIterator, str::FromStr};
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
    /// Constructs [`File`] from index.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not in the range (0..=7).
    pub fn new(i: usize) -> Self {
        i.try_into().unwrap()
    }

    /// This files's index in the range (0..=7).
    pub fn index(&self) -> usize {
        (*self).into()
    }

    /// Returns an iterator over [`File`]s ordered by [index][`File::index`].
    pub fn iter() -> impl DoubleEndedIterator<Item = Self> + ExactSizeIterator + FusedIterator {
        (0usize..8).map(File::new)
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
        Self::iter()
            .find(|&f| char::from(f) == c)
            .ok_or(FileOutOfRange)
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
        use File::*;

        [A, B, C, D, E, F, G, H]
            .get(i)
            .copied()
            .ok_or(FileIndexOutOfRange)
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
        File::new(usize::from(f))
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
        fn iter_returns_iterator_over_files_in_order(_: ()) {
            use File::*;
            assert_eq!(
                File::iter().collect::<Vec<_>>(),
                vec![A, B, C, D, E, F, G, H]
            );
        }

        #[test]
        fn iter_returns_double_ended_iterator(_: ()) {
            use File::*;
            assert_eq!(
                File::iter().rev().collect::<Vec<_>>(),
                vec![H, G, F, E, D, C, B, A]
            );
        }

        #[test]
        fn iter_returns_iterator_of_exact_size(_: ()) {
            assert_eq!(File::iter().len(), 8);
        }

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
        fn new_constructs_file_by_index(i in (0usize..=7)) {
            assert_eq!(File::new(i).index(), i);
        }

        #[test]
        #[should_panic]
        fn new_panics_if_index_out_of_range(i in (8usize..)) {
            File::new(i);
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

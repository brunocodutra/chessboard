use derive_more::{Display, Error, From};
use shakmaty as sm;
use std::convert::{TryFrom, TryInto};
use std::{char::ParseCharError, iter::FusedIterator, str::FromStr};

#[cfg(test)]
use test_strategy::Arbitrary;

/// Denotes a column on the chess board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(Arbitrary))]
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
#[display(fmt = "failed to parse file")]
pub enum ParseFileError {
    ParseCharError(ParseCharError),
    OutOfRange(FileOutOfRange),
}

impl FromStr for File {
    type Err = ParseFileError;

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
    use test_strategy::proptest;

    #[proptest]
    fn iter_returns_iterator_over_files_in_order() {
        use File::*;
        assert_eq!(
            File::iter().collect::<Vec<_>>(),
            vec![A, B, C, D, E, F, G, H]
        );
    }

    #[proptest]
    fn iter_returns_double_ended_iterator() {
        use File::*;
        assert_eq!(
            File::iter().rev().collect::<Vec<_>>(),
            vec![H, G, F, E, D, C, B, A]
        );
    }

    #[proptest]
    fn iter_returns_iterator_of_exact_size() {
        assert_eq!(File::iter().len(), 8);
    }

    #[proptest]
    fn parsing_printed_file_is_an_identity(f: File) {
        assert_eq!(f.to_string().parse(), Ok(f));
    }

    #[proptest]
    fn parsing_file_succeeds_for_lower_case_letter_between_a_and_h(#[strategy(b'a'..=b'h')] c: u8) {
        let c = char::from(c);
        assert_eq!(c.to_string().parse::<File>(), Ok(c.try_into()?));
    }

    #[proptest]
    fn parsing_file_fails_for_upper_case_letter(#[strategy("[A-Z]")] s: String) {
        assert_eq!(
            s.parse::<File>(),
            Err(ParseFileError::OutOfRange(FileOutOfRange))
        );
    }

    #[proptest]
    fn parsing_file_fails_for_strings_of_length_not_one(#[strategy(".{2,}?")] s: String) {
        use ParseFileError::*;
        assert_eq!(
            s.parse::<File>(),
            Err(ParseCharError(s.parse::<char>().unwrap_err()))
        );
    }

    #[proptest]
    fn parsing_file_fails_for_char_other_than_lower_case_letter_between_a_and_h(
        #[filter(!('a'..='h').contains(&#c))] c: char,
    ) {
        use ParseFileError::*;
        assert_eq!(
            c.to_string().parse::<File>(),
            Err(OutOfRange(File::try_from(c).unwrap_err()))
        );
    }

    #[proptest]
    fn file_can_be_converted_into_char(f: File) {
        assert_eq!(char::from(f).try_into(), Ok(f));
    }

    #[proptest]
    fn converting_file_from_char_out_of_range_fails(#[strategy(b'i'..)] c: u8) {
        assert_eq!(File::try_from(char::from(c)), Err(FileOutOfRange));
    }

    #[proptest]
    fn file_has_an_index(f: File) {
        assert_eq!(f.index().try_into(), Ok(f));
    }

    #[proptest]
    fn new_constructs_file_by_index(#[strategy(0usize..=7)] i: usize) {
        assert_eq!(File::new(i).index(), i);
    }

    #[proptest]
    #[should_panic]
    fn new_panics_if_index_out_of_range(#[strategy(8usize..)] i: usize) {
        File::new(i);
    }

    #[proptest]
    fn converting_file_from_index_out_of_range_fails(#[strategy(8usize..)] i: usize) {
        assert_eq!(File::try_from(i), Err(FileIndexOutOfRange));
    }

    #[proptest]
    fn file_is_ordered_by_index(a: File, b: File) {
        assert_eq!(a < b, a.index() < b.index());
    }

    #[proptest]
    fn file_has_an_equivalent_shakmaty_representation(f: File) {
        assert_eq!(File::from(sm::File::from(f)), f);
    }
}

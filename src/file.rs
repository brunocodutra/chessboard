use derive_more::{DebugCustom, Display, Error, From};
use shakmaty as sm;
use std::convert::{TryFrom, TryInto};
use std::{char::ParseCharError, num::TryFromIntError, ops::Sub, str::FromStr};

#[cfg(test)]
use proptest::sample::select;

/// Denotes a column on the chess board.
#[derive(DebugCustom, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug(fmt = "{}", self)]
#[display(fmt = "{}", _0)]
pub struct File(#[cfg_attr(test, strategy(select(sm::File::ALL.as_ref())))] sm::File);

impl File {
    /// Constructs [`File`] from index.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not in the range (0..=7).
    pub fn from_index(i: u8) -> Self {
        i.try_into().unwrap()
    }

    /// This files's index in the range (0..=7).
    pub fn index(&self) -> u8 {
        (*self).into()
    }

    /// Returns an iterator over [`File`]s ordered by [index][`File::index`].
    pub fn iter() -> impl DoubleEndedIterator<Item = Self> + ExactSizeIterator {
        sm::File::ALL.into_iter().map(File)
    }
}

impl Sub for File {
    type Output = i8;

    fn sub(self, rhs: Self) -> Self::Output {
        self.index() as i8 - rhs.index() as i8
    }
}

/// The reason why parsing [`File`] failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error, From)]
#[display(fmt = "failed to parse file")]
pub enum ParseFileError {
    ParseCharError(ParseCharError),
    InvalidFile(InvalidFile),
}

impl FromStr for File {
    type Err = ParseFileError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse::<char>()?.try_into()?)
    }
}

/// The reason why converting [`File`] from index failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(fmt = "expected lower case letter in the range `('a'..='h')`")]
pub struct InvalidFile;

impl TryFrom<char> for File {
    type Error = InvalidFile;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        sm::File::from_char(c).map(File).ok_or(InvalidFile)
    }
}

impl From<File> for char {
    fn from(f: File) -> char {
        f.0.char()
    }
}

/// The reason why converting [`File`] from index failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(fmt = "expected integer in the range `(0..=7)`")]
pub struct FileOutOfRange;

impl From<TryFromIntError> for FileOutOfRange {
    fn from(_: TryFromIntError) -> Self {
        FileOutOfRange
    }
}

impl TryFrom<u8> for File {
    type Error = FileOutOfRange;

    fn try_from(i: u8) -> Result<Self, Self::Error> {
        Ok(File(i.try_into()?))
    }
}

impl From<File> for u8 {
    fn from(f: File) -> u8 {
        f.0.into()
    }
}

#[doc(hidden)]
impl From<sm::File> for File {
    fn from(f: sm::File) -> Self {
        File(f)
    }
}

#[doc(hidden)]
impl From<File> for sm::File {
    fn from(f: File) -> Self {
        f.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;
    use test_strategy::proptest;

    #[proptest]
    fn file_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<File>>(), size_of::<File>());
    }

    #[proptest]
    fn iter_returns_iterator_over_files_in_order() {
        assert_eq!(
            File::iter().collect::<Vec<_>>(),
            (0..=7).map(File::from_index).collect::<Vec<_>>()
        );
    }

    #[proptest]
    fn iter_returns_double_ended_iterator() {
        assert_eq!(
            File::iter().rev().collect::<Vec<_>>(),
            (0..=7).rev().map(File::from_index).collect::<Vec<_>>()
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
            Err(ParseFileError::InvalidFile(InvalidFile))
        );
    }

    #[proptest]
    fn parsing_file_fails_for_strings_of_length_not_one(#[strategy(".{2,}?")] s: String) {
        assert_eq!(
            s.parse::<File>().err(),
            s.parse::<char>().err().map(Into::into)
        );
    }

    #[proptest]
    fn parsing_file_fails_for_letters_out_of_range(#[filter(!('a'..='h').contains(&#c))] c: char) {
        assert_eq!(
            c.to_string().parse::<File>().err(),
            File::try_from(c).err().map(Into::into)
        );
    }

    #[proptest]
    fn file_can_be_converted_to_char(f: File) {
        assert_eq!(char::from(f).try_into(), Ok(f));
    }

    #[proptest]
    fn converting_file_from_letter_out_of_range_fails(
        #[filter(!('a'..='h').contains(&#c))] c: char,
    ) {
        assert_eq!(File::try_from(c), Err(InvalidFile));
    }

    #[proptest]
    fn file_has_an_index(f: File) {
        assert_eq!(f.index().try_into(), Ok(f));
    }

    #[proptest]
    fn subtracting_files_gives_distance(a: File, b: File) {
        assert_eq!(a - b, a.index() as i8 - b.index() as i8);
    }

    #[proptest]
    fn from_index_constructs_file_by_index(#[strategy(0u8..8)] i: u8) {
        assert_eq!(File::from_index(i).index(), i);
    }

    #[proptest]
    #[should_panic]
    fn from_index_panics_if_index_out_of_range(#[strategy(8u8..)] i: u8) {
        File::from_index(i);
    }

    #[proptest]
    fn converting_file_from_index_out_of_range_fails(#[strategy(8u8..)] i: u8) {
        assert_eq!(File::try_from(i), Err(FileOutOfRange));
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

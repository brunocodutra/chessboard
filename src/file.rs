use derive_more::{Display, Error};
use shakmaty as sm;
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;
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
}

/// The reason why parsing [`File`] failed.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Error)]
#[display(
    fmt = "unable to parse file; expected lower case letter in the range `[{}-{}]`",
    File::A,
    File::H
)]
pub struct ParseFileError;

impl FromStr for File {
    type Err = ParseFileError;

    #[instrument(level = "trace", err)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<char>().map_err(|_| ParseFileError)?.try_into()
    }
}

impl TryFrom<char> for File {
    type Error = ParseFileError;

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
            _ => Err(ParseFileError),
        }
    }
}

impl From<File> for char {
    fn from(f: File) -> char {
        (f as u8).into()
    }
}

#[doc(hidden)]
impl From<sm::File> for File {
    fn from(c: sm::File) -> Self {
        c.char().try_into().unwrap()
    }
}

#[doc(hidden)]
impl From<File> for sm::File {
    fn from(f: File) -> Self {
        sm::File::new(f as u32 - File::A as u32)
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
            assert_eq!(char::from(c).to_string().parse::<File>(), char::from(c).try_into());
        }

        #[test]
        fn parsing_file_fails_for_upper_case_letter(s in "[A-Z]") {
            assert_eq!(s.parse::<File>(), Err(ParseFileError));
        }

        #[test]
        fn parsing_file_fails_except_for_lower_case_letter_between_a_and_h(s in "[^a-h]*|[a-h]{2,}") {
            assert_eq!(s.parse::<File>(), Err(ParseFileError));
        }

        #[test]
        fn file_can_be_converted_into_char(f: File) {
            assert_eq!(char::from(f).try_into(), Ok(f));
        }

        #[test]
        fn file_has_an_equivalent_shakmaty_representation(f: File) {
            assert_eq!(File::from(sm::File::from(f)), f);
        }
    }
}

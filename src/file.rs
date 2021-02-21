use crate::foreign;
use derive_more::{Display, Error, From};
use std::str::FromStr;

/// A column of the board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum File {
    #[display(fmt = "a")]
    A,
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
#[derive(Debug, Display, Clone, Eq, PartialEq, Hash, Error, From)]
#[display(
    fmt = "unable to parse file from `{}`; expected lower case letter in the range `[{}-{}]`",
    _0,
    File::A,
    File::H
)]
#[from(forward)]
pub struct ParseFileError(#[error(not(source))] pub String);

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
            _ => Err(s.into()),
        }
    }
}

impl From<foreign::File> for File {
    fn from(f: foreign::File) -> Self {
        File::VARIANTS[f.to_index()]
    }
}

impl Into<foreign::File> for File {
    fn into(self) -> foreign::File {
        foreign::File::from_index(self as usize)
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
            assert_eq!(char::from(c).to_string().parse::<File>(), Ok(File::VARIANTS[usize::from(c - b'a')]));
        }


        #[test]
        fn parsing_file_fails_for_upper_case_letter(s in "[A-Z]") {
            assert_eq!(s.parse::<File>(), Err(ParseFileError(s)));
        }

        #[test]
        fn parsing_file_fails_except_for_lower_case_letter_between_a_and_h(s in "[^a-h]*|[a-h]{2,}") {
            assert_eq!(s.parse::<File>(), Err(ParseFileError(s)));
        }
    }
}

use crate::foreign;
use derive_more::{Display, Error};
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

/// The reason parsing a [`File`] failed.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Error)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(
    fmt = "unable to parse file, expected a sigle letter in the range [{}-{}]",
    "File::A",
    "File::H"
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
        fn parsing_file_fails_except_for_single_letters_between_a_and_h(f in "[^a-h]*") {
            assert_eq!(f.parse::<File>(), Err(ParseFileError));
        }
    }
}

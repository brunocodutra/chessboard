use crate::foreign;
use derive_more::Display;

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

impl From<foreign::File> for File {
    fn from(f: foreign::File) -> Self {
        File::VARIANTS[f.to_index()]
    }
}

impl Into<foreign::File> for File {
    fn into(self: Self) -> foreign::File {
        foreign::File::from_index(self as usize)
    }
}

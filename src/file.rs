use crate::foreign;

/// A column of the board.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum File {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
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

    pub fn to_str(self) -> &'static str {
        use File::*;
        match self {
            A => &"a",
            B => &"b",
            C => &"c",
            D => &"d",
            E => &"e",
            F => &"f",
            G => &"g",
            H => &"h",
        }
    }
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

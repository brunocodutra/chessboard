use crate::{chess::Mirror, util::Integer};
use cozy_chess as cc;
use derive_more::Display;
use std::ops::Sub;

/// A column on the chess board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(u8)]
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

unsafe impl const Integer for File {
    type Repr = u8;
    const MIN: Self::Repr = File::A as _;
    const MAX: Self::Repr = File::H as _;
}

impl const Mirror for File {
    #[inline(always)]
    fn mirror(&self) -> Self {
        File::from_repr(Self::MAX - self.repr())
    }
}

impl Sub for File {
    type Output = i8;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        self as i8 - rhs as i8
    }
}

#[doc(hidden)]
impl From<cc::File> for File {
    #[inline(always)]
    fn from(f: cc::File) -> Self {
        File::from_repr(f as _)
    }
}

#[doc(hidden)]
impl From<File> for cc::File {
    #[inline(always)]
    fn from(f: File) -> Self {
        cc::File::index_const(f as _)
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
    fn mirroring_file_returns_complement(f: File) {
        assert_eq!(f.mirror().repr(), File::MAX - f.repr());
    }

    #[proptest]
    fn subtracting_files_returns_distance(a: File, b: File) {
        assert_eq!(a - b, a as i8 - b as i8);
    }

    #[proptest]
    fn file_has_an_equivalent_cozy_chess_representation(f: File) {
        assert_eq!(File::from(cc::File::from(f)), f);
    }
}

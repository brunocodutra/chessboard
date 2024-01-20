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

impl File {
    pub const ALL: [Self; 8] = [
        File::A,
        File::B,
        File::C,
        File::D,
        File::E,
        File::F,
        File::G,
        File::H,
    ];

    /// Constructs [`File`] from index.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not in the range (0..=7).
    pub fn from_index(i: u8) -> Self {
        Self::ALL[i as usize]
    }

    /// This files's index in the range (0..=7).
    pub fn index(&self) -> u8 {
        *self as _
    }

    /// Mirrors this file.
    pub fn mirror(&self) -> Self {
        Self::from_index(File::H as u8 - *self as u8)
    }
}

impl Sub for File {
    type Output = i8;

    fn sub(self, rhs: Self) -> Self::Output {
        self.index() as i8 - rhs.index() as i8
    }
}

#[doc(hidden)]
impl From<cc::File> for File {
    #[inline(always)]
    fn from(f: cc::File) -> Self {
        File::from_index(f as _)
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
    use crate::util::Buffer;
    use std::mem::size_of;
    use test_strategy::proptest;

    #[proptest]
    fn file_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<File>>(), size_of::<File>());
    }

    #[proptest]
    fn file_has_an_index(f: File) {
        assert_eq!(File::from_index(f.index()), f);
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
    fn file_is_ordered_by_index(a: File, b: File) {
        assert_eq!(a < b, a.index() < b.index());
    }

    #[proptest]
    fn all_contains_files_in_order() {
        assert_eq!(
            File::ALL.into_iter().collect::<Buffer<_, 8>>(),
            (0..=7).map(File::from_index).collect()
        );
    }

    #[proptest]
    fn file_has_a_mirror(f: File) {
        assert_eq!(f.mirror().index(), 7 - f.index());
    }

    #[proptest]
    fn subtracting_files_gives_distance(a: File, b: File) {
        assert_eq!(a - b, a.index() as i8 - b.index() as i8);
    }

    #[proptest]
    fn file_has_an_equivalent_cozy_chess_representation(f: File) {
        assert_eq!(File::from(cc::File::from(f)), f);
    }
}

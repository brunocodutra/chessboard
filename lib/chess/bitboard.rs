use crate::chess::{File, Mirror, Perspective, Rank, Square};
use crate::util::{Assume, Integer};
use derive_more::{Debug, *};
use std::fmt::{self, Write};

/// A set of squares on a chess board.
#[derive(
    Default,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Constructor,
    Deref,
    BitAnd,
    BitAndAssign,
    BitOr,
    BitOrAssign,
    BitXor,
    BitXorAssign,
    Not,
)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(transparent)]
pub struct Bitboard(u64);

impl fmt::Debug for Bitboard {
    #[coverage(off)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('\n')?;
        for rank in Rank::iter().rev() {
            for file in File::iter() {
                let sq = Square::new(file, rank);
                f.write_char(if self.contains(sq) { '■' } else { '◻' })?;
                f.write_char(if file < File::H { ' ' } else { '\n' })?;
            }
        }

        Ok(())
    }
}

impl Bitboard {
    /// An empty board.
    #[inline(always)]
    pub const fn empty() -> Self {
        Bitboard(0)
    }

    /// A full board.
    #[inline(always)]
    pub const fn full() -> Self {
        Bitboard(0xFFFFFFFFFFFFFFFF)
    }

    /// Border squares.
    #[inline(always)]
    pub const fn border() -> Self {
        Bitboard(0xFF818181818181FF)
    }

    /// Light squares.
    #[inline(always)]
    pub const fn light() -> Self {
        Bitboard(0x55AA55AA55AA55AA)
    }

    /// Dark squares.
    #[inline(always)]
    pub const fn dark() -> Self {
        Bitboard(0xAA55AA55AA55AA55)
    }

    /// Fills out squares on a bitboard.
    ///
    /// Starting from a square, fills out the squares by stepping on the board in each direction.
    /// Movement in a direction stops when an occupied square is reached.
    ///
    /// # Example
    /// ```
    /// # use lib::chess::*;
    /// assert_eq!(
    ///     Vec::from_iter(Bitboard::fill(Square::E2, &[(-1, 2), (1, -1)], Square::C6.bitboard())),
    ///     vec![Square::F1, Square::E2, Square::D4, Square::C6]
    /// );
    /// ```
    #[inline(always)]
    pub const fn fill(sq: Square, steps: &[(i8, i8)], occupied: Bitboard) -> Bitboard {
        let mut bitboard = sq.bitboard();
        let mut i = steps.len();
        while i > 0 {
            i -= 1;
            let (df, dr) = steps[i];
            let mut sq = sq;
            let mut f = sq.file().get() + df;
            let mut r = sq.rank().get() + dr;
            while !occupied.contains(sq)
                && (File::MIN <= f && f <= File::MAX)
                && (Rank::MIN <= r && r <= Rank::MAX)
            {
                let file = File::new(f);
                let rank = Rank::new(r);
                sq = Square::new(file, rank);
                bitboard = bitboard.with(sq);
                f = sq.file().get() + df;
                r = sq.rank().get() + dr;
            }
        }

        bitboard
    }

    /// Bitboard with squares in line with two other squares.
    ///
    /// # Example
    /// ```
    /// # use lib::chess::*;
    /// assert_eq!(
    ///     Vec::from_iter(Bitboard::line(Square::B4, Square::E1)),
    ///     vec![Square::E1, Square::D2, Square::C3, Square::B4, Square::A5]
    /// );
    /// ```
    #[inline(always)]
    pub const fn line(whence: Square, whither: Square) -> Self {
        const TABLE: [[Bitboard; 64]; 64] = {
            let mut table = [[Bitboard::empty(); 64]; 64];
            let mut i = Square::MIN;
            while i <= Square::MAX {
                let wc = <Square as Integer>::new(i);
                let mut j = Square::MIN;
                while j <= Square::MAX {
                    let wt = <Square as Integer>::new(j);
                    let df = wt.file().get() - wc.file().get();
                    let dr = wt.rank().get() - wc.rank().get();
                    if df == 0 && dr == 0 {
                        table[i as usize][j as usize] = wc.bitboard();
                    } else if df == 0 {
                        table[i as usize][j as usize] = wc.file().bitboard();
                    } else if dr == 0 {
                        table[i as usize][j as usize] = wc.rank().bitboard();
                    } else if df.abs() == dr.abs() {
                        let steps = [(df.signum(), dr.signum()), (-df.signum(), -dr.signum())];
                        let bb = Bitboard::fill(wc, &steps, Bitboard::empty());
                        table[i as usize][j as usize] = bb;
                    }

                    j += 1;
                }

                i += 1;
            }

            table
        };

        TABLE[whence as usize][whither as usize]
    }

    /// Bitboard with squares in the open segment between two squares.
    ///
    /// # Example
    /// ```
    /// # use lib::chess::*;
    /// assert_eq!(
    ///     Vec::from_iter(Bitboard::segment(Square::B4, Square::E1)),
    ///     vec![Square::D2, Square::C3]
    /// );
    /// ```
    #[inline(always)]
    pub const fn segment(whence: Square, whither: Square) -> Self {
        const TABLE: [[Bitboard; 64]; 64] = {
            let mut table = [[Bitboard::empty(); 64]; 64];
            let mut i = Square::MIN;
            while i <= Square::MAX {
                let wc = <Square as Integer>::new(i);
                let mut j = Square::MIN;
                while j <= Square::MAX {
                    let wt = <Square as Integer>::new(j);
                    let df = wt.file().get() - wc.file().get();
                    let dr = wt.rank().get() - wc.rank().get();
                    if df == 0 || dr == 0 || df.abs() == dr.abs() {
                        let steps = [(df.signum(), dr.signum())];
                        let bb = Bitboard::fill(wc, &steps, wt.bitboard());
                        table[i as usize][j as usize] = bb.without(wc).without(wt);
                    }

                    j += 1;
                }

                i += 1;
            }

            table
        };

        TABLE[whence as usize][whither as usize]
    }

    /// The number of [`Square`]s in the set.
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.0.count_ones() as _
    }

    /// Whether the board is empty.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether this [`Square`] is in the set.
    #[inline(always)]
    pub const fn contains(&self, sq: Square) -> bool {
        !sq.bitboard().intersection(*self).is_empty()
    }

    /// Adds a [`Square`] to this bitboard.
    #[inline(always)]
    pub const fn with(&self, sq: Square) -> Self {
        sq.bitboard().union(*self)
    }

    /// Removes a [`Square`]s from this bitboard.
    #[inline(always)]
    pub const fn without(&self, sq: Square) -> Self {
        sq.bitboard().inverse().intersection(*self)
    }

    /// The set of [`Square`]s not in this bitboard.
    #[inline(always)]
    pub const fn inverse(&self) -> Self {
        Bitboard(!self.0)
    }

    /// The set of [`Square`]s in both bitboards.
    #[inline(always)]
    pub const fn intersection(&self, bb: Bitboard) -> Self {
        Bitboard(self.0 & bb.0)
    }

    /// The set of [`Square`]s in either bitboard.
    #[inline(always)]
    pub const fn union(&self, bb: Bitboard) -> Self {
        Bitboard(self.0 | bb.0)
    }

    /// An iterator over the [`Square`]s in this bitboard.
    #[inline(always)]
    pub const fn iter(&self) -> Squares {
        Squares::new(*self)
    }

    /// An iterator over the subsets of this bitboard.
    #[inline(always)]
    pub const fn subsets(&self) -> Subsets {
        Subsets::new(*self)
    }
}

impl const Mirror for Bitboard {
    /// Mirrors all squares in the set.
    #[inline(always)]
    fn mirror(&self) -> Self {
        Bitboard(self.0.reverse_bits())
    }
}

impl const Perspective for Bitboard {
    /// Flips all squares in the set.
    #[inline(always)]
    fn flip(&self) -> Self {
        Self(self.0.swap_bytes())
    }
}

impl From<File> for Bitboard {
    #[inline(always)]
    fn from(f: File) -> Self {
        f.bitboard()
    }
}

impl From<Rank> for Bitboard {
    #[inline(always)]
    fn from(r: Rank) -> Self {
        r.bitboard()
    }
}

impl From<Square> for Bitboard {
    #[inline(always)]
    fn from(sq: Square) -> Self {
        sq.bitboard()
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = Squares;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        Squares::new(self)
    }
}

/// An iterator over the [`Square`]s in a [`Bitboard`].
#[derive(Debug, Constructor)]
pub struct Squares(Bitboard);

impl Iterator for Squares {
    type Item = Square;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            None
        } else {
            let sq: Square = self.0.trailing_zeros().convert().assume();
            self.0 ^= sq.bitboard();
            Some(sq)
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for Squares {
    #[inline(always)]
    fn len(&self) -> usize {
        self.0.len()
    }
}

/// An iterator over the subsets of a [`Bitboard`].
#[derive(Debug)]
pub struct Subsets(u64, Option<u64>);

impl Subsets {
    #[inline(always)]
    pub const fn new(bb: Bitboard) -> Self {
        Self(bb.0, Some(0))
    }
}

impl Iterator for Subsets {
    type Item = Bitboard;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let bits = self.1?;
        self.1 = match bits.wrapping_sub(self.0) & self.0 {
            0 => None,
            next => Some(next),
        };

        Some(Bitboard(bits))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashSet, fmt::Debug};
    use test_strategy::proptest;

    #[test]
    fn empty_constructs_board_with_no_squares() {
        assert_eq!(Bitboard::empty().iter().count(), 0);
    }

    #[test]
    fn full_constructs_board_with_all_squares() {
        assert_eq!(Bitboard::full().iter().count(), 64);
    }

    #[test]
    fn border_constructs_bitboard_with_first_rank_eighth_rank_a_file_h_file() {
        assert_eq!(
            Bitboard::border(),
            Rank::First.bitboard()
                | Rank::Eighth.bitboard()
                | File::A.bitboard()
                | File::H.bitboard()
        );
    }

    #[proptest]
    fn line_contains_both_squares(a: Square, b: Square) {
        assert_eq!(
            Bitboard::line(a, b).contains(a),
            Bitboard::line(a, b).contains(b)
        );
    }

    #[proptest]
    fn line_degenerates_to_point(sq: Square) {
        assert_eq!(Bitboard::line(sq, sq), sq.bitboard());
    }

    #[proptest]
    fn line_contains_segment(a: Square, b: Square) {
        assert_eq!(
            Bitboard::line(a, b) & Bitboard::segment(a, b),
            Bitboard::segment(a, b),
        );
    }

    #[proptest]
    fn segment_does_not_contain_whence(a: Square, b: Square) {
        assert!(!Bitboard::segment(a, b).contains(a));
    }

    #[proptest]
    fn segment_does_not_contain_whither(a: Square, b: Square) {
        assert!(!Bitboard::segment(a, b).contains(b));
    }

    #[test]
    fn light_bitboards_contains_light_squares() {
        assert!(Bitboard::light()
            .iter()
            .all(|sq| (sq.file().get() + sq.rank().get()) % 2 != 0));
    }

    #[test]
    fn dark_bitboards_contains_dark_squares() {
        assert!(Bitboard::dark()
            .iter()
            .all(|sq| (sq.file().get() + sq.rank().get()) % 2 == 0));
    }

    #[test]
    fn squares_are_either_light_or_dark() {
        assert_eq!(Bitboard::light() ^ Bitboard::dark(), Bitboard::full());
    }

    #[proptest]
    fn len_returns_number_of_squares_on_the_board(bb: Bitboard) {
        assert_eq!(bb.len() as u32, bb.count_ones());
    }

    #[proptest]
    #[allow(clippy::len_zero)]
    fn is_empty_returns_whether_there_are_squares_on_the_board(bb: Bitboard) {
        assert_eq!(bb.is_empty(), bb.len() == 0);
    }

    #[proptest]
    fn contains_checks_whether_square_is_on_the_board(bb: Bitboard) {
        for sq in bb {
            assert!(bb.contains(sq));
        }
    }

    #[proptest]
    fn with_adds_square_to_set(bb: Bitboard, sq: Square) {
        assert!(bb.with(sq).contains(sq));
    }

    #[proptest]
    fn without_removes_square_to_set(bb: Bitboard, sq: Square) {
        assert!(!bb.without(sq).contains(sq));
    }

    #[proptest]
    fn inverse_returns_squares_not_in_set(bb: Bitboard) {
        let pp = bb.inverse();
        for sq in Square::iter() {
            assert_ne!(bb.contains(sq), pp.contains(sq));
        }
    }

    #[proptest]
    fn intersection_returns_squares_in_both_sets(a: Bitboard, b: Bitboard) {
        let c = a.intersection(b);
        for sq in Square::iter() {
            assert_eq!(c.contains(sq), a.contains(sq) && b.contains(sq));
        }
    }

    #[proptest]
    fn union_returns_squares_in_either_set(a: Bitboard, b: Bitboard) {
        let c = a.union(b);
        for sq in Square::iter() {
            assert_eq!(c.contains(sq), a.contains(sq) || b.contains(sq));
        }
    }

    #[proptest]
    fn mirroring_bitboard_mirrors_every_square(bb: Bitboard) {
        assert_eq!(
            HashSet::<Square>::from_iter(bb.mirror()),
            HashSet::<Square>::from_iter(bb.iter().map(|sq| sq.mirror()))
        );
    }

    #[proptest]
    fn flipping_a_bitboard_flips_every_square(bb: Bitboard) {
        assert_eq!(
            HashSet::<Square>::from_iter(bb.flip()),
            HashSet::<Square>::from_iter(bb.iter().map(|sq| sq.flip()))
        );
    }

    #[proptest]
    fn can_iterate_over_squares_in_a_bitboard(bb: Bitboard, sq: Square) {
        let v = Vec::from_iter(bb);
        assert_eq!(bb.iter().len(), v.len());
        assert_eq!(bb.contains(sq), v.contains(&sq));
    }

    #[proptest]
    fn can_iterate_over_subsets_of_a_bitboard(a: [Square; 6], b: [Square; 3]) {
        let a = a.into_iter().fold(Bitboard::empty(), |bb, sq| bb.with(sq));
        let b = b.into_iter().fold(Bitboard::empty(), |bb, sq| bb.with(sq));
        let set: HashSet<_> = a.subsets().collect();
        assert_eq!(a & b == b, set.contains(&b));
    }

    #[proptest]
    fn bitboard_can_be_created_from_file(f: File) {
        assert_eq!(Bitboard::from(f), f.bitboard());
    }

    #[proptest]
    fn bitboard_can_be_created_from_rank(r: Rank) {
        assert_eq!(Bitboard::from(r), r.bitboard());
    }

    #[proptest]
    fn bitboard_can_be_created_from_square(sq: Square) {
        assert_eq!(Bitboard::from(sq), sq.bitboard());
    }
}

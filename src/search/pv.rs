use super::Transposition;
use crate::chess::Move;
use arrayvec::ArrayVec;
use derive_more::{Deref, DerefMut, Display, IntoIterator};

/// The [principal variation].
///
/// [principal variation]: https://www.chessprogramming.org/Principal_Variation
#[derive(Debug, Display, Default, Clone, Eq, PartialEq, Hash, Deref, DerefMut, IntoIterator)]
#[display(
    fmt = "{}",
    "self.iter().map(Move::to_string).collect::<ArrayVec<_, N>>().join(\" \")"
)]
pub struct Pv<const N: usize> {
    #[deref(forward)]
    #[deref_mut(forward)]
    #[into_iterator(owned, ref, ref_mut)]
    moves: ArrayVec<Move, N>,
    info: Option<(u8, i16)>,
}

impl<const N: usize> Pv<N> {
    /// The depth of this sequence.
    pub fn depth(&self) -> Option<u8> {
        self.info.map(|(d, _)| d)
    }

    /// This sequence's score from the point of view of the side to move.
    pub fn score(&self) -> Option<i16> {
        self.info.map(|(_, s)| s)
    }

    /// The number of moves in this sequence.
    pub fn len(&self) -> usize {
        self.moves.len()
    }

    /// If this sequence has at least one move.
    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    /// Iterate over the moves in this sequence.
    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }

    /// Truncates sequence to `M` moves.
    ///
    /// No moves are discarded if `M >= self.len()`.
    pub fn truncate<const M: usize>(self) -> Pv<M> {
        Pv {
            info: self.info,
            moves: self.moves.into_iter().take(M).collect(),
        }
    }
}

/// Create a [`Pv`] from an iterator.
///
/// Truncates the sequence at the N-th [`Transposition`].
///
/// # Panics
/// Panics if the sequence is not strictly decreasing by draft.
impl<const N: usize> FromIterator<Transposition> for Pv<N> {
    fn from_iter<I: IntoIterator<Item = Transposition>>(tts: I) -> Self {
        let mut tts = tts.into_iter().filter(|t| t.draft() > 0).peekable();
        let info = tts.peek().map(|t| (t.draft() as u8, t.score()));

        let mut draft = i8::MAX;
        let moves = ArrayVec::from_iter(tts.take(N).map(|t| {
            assert!(t.draft() <= draft);
            draft = t.draft() - 1;
            t.best()
        }));

        Pv { info, moves }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::{MoveKind, Position};
    use crate::search::TranspositionTable;
    use proptest::prop_assume;
    use proptest::sample::{size_range, Selector};
    use test_strategy::proptest;

    #[proptest]
    fn len_returns_number_of_moves_in_the_sequence(tt: TranspositionTable, pos: Position) {
        assert_eq!(tt.iter(&pos).collect::<Pv<0>>().len(), 0);
        assert_eq!(
            tt.iter(&pos).collect::<Pv<10>>().len(),
            tt.iter(&pos).filter(|t| t.draft() > 0).count()
        );
    }

    #[proptest]
    fn is_empty_returns_whether_there_are_no_moves_in_the_sequence(
        tt: TranspositionTable,
        pos: Position,
    ) {
        assert!(tt.iter(&pos).collect::<Pv<0>>().is_empty());
        assert_eq!(
            tt.iter(&pos).collect::<Pv<10>>().is_empty(),
            tt.iter(&pos).filter(|t| t.draft() > 0).count() == 0
        );
    }

    #[proptest]
    fn collects_truncated_sequence(
        tt: TranspositionTable,
        #[by_ref]
        #[filter(#pos.moves(MoveKind::ANY).len() > 0)]
        pos: Position,
        s: i16,
        #[strategy(1i8..=Transposition::MAX_DRAFT)] d: i8,
        selector: Selector,
    ) {
        let (m, _, next) = selector.select(pos.moves(MoveKind::ANY));
        prop_assume!(next.moves(MoveKind::ANY).len() > 0);

        let (n, _, _) = selector.select(next.moves(MoveKind::ANY));

        let t = Transposition::lower(s, d, m);
        tt.unset(pos.zobrist());
        tt.set(pos.zobrist(), t);

        let u = Transposition::lower(s, d - 1, n);
        tt.unset(next.zobrist());
        tt.set(next.zobrist(), u);

        prop_assume!(tt.get(pos.zobrist()) == Some(t));
        prop_assume!(tt.get(next.zobrist()) == Some(u));

        assert!(tt.iter(&pos).count() > 1);
        assert_eq!(&tt.iter(&pos).collect::<Pv<1>>()[..], [t.best()].as_slice());
    }

    #[proptest]
    fn collects_positive_draft_only(
        tt: TranspositionTable,
        #[by_ref]
        #[filter(#pos.moves(MoveKind::ANY).len() > 0)]
        pos: Position,
        s: i16,
        #[strategy(1..=Transposition::MAX_DRAFT)] a: i8,
        #[strategy(Transposition::MIN_DRAFT..=0)] b: i8,
        selector: Selector,
    ) {
        let (m, _, next) = selector.select(pos.moves(MoveKind::ANY));
        prop_assume!(next.moves(MoveKind::ANY).len() > 0);

        let (n, _, _) = selector.select(next.moves(MoveKind::ANY));

        let t = Transposition::lower(s, a, m);
        tt.unset(pos.zobrist());
        tt.set(pos.zobrist(), t);

        let u = Transposition::lower(s, b, n);
        tt.unset(next.zobrist());
        tt.set(next.zobrist(), u);

        prop_assume!(tt.get(pos.zobrist()) == Some(t));
        prop_assume!(tt.get(next.zobrist()) == Some(u));

        assert!(tt.iter(&pos).count() > 1);
        assert_eq!(
            &tt.iter(&pos).collect::<Pv<10>>()[..],
            [t.best()].as_slice()
        );
    }

    #[proptest]
    fn depth_and_score_are_available_even_if_n_is_0(
        tt: TranspositionTable,
        #[by_ref]
        #[filter(#pos.moves(MoveKind::ANY).len() > 0)]
        pos: Position,
        s: i16,
        #[strategy(1..Transposition::MAX_DRAFT)] d: i8,
        selector: Selector,
    ) {
        let (m, _, _) = selector.select(pos.moves(MoveKind::ANY));

        let t = Transposition::lower(s, d, m);
        tt.unset(pos.zobrist());
        tt.set(pos.zobrist(), t);

        let pv: Pv<0> = tt.iter(&pos).collect();
        assert_eq!(pv.depth(), Some(t.draft() as u8));
        assert_eq!(pv.score(), Some(t.score()));
        assert_eq!(&pv[..], [].as_slice())
    }

    #[proptest]
    fn depth_and_score_are_not_available_if_draft_is_not_positive(
        tt: TranspositionTable,
        #[by_ref]
        #[filter(#pos.moves(MoveKind::ANY).len() > 0)]
        pos: Position,
        s: i16,
        #[strategy(Transposition::MIN_DRAFT..=0)] d: i8,
        selector: Selector,
    ) {
        let (m, _, _) = selector.select(pos.moves(MoveKind::ANY));

        let t = Transposition::lower(s, d, m);
        tt.unset(pos.zobrist());
        tt.set(pos.zobrist(), t);

        let pv: Pv<10> = tt.iter(&pos).collect();
        assert_eq!(pv.depth(), None);
        assert_eq!(pv.score(), None);
        assert_eq!(&pv[..], [].as_slice())
    }

    #[proptest]
    #[should_panic]
    fn panics_if_sequence_is_not_strictly_decreasing_by_draft(
        #[by_ref]
        #[any(size_range(2..=10).lift())]
        mut tts: Vec<Transposition>,
    ) {
        tts.sort_by_key(|t| -t.draft());
        tts.rotate_left(1);
        let _: Pv<10> = tts.iter().copied().collect();
    }
}

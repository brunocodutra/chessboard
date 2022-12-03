use crate::{chess::Move, eval::Value, transposition::Transposition};
use arrayvec::ArrayVec;
use derive_more::{Deref, DerefMut, Display, IntoIterator};
use proptest::prelude::*;
use test_strategy::Arbitrary;

/// The [principal variation].
///
/// [principal variation]: https://www.chessprogramming.org/Principal_Variation
#[derive(
    Debug, Display, Default, Clone, Eq, PartialEq, Hash, Arbitrary, Deref, DerefMut, IntoIterator,
)]
#[display(
    fmt = "{}",
    "self.iter().map(Move::to_string).collect::<ArrayVec<_, N>>().join(\" \")"
)]
pub struct Pv<const N: usize> {
    #[strategy(any::<Vec<Move>>().prop_map(|v| v.into_iter().take(N).collect()))]
    #[deref(forward)]
    #[deref_mut(forward)]
    #[into_iterator(owned, ref, ref_mut)]
    moves: ArrayVec<Move, N>,
    #[strategy(any::<(u8, Value)>().prop_map(move |i| #moves.first().map(move |_| i)))]
    info: Option<(u8, Value)>,
}

impl<const N: usize> Pv<N> {
    /// Constructs a new [`Pv`] given depth, score, and sequence of moves.
    pub fn new<I: IntoIterator<Item = Move>>(depth: u8, score: Value, moves: I) -> Self {
        Self {
            moves: moves.into_iter().take(N).collect(),
            info: Some((depth, score)),
        }
    }

    /// The depth of this sequence.
    pub fn depth(&self) -> Option<u8> {
        self.info.map(|(d, _)| d)
    }

    /// This sequence's score from the point of view of the side to move.
    pub fn score(&self) -> Option<Value> {
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
/// Panics if the sequence is not strictly decreasing by depth.
impl<const N: usize> FromIterator<Transposition> for Pv<N> {
    fn from_iter<I: IntoIterator<Item = Transposition>>(tts: I) -> Self {
        let mut tts = tts.into_iter().filter(|t| t.depth() > 0).peekable();
        let info = tts.peek().map(|t| (t.depth(), t.score()));

        let mut depth = u8::MAX;
        let moves = ArrayVec::from_iter(tts.take(N).map(|t| {
            assert!(t.depth() <= depth);
            depth = t.depth() - 1;
            t.best()
        }));

        Pv { info, moves }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::{MoveKind, Position};
    use crate::transposition::Table;
    use proptest::prop_assume;
    use proptest::sample::{size_range, Selector};
    use test_strategy::proptest;

    #[proptest]
    fn new_truncates_moves(d: u8, s: Value, #[any(size_range(10..20).lift())] m: Vec<Move>) {
        assert_eq!(
            Pv::<10>::new(d, s, m.clone()),
            Pv {
                moves: m[..10].iter().copied().collect(),
                info: Some((d, s)),
            }
        );
    }

    #[proptest]
    fn len_returns_number_of_moves_in_the_sequence(tt: Table, pos: Position) {
        assert_eq!(tt.iter(&pos).collect::<Pv<0>>().len(), 0);
        assert_eq!(
            tt.iter(&pos).collect::<Pv<10>>().len(),
            tt.iter(&pos).filter(|t| t.depth() > 0).count()
        );
    }

    #[proptest]
    fn is_empty_returns_whether_there_are_no_moves_in_the_sequence(tt: Table, pos: Position) {
        assert!(tt.iter(&pos).collect::<Pv<0>>().is_empty());
        assert_eq!(
            tt.iter(&pos).collect::<Pv<10>>().is_empty(),
            tt.iter(&pos).filter(|t| t.depth() > 0).count() == 0
        );
    }

    #[proptest]
    fn collects_truncated_sequence(
        tt: Table,
        #[by_ref]
        #[filter(#pos.moves(MoveKind::ANY).len() > 0)]
        pos: Position,
        s: Value,
        #[strategy(1u8..=Transposition::MAX_DEPTH)] d: u8,
        selector: Selector,
    ) {
        let (m, next) = selector.select(pos.moves(MoveKind::ANY));
        prop_assume!(next.moves(MoveKind::ANY).len() > 0);

        let (n, _) = selector.select(next.moves(MoveKind::ANY));

        let t = Transposition::lower(d, s, m);
        tt.unset(pos.zobrist());
        tt.set(pos.zobrist(), t);

        let u = Transposition::lower(d - 1, s, n);
        tt.unset(next.zobrist());
        tt.set(next.zobrist(), u);

        prop_assume!(tt.get(pos.zobrist()) == Some(t));
        prop_assume!(tt.get(next.zobrist()) == Some(u));

        assert!(tt.iter(&pos).count() > 1);
        assert_eq!(&tt.iter(&pos).collect::<Pv<1>>()[..], [t.best()].as_slice());
    }

    #[proptest]
    fn collects_positive_depth_only(
        tt: Table,
        #[by_ref]
        #[filter(#pos.moves(MoveKind::ANY).len() > 0)]
        pos: Position,
        s: Value,
        #[strategy(1..=Transposition::MAX_DEPTH)] a: u8,
        selector: Selector,
    ) {
        let (m, next) = selector.select(pos.moves(MoveKind::ANY));
        prop_assume!(next.moves(MoveKind::ANY).len() > 0);

        let (n, _) = selector.select(next.moves(MoveKind::ANY));

        let t = Transposition::lower(a, s, m);
        tt.unset(pos.zobrist());
        tt.set(pos.zobrist(), t);

        let u = Transposition::lower(0, s, n);
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
        tt: Table,
        #[by_ref]
        #[filter(#pos.moves(MoveKind::ANY).len() > 0)]
        pos: Position,
        s: Value,
        #[strategy(1..Transposition::MAX_DEPTH)] d: u8,
        selector: Selector,
    ) {
        let (m, _) = selector.select(pos.moves(MoveKind::ANY));

        let t = Transposition::lower(d, s, m);
        tt.unset(pos.zobrist());
        tt.set(pos.zobrist(), t);

        let pv: Pv<0> = tt.iter(&pos).collect();
        assert_eq!(pv.depth(), Some(t.depth()));
        assert_eq!(pv.score(), Some(t.score()));
        assert_eq!(&pv[..], [].as_slice())
    }

    #[proptest]
    fn depth_and_score_are_not_available_if_depth_is_not_positive(
        tt: Table,
        #[by_ref]
        #[filter(#pos.moves(MoveKind::ANY).len() > 0)]
        pos: Position,
        s: Value,
        selector: Selector,
    ) {
        let (m, _) = selector.select(pos.moves(MoveKind::ANY));

        let t = Transposition::lower(0, s, m);
        tt.unset(pos.zobrist());
        tt.set(pos.zobrist(), t);

        let pv: Pv<10> = tt.iter(&pos).collect();
        assert_eq!(pv.depth(), None);
        assert_eq!(pv.score(), None);
        assert_eq!(&pv[..], [].as_slice())
    }

    #[proptest]
    #[should_panic]
    fn panics_if_sequence_is_not_strictly_decreasing_by_depth(
        #[by_ref]
        #[any(size_range(2..=10).lift())]
        mut tts: Vec<Transposition>,
    ) {
        tts.sort_by_key(|t| !t.depth());
        tts.rotate_left(1);
        let _: Pv<10> = tts.iter().copied().collect();
    }
}

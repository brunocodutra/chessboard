use super::{Transposition, TranspositionTable};
use crate::chess::Position;

/// An iterator over the sequence of [`Transposition`]s in a [`TranspositionTable`].
#[derive(Debug, Clone)]
pub struct TranspositionIterator<'a> {
    tt: &'a TranspositionTable,
    pos: Position,
    draft: Option<i8>,
}

impl<'a> TranspositionIterator<'a> {
    pub fn new(tt: &'a TranspositionTable, pos: Position) -> Self {
        TranspositionIterator {
            tt,
            pos,
            draft: Some(i8::MAX),
        }
    }
}

impl<'a> Iterator for TranspositionIterator<'a> {
    type Item = Transposition;

    fn next(&mut self) -> Option<Self::Item> {
        let d = self.draft?;
        let key = self.pos.zobrist();
        let t = self.tt.get(key).filter(|t| t.draft() <= d)?;
        self.draft = t.draft().checked_sub(1);
        self.pos.make(t.best()).ok()?;
        Some(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::MoveKind;
    use proptest::{prop_assume, sample::Selector};
    use test_strategy::proptest;

    #[proptest]
    fn iterates_over_moves_searched_with_strictly_decreasing_draft(
        #[by_ref]
        #[filter(#tt.capacity() > 1)]
        tt: TranspositionTable,
        #[by_ref]
        #[filter(#pos.moves(MoveKind::ANY).len() > 0)]
        pos: Position,
        #[strategy(Transposition::MIN_DRAFT..=Transposition::MAX_DRAFT)] d: i8,
        s: i16,
        selector: Selector,
    ) {
        let (m, _, next) = selector.select(pos.moves(MoveKind::ANY));
        prop_assume!(next.moves(MoveKind::ANY).len() > 0);

        let (n, _, _) = selector.select(next.moves(MoveKind::ANY));

        let t = Transposition::lower(s, d, m);
        tt.unset(pos.zobrist());
        tt.set(pos.zobrist(), t);

        let u = Transposition::lower(s.saturating_neg(), d, n);
        tt.unset(next.zobrist());
        tt.set(next.zobrist(), u);

        prop_assume!(tt.get(pos.zobrist()) == Some(t));
        prop_assume!(tt.get(next.zobrist()) == Some(u));

        let mut pv = TranspositionIterator::new(&tt, pos);
        assert_eq!(pv.next(), Some(t));
        assert_eq!(pv.next(), None);
    }

    #[proptest]
    fn iterates_over_legal_moves_only(
        tt: TranspositionTable,
        #[by_ref] pos: Position,
        #[filter(#pos.clone().make(#t.best()).is_err())] t: Transposition,
    ) {
        tt.unset(pos.zobrist());
        tt.set(pos.zobrist(), t);
        assert_eq!(TranspositionIterator::new(&tt, pos).next(), None);
    }

    #[proptest]
    fn is_fused(tt: TranspositionTable, pos: Position) {
        let mut pv = TranspositionIterator::new(&tt, pos);

        while pv.next().is_some() {}

        assert_eq!(pv.next(), None);
        assert_eq!(pv.next(), None);
    }
}

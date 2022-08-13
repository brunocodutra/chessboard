use crate::{Position, Transposition, TranspositionTable};

/// The strongest sequence from a starting [`Position`].
#[derive(Debug, Clone)]
pub struct Pv<'a> {
    tt: &'a TranspositionTable,
    pos: Position,
    draft: Option<i8>,
}

impl<'a> Pv<'a> {
    pub fn new(tt: &'a TranspositionTable, pos: Position) -> Self {
        Pv {
            tt,
            pos,
            draft: Some(i8::MAX),
        }
    }
}

impl<'a> Iterator for Pv<'a> {
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
    use crate::Move;
    use proptest::{prop_assume, sample::Selector};
    use test_strategy::proptest;

    #[proptest]
    fn pv_iterates_over_moves_searched_with_strictly_decreasing_draft(
        #[by_ref]
        #[filter(#tt.capacity() > 1)]
        tt: TranspositionTable,
        #[by_ref]
        #[filter(#pos.moves().len() > 0)]
        pos: Position,
        #[strategy(Transposition::MIN_DRAFT..=Transposition::MAX_DRAFT)] d: i8,
        s: i16,
        selector: Selector,
    ) {
        let (m, next) = selector.select(pos.moves());
        prop_assume!(next.moves().len() > 0);

        let (n, _) = selector.select(pos.moves());

        let t = Transposition::lower(s, d, m);
        tt.set(pos.zobrist(), t);

        let u = Transposition::lower(s.saturating_neg(), d, n);
        tt.set(next.zobrist(), u);

        prop_assume!(tt.get(pos.zobrist()) == Some(t));
        prop_assume!(tt.get(next.zobrist()) == Some(u));

        let mut pv = Pv::new(&tt, pos);
        assert_eq!(pv.next(), Some(t));
        assert_eq!(pv.next(), None);
    }

    #[proptest]
    fn pv_iterates_over_legal_moves_only(
        tt: TranspositionTable,
        #[by_ref] pos: Position,
        #[strategy(Transposition::MIN_DRAFT..=Transposition::MAX_DRAFT)] d: i8,
        s: i16,
        #[filter(#pos.clone().make(#m).is_err())] m: Move,
    ) {
        let t = Transposition::lower(s, d, m);
        tt.set(pos.zobrist(), t);
        prop_assume!(tt.get(pos.zobrist()) == Some(t));
        assert_eq!(Pv::new(&tt, pos).next(), None);
    }

    #[proptest]
    fn pv_is_fused(tt: TranspositionTable, pos: Position) {
        let mut pv = Pv::new(&tt, pos);

        while pv.next().is_some() {}

        assert_eq!(pv.next(), None);
        assert_eq!(pv.next(), None);
    }
}

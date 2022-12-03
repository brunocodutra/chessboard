use super::{Table, Transposition};
use crate::chess::Position;

/// An iterator over the sequence of [`Transposition`]s in a transposition [`Table`].
#[derive(Debug, Clone)]
pub struct Iter<'a> {
    tt: &'a Table,
    pos: Position,
    depth: Option<u8>,
}

impl<'a> Iter<'a> {
    pub fn new(tt: &'a Table, pos: Position) -> Self {
        Iter {
            tt,
            pos,
            depth: Some(u8::MAX),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Transposition;

    fn next(&mut self) -> Option<Self::Item> {
        let d = self.depth?;
        let key = self.pos.zobrist();
        let t = self.tt.get(key).filter(|t| t.depth() <= d)?;
        self.depth = t.depth().checked_sub(1);
        self.pos.make(t.best()).ok()?;
        Some(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{chess::MoveKind, eval::Value};
    use proptest::{prop_assume, sample::Selector};
    use test_strategy::proptest;

    #[proptest]
    fn iterates_over_moves_searched_with_strictly_decreasing_depth(
        #[by_ref]
        #[filter(#tt.capacity() > 1)]
        tt: Table,
        #[by_ref]
        #[filter(#pos.moves(MoveKind::ANY).len() > 0)]
        pos: Position,
        #[strategy(0..=Transposition::MAX_DEPTH)] d: u8,
        s: Value,
        selector: Selector,
    ) {
        let (m, next) = selector.select(pos.moves(MoveKind::ANY));
        prop_assume!(next.moves(MoveKind::ANY).len() > 0);

        let (n, _) = selector.select(next.moves(MoveKind::ANY));

        let t = Transposition::lower(d, s, m);
        tt.unset(pos.zobrist());
        tt.set(pos.zobrist(), t);

        let u = Transposition::lower(d, -s, n);
        tt.unset(next.zobrist());
        tt.set(next.zobrist(), u);

        prop_assume!(tt.get(pos.zobrist()) == Some(t));
        prop_assume!(tt.get(next.zobrist()) == Some(u));

        let mut pv = Iter::new(&tt, pos);
        assert_eq!(pv.next(), Some(t));
        assert_eq!(pv.next(), None);
    }

    #[proptest]
    fn iterates_over_legal_moves_only(
        tt: Table,
        #[by_ref] pos: Position,
        #[filter(#pos.clone().make(#t.best()).is_err())] t: Transposition,
    ) {
        tt.unset(pos.zobrist());
        tt.set(pos.zobrist(), t);
        assert_eq!(Iter::new(&tt, pos).next(), None);
    }

    #[proptest]
    fn is_fused(tt: Table, pos: Position) {
        let mut pv = Iter::new(&tt, pos);

        while pv.next().is_some() {}

        assert_eq!(pv.next(), None);
        assert_eq!(pv.next(), None);
    }
}

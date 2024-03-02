use crate::chess::{Color, Move};
use crate::search::Ply;

/// A set of [killer moves] indexed by [`Ply`] and side to move.
///
/// [killer moves]: https://www.chessprogramming.org/Killer_Move
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Killers<const P: usize>([[[Option<Move>; 2]; 2]; P]);

impl<const P: usize> Killers<P> {
    /// Constructs an empty set of killer moves.
    #[inline(always)]
    pub const fn new() -> Self {
        Killers([[[None; 2]; 2]; P])
    }

    /// Adds a killer move to the set at a given ply for a given side to move.
    #[inline(always)]
    pub fn insert(&mut self, ply: Ply, side: Color, m: Move) {
        if let Some(ks) = self.0.get_mut(ply.get() as usize) {
            let [first, last] = &mut ks[side as usize];
            if *first != Some(m) {
                *last = *first;
                *first = Some(m);
            }
        }
    }

    /// Checks whether move is a known killer at a given ply for a given side to move.
    #[inline(always)]
    pub fn contains(&self, ply: Ply, side: Color, m: Move) -> bool {
        self.0
            .get(ply.get() as usize)
            .is_some_and(|ks| ks[side as usize].contains(&Some(m)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Integer;
    use proptest::sample::size_range;
    use std::collections::HashSet;
    use test_strategy::proptest;

    #[proptest]
    fn insert_avoids_duplicated_moves(#[filter(#p >= 0)] p: Ply, c: Color, m: Move) {
        let mut ks = Killers::<{ Ply::MAX as usize + 1 }>::new();

        ks.insert(p, c, m);
        ks.insert(p, c, m);

        assert_eq!(ks.0[p.get() as usize][c as usize], [Some(m), None]);
    }

    #[proptest]
    fn insert_keeps_most_recent(
        #[filter(#p >= 0)] p: Ply,
        c: Color,
        #[any(size_range(2..10).lift())] ms: HashSet<Move>,
        m: Move,
    ) {
        let mut ks = Killers::<{ Ply::MAX as usize + 1 }>::new();

        for m in ms {
            ks.insert(p, c, m);
        }

        ks.insert(p, c, m);
        assert_eq!(ks.0[p.get() as usize][c as usize][0], Some(m));
    }

    #[proptest]
    fn insert_ignores_ply_out_of_bounds(
        mut ks: Killers<1>,
        #[filter(#p > 0)] p: Ply,
        c: Color,
        m: Move,
    ) {
        let prev = ks;
        ks.insert(p, c, m);
        assert_eq!(ks, prev);
    }

    #[proptest]
    fn contains_returns_true_only_if_inserted(#[filter(#p >= 0)] p: Ply, c: Color, m: Move) {
        let mut ks = Killers::<{ Ply::MAX as usize + 1 }>::new();
        assert!(!ks.contains(p, c, m));
        ks.insert(p, c, m);
        assert!(ks.contains(p, c, m));
    }

    #[proptest]
    fn contains_returns_false_if_ply_out_of_bounds(
        ks: Killers<1>,
        #[filter(#p > 0)] p: Ply,
        c: Color,
        m: Move,
    ) {
        assert!(!ks.contains(p, c, m));
    }
}

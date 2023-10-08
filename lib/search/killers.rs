use crate::chess::{Color, Move};
use crate::search::Ply;

/// A set of [killer moves] indexed by [`Ply`] and side to move.
///
/// [killer moves]: https://www.chessprogramming.org/Killer_Move
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Killers<const N: usize, const P: usize>([[[Option<Move>; N]; 2]; P]);

impl<const N: usize, const P: usize> Default for Killers<N, P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize, const P: usize> Killers<N, P> {
    /// Constructs an empty set of killer moves.
    pub const fn new() -> Self {
        Killers([[[None; N]; 2]; P])
    }

    /// Adds a killer move to the set at a given ply for a given side to move.
    pub fn insert(&mut self, ply: Ply, side: Color, m: Move) {
        if let Some(ks) = self.0.get_mut(ply.get() as usize) {
            if !ks[side as usize].contains(&Some(m)) {
                ks[side as usize].rotate_right(1);
                ks[side as usize][0] = Some(m);
            }
        }
    }

    /// Checks whether move is a known killer at a given ply for a given side to move.
    pub fn contains(&self, ply: Ply, side: Color, m: Move) -> bool {
        self.0
            .get(ply.get() as usize)
            .is_some_and(|ks| ks[side as usize].contains(&Some(m)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{search::PlyBounds, util::Bounds};
    use proptest::sample::size_range;
    use std::collections::HashSet;
    use test_strategy::proptest;

    #[proptest]
    fn insert_avoids_duplicated_moves(#[filter(#p >= 0)] p: Ply, c: Color, m: Move) {
        let mut ks = Killers::<2, { PlyBounds::UPPER as usize + 1 }>::default();

        ks.insert(p, c, m);
        ks.insert(p, c, m);

        assert_eq!(ks.0[p.get() as usize][c as usize], [Some(m), None]);
    }

    #[proptest]
    fn insert_keeps_most_recent(
        #[filter(#p >= 0)] p: Ply,
        c: Color,
        #[any(size_range(2..10).lift())] ms: HashSet<Move>,
        #[filter(!#ms.contains(&#m))] m: Move,
    ) {
        let mut ks = Killers::<1, { PlyBounds::UPPER as usize + 1 }>::default();

        for m in ms {
            ks.insert(p, c, m);
        }

        ks.insert(p, c, m);
        assert_eq!(ks.0[p.get() as usize][c as usize], [Some(m)]);
    }

    #[proptest]
    fn insert_ignores_ply_out_of_bounds(
        mut ks: Killers<2, 1>,
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
        let mut ks = Killers::<2, { PlyBounds::UPPER as usize + 1 }>::default();
        assert!(!ks.contains(p, c, m));
        ks.insert(p, c, m);
        assert!(ks.contains(p, c, m));
    }

    #[proptest]
    fn contains_returns_false_if_ply_out_of_bounds(
        ks: Killers<2, 1>,
        #[filter(#p > 0)] p: Ply,
        c: Color,
        m: Move,
    ) {
        assert!(!ks.contains(p, c, m));
    }
}

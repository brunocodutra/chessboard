use crate::{chess::Move, search::Score};
use derive_more::{Constructor, Deref};
use std::cmp::Ordering;
use std::ops::{Neg, Shr};

/// The [principal variation].
///
/// [principal variation]: https://www.chessprogramming.org/Principal_Variation
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Constructor, Deref)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Pv {
    score: Score,
    #[deref]
    r#move: Option<Move>,
}

impl Pv {
    /// The score from the point of view of the side to move.
    #[inline(always)]
    pub fn score(&self) -> Score {
        self.score
    }
}

impl Ord for Pv {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl PartialOrd for Pv {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> PartialEq<T> for Pv
where
    Score: PartialEq<T>,
{
    #[inline(always)]
    fn eq(&self, other: &T) -> bool {
        self.score.eq(other)
    }
}

impl<T> PartialOrd<T> for Pv
where
    Score: PartialOrd<T>,
{
    #[inline(always)]
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        self.score.partial_cmp(other)
    }
}

impl Neg for Pv {
    type Output = Self;

    #[inline(always)]
    fn neg(mut self) -> Self::Output {
        self.score = -self.score;
        self
    }
}

impl Shr<Pv> for Move {
    type Output = Pv;

    #[inline(always)]
    fn shr(self, mut pv: Pv) -> Self::Output {
        pv.r#move = Some(self);
        pv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn score_returns_score(pv: Pv) {
        assert_eq!(pv.score(), pv.score);
    }

    #[proptest]
    fn negation_changes_score(pv: Pv) {
        assert_eq!(pv.neg().score(), -pv.score());
    }

    #[proptest]
    fn negation_preserves_best(pv: Pv) {
        assert_eq!(*pv.neg(), *pv);
    }

    #[proptest]
    fn shift_changes_best(pv: Pv, m: Move) {
        assert_eq!(*m.shr(pv), Some(m));
    }

    #[proptest]
    fn shift_preserves_score(pv: Pv, m: Move) {
        assert_eq!(m.shr(pv).score(), pv.score());
    }

    #[proptest]
    fn pv_with_larger_score_is_larger(p: Pv, #[filter(#p.score() != #q.score())] q: Pv) {
        assert_eq!(p < q, p.score() < q.score());
    }
}

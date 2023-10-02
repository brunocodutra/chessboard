use crate::chess::Move;
use crate::search::{DepthBounds, Score};
use crate::util::{Bounds, Buffer};
use derive_more::{Deref, DerefMut, IntoIterator};
use std::{cmp::Ordering, ops::Neg};

/// The [principal variation].
///
/// [principal variation]: https://www.chessprogramming.org/Principal_Variation
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deref, DerefMut, IntoIterator)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Pv<const N: usize = { DepthBounds::UPPER as _ }> {
    score: Score,
    #[deref]
    #[deref_mut]
    #[into_iterator(owned)]
    line: Buffer<Move, N>,
}

impl<const N: usize> Pv<N> {
    /// Constructs a pv.
    pub fn new<I: IntoIterator<Item = Move>>(score: Score, line: I) -> Self {
        Pv {
            score,
            line: Buffer::from_iter(line),
        }
    }

    /// The score from the point of view of the side to move.
    pub fn score(&self) -> Score {
        self.score
    }

    /// The strongest [`Line`].
    pub fn line(&self) -> &[Move] {
        &self.line
    }
}

impl<const N: usize> Ord for Pv<N> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl<const N: usize> PartialOrd for Pv<N> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T, const N: usize> PartialEq<T> for Pv<N>
where
    Score: PartialEq<T>,
{
    fn eq(&self, other: &T) -> bool {
        self.score.eq(other)
    }
}

impl<T, const N: usize> PartialOrd<T> for Pv<N>
where
    Score: PartialOrd<T>,
{
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        self.score.partial_cmp(other)
    }
}

impl<const N: usize> Neg for Pv<N> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Pv::new(-self.score, self.line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn score_returns_score(pv: Pv<3>) {
        assert_eq!(pv.score(), pv.score);
    }

    #[proptest]
    fn line_returns_line(pv: Pv<3>) {
        assert_eq!(pv.line(), &*pv.line);
    }

    #[proptest]
    fn negation_changes_score(pv: Pv<3>) {
        assert_eq!(pv.clone().neg().score(), -pv.score());
    }

    #[proptest]
    fn negation_preserves_line(pv: Pv<3>) {
        assert_eq!(pv.clone().neg().line(), pv.line());
    }

    #[proptest]
    fn pv_with_larger_score_is_larger(p: Pv<3>, #[filter(#p.score() != #q.score())] q: Pv<3>) {
        assert_eq!(p < q, p.score() < q.score());
    }
}

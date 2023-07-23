use crate::search::{Depth, DepthBounds, Line, Ply, Score};
use crate::{chess::Move, util::Bounds};
use derive_more::{Deref, IntoIterator};
use std::{cmp::Ordering, iter::once, mem, ops::Neg};
use test_strategy::Arbitrary;

/// The [principal variation].
///
/// [principal variation]: https://www.chessprogramming.org/Principal_Variation
#[derive(Debug, Clone, Eq, PartialEq, Arbitrary, Deref, IntoIterator)]
pub struct Pv<const N: usize = { DepthBounds::UPPER as _ }> {
    score: Score,
    depth: Depth,
    #[filter(#ply >= 0)]
    ply: Ply,
    #[deref]
    #[into_iterator(owned, ref, ref_mut)]
    line: Line<N>,
}

impl<const N: usize> Pv<N> {
    /// Constructs a pv.
    pub fn new<I>(score: Score, depth: Depth, ply: Ply, line: I) -> Self
    where
        I: IntoIterator<Item = Move>,
    {
        Pv {
            score,
            depth,
            ply,
            line: line.into_iter().collect(),
        }
    }

    /// Constructs a pv leaf.
    pub fn leaf(score: Score, depth: Depth, ply: Ply) -> Self {
        Self::new(score, depth, ply, [])
    }

    /// Constructs a drawn pv leaf.
    pub fn drawn(depth: Depth, ply: Ply) -> Self {
        Self::leaf(Score::new(0), depth, ply)
    }

    /// Constructs a lost pv leaf.
    pub fn lost(depth: Depth, ply: Ply) -> Self {
        Self::leaf(Score::LOWER.normalize(ply), depth, ply)
    }

    /// The score from the point of view of the side to move.
    pub fn score(&self) -> Score {
        self.score
    }

    /// The depth searched.
    pub fn depth(&self) -> Depth {
        self.depth
    }

    /// The ply reached.
    pub fn ply(&self) -> Ply {
        if self.ply < 0 {
            -self.ply
        } else {
            self.ply
        }
    }

    /// The tempo bonus from the point of view of the side to move.
    pub fn tempo(&self) -> Ply {
        if self.ply < 0 {
            -(self.ply + self.depth)
        } else {
            -(self.ply - self.depth)
        }
    }

    /// The strongest [`Line`].
    pub fn line(&self) -> &Line<N> {
        &self.line
    }

    /// Continues the [`Line`] from the given [`Move`].
    pub fn shift(mut self, m: Move) -> Pv<N> {
        let tail = mem::take(&mut self.line);
        self.line.extend(once(m).chain(tail));
        self
    }
}

impl<const N: usize> Ord for Pv<N> {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.score(), self.tempo()).cmp(&(other.score(), other.tempo()))
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
        Pv::new(-self.score, self.depth, -self.ply, self.line)
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
    fn depth_returns_depth(pv: Pv<3>) {
        assert_eq!(pv.depth(), pv.depth);
    }

    #[proptest]
    fn ply_returns_ply(pv: Pv<3>) {
        assert_eq!(pv.ply().get(), pv.ply.get().abs());
    }

    #[proptest]
    fn line_returns_line(pv: Pv<3>) {
        assert_eq!(pv.line(), &pv.line);
    }

    #[proptest]
    fn negation_changes_score(pv: Pv<3>) {
        assert_eq!(pv.clone().neg().score(), -pv.score());
    }

    #[proptest]
    fn negation_changes_tempo(#[filter(#pv.ply() > 0)] pv: Pv<3>) {
        assert_eq!(pv.clone().neg().tempo(), -pv.tempo());
    }

    #[proptest]
    fn negation_preserves_depth(pv: Pv<3>) {
        assert_eq!(pv.clone().neg().depth(), pv.depth());
    }

    #[proptest]
    fn negation_preserves_ply(pv: Pv<3>) {
        assert_eq!(pv.clone().neg().ply(), pv.ply());
    }

    #[proptest]
    fn negation_preserves_line(pv: Pv<3>) {
        assert_eq!(pv.clone().neg().line(), pv.line());
    }

    #[proptest]
    fn shift_prepends_move(#[filter(#pv.len() < 3)] pv: Pv<3>, m: Move) {
        assert_eq!(pv.clone().shift(m)[..], [[m].as_slice(), &pv[..]].concat());
    }

    #[proptest]
    fn shift_truncates_line_on_overflow(#[filter(#pv.len() == 3)] pv: Pv<3>, m: Move) {
        assert_eq!(pv.clone().shift(m)[..], [[m].as_slice(), &pv[..2]].concat());
    }

    #[proptest]
    fn pv_with_larger_score_is_larger(p: Pv<3>, #[filter(#p.score() != #q.score())] q: Pv<3>) {
        assert_eq!(p < q, p.score() < q.score());
    }

    #[proptest]
    fn pvs_with_same_score_are_compared_by_tempo(
        s: Score,
        dp: Depth,
        dq: Depth,
        pp: Ply,
        pq: Ply,
        lp: Line<3>,
        lq: Line<3>,
    ) {
        let p = Pv::<3>::new(s, dp, pp, lp);
        let q = Pv::<3>::new(s, dq, pq, lq);
        assert_eq!(p < q, p.tempo() < q.tempo());
    }
}

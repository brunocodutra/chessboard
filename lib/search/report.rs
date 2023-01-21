use super::{Depth, Pv, Score};
use derive_more::Constructor;
use test_strategy::Arbitrary;

/// The search result.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Arbitrary, Constructor)]
pub struct Report {
    depth: Depth,
    #[map(|s: Score| match s.mate() {
        Some(p) if p > 0 => Score::upper().normalize(p / 2 * 2 + 1),
        Some(p) => -Score::upper().normalize(p / 2 * 2),
        None => s
    })]
    score: Score,
    pv: Pv,
}

impl Report {
    /// The depth searched.
    #[inline]
    pub fn depth(&self) -> Depth {
        self.depth
    }

    /// The score from the point of view of the side to move.
    #[inline]
    pub fn score(&self) -> Score {
        self.score
    }

    /// The best line .
    #[inline]
    pub fn pv(&self) -> &Pv {
        &self.pv
    }
}

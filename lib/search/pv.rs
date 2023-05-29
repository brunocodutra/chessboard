use super::{Depth, Line, Score};
use derive_more::{Constructor, Deref};
use test_strategy::Arbitrary;

/// The [principal variation].
///
/// [principal variation]: https://www.chessprogramming.org/Principal_Variation
#[derive(Debug, Clone, Eq, PartialEq, Hash, Arbitrary, Constructor, Deref)]
pub struct Pv {
    depth: Depth,
    #[map(|s: Score| match s.mate() {
        Some(p) if p > 0 => Score::upper().normalize(p / 2 * 2 + 1),
        Some(p) => -Score::upper().normalize(p / 2 * 2),
        None => s
    })]
    score: Score,
    #[deref]
    line: Line,
}

impl Pv {
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

    /// The strongest [`Line`].
    #[inline]
    pub fn line(&self) -> &Line {
        &self.line
    }
}

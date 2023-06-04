use super::{Depth, Line, PlyBounds, Score};
use crate::util::Bounds;
use derive_more::{Constructor, Deref};
use test_strategy::Arbitrary;

/// The [principal variation].
///
/// [principal variation]: https://www.chessprogramming.org/Principal_Variation
#[derive(Debug, Clone, Eq, PartialEq, Hash, Arbitrary, Constructor, Deref)]
pub struct Pv<const N: usize = { PlyBounds::UPPER as _ }> {
    depth: Depth,
    score: Score,
    #[deref]
    line: Line<N>,
}

impl<const N: usize> Pv<N> {
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
    pub fn line(&self) -> &Line<N> {
        &self.line
    }
}

use super::{Depth, Pv};
use crate::eval::Value;
use derive_more::Constructor;
use test_strategy::Arbitrary;

/// The result of an  .
#[derive(Debug, Clone, Eq, PartialEq, Hash, Arbitrary, Constructor)]
pub struct Report {
    depth: Depth,
    score: Value,
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
    pub fn score(&self) -> Value {
        self.score
    }

    /// The best line .
    #[inline]
    pub fn pv(&self) -> &Pv {
        &self.pv
    }
}

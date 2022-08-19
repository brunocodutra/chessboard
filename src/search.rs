use crate::{Position, Pv, SearchLimits};

/// Trait for types that implement adversarial search algorithms.
#[cfg_attr(test, mockall::automock)]
pub trait Search {
    /// Searches for the strongest [variation][`Pv`].
    fn search(&mut self, pos: &Position, limits: SearchLimits) -> Pv<'_>;

    /// Clear the transposition table.
    fn clear(&mut self);
}

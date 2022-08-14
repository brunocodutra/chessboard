use crate::{Position, Pv, SearchLimits};

/// Trait for types that implement adversarial search algorithms.
pub trait Search {
    /// The currently configured search limits.
    fn limits(&self) -> SearchLimits;

    /// Searches for the strongest [variation][`Pv`].
    fn search(&mut self, pos: &Position) -> Pv<'_>;
}

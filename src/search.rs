use crate::{Position, Pv};

/// Trait for types that implement adversarial search algorithms.
pub trait Search {
    /// Searches for the strongest [variation][`Pv`].
    fn search(&self, pos: &Position) -> Pv<'_>;
}

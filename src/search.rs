use crate::{Move, Position};

/// Trait for types that implement adversarial search algorithms.
#[cfg_attr(test, mockall::automock)]
pub trait Search {
    /// Searches for the strongest [`Move`], if one exists.
    fn search(&self, pos: &Position) -> Option<Move>;
}

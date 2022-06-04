use crate::{Move, Position};

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SearchControl {
    pub max_depth: Option<u32>,
}

/// Trait for types that implement adversarial search algorithms.
#[cfg_attr(test, mockall::automock)]
pub trait Search {
    /// Searches for the strongest [`Move`] in this [`Position`], if one exists.
    ///
    /// Implementors are expected to respect the limits specified in `ctrl`.
    fn search(&self, pos: &Position, ctrl: SearchControl) -> Option<Move>;
}

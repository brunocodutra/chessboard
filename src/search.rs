use crate::{Action, Game};

/// Trait for types that implement adversarial search algorithms.
#[cfg_attr(test, mockall::automock)]
pub trait Search {
    /// Searches for the strongest [`Action`], if one exists.
    fn search(&self, game: &Game) -> Option<Action>;
}

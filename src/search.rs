use crate::{Action, Game};

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SearchControl {
    pub depth: Option<u8>,
}

/// Trait for types that implement adversarial search algorithms.
#[cfg_attr(test, mockall::automock)]
pub trait Search {
    /// Searches for the strongest [`Action`], if one exists.
    ///
    /// Implementors are expected to respect the limits specified in `ctrl`.
    fn search(&self, game: &Game, ctrl: SearchControl) -> Option<Action>;
}

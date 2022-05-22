use crate::{EngineDispatcher, Move, Position};
use derive_more::{DebugCustom, From};

mod negamax;

pub use negamax::Negamax;

/// Trait for types that implement adversarial search algorithms.
#[cfg_attr(test, mockall::automock)]
pub trait Search {
    /// Searches for the strongest [`Move`] in this [`Position`], if one exists.
    fn search(&mut self, pos: &Position) -> Option<Move>;
}

/// A static dispatcher for [`Search`].
#[derive(DebugCustom, From)]
pub enum Dispatcher {
    #[debug(fmt = "{:?}", _0)]
    Negamax(Negamax<EngineDispatcher>),
}

impl Search for Dispatcher {
    fn search(&mut self, pos: &Position) -> Option<Move> {
        use Dispatcher::*;
        match self {
            Negamax(s) => s.search(pos),
        }
    }
}

use crate::Position;
use derive_more::{DebugCustom, From};

mod random;

pub use random::Random;

/// Trait for types that implement adversarial search algorithms.
#[cfg_attr(test, mockall::automock)]
pub trait Engine {
    /// Evaluates a position.
    ///
    /// Positive values favor the current side to play.
    fn evaluate(&self, pos: &Position) -> i32;
}

/// A static dispatcher for [`Engine`].
#[derive(DebugCustom, Clone, From)]
pub enum Dispatcher {
    #[debug(fmt = "{:?}", _0)]
    Random(Random),
}

impl Engine for Dispatcher {
    fn evaluate(&self, pos: &Position) -> i32 {
        use Dispatcher::*;
        match self {
            Random(e) => e.evaluate(pos),
        }
    }
}

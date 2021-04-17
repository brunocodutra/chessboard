use crate::Position;
use derive_more::{DebugCustom, From};
use tracing::instrument;

mod random;

pub use random::Random;

/// Trait for types that implement adversarial search algorithms.
#[cfg_attr(test, mockall::automock)]
pub trait Engine {
    /// Evaluates a position.
    ///
    /// Positive values favor [`Color::White`][crate::Color::White],
    /// while negative values favor [`Color::Black`][crate::Color::Black].
    fn evaluate(&self, pos: &Position) -> i32;
}

#[cfg(test)]
impl std::fmt::Debug for MockEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MockEngine")
    }
}

/// A static dispatcher for [`Engine`].
#[derive(DebugCustom, From)]
pub enum EngineDispatcher {
    #[debug(fmt = "{:?}", _0)]
    Random(Random),
}

impl Engine for EngineDispatcher {
    #[instrument(level = "trace")]
    fn evaluate(&self, pos: &Position) -> i32 {
        use EngineDispatcher::*;
        match self {
            Random(e) => e.evaluate(pos),
        }
    }
}

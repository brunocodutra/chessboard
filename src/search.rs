use crate::{EngineDispatcher, Move, Position};
use async_trait::async_trait;
use derive_more::{DebugCustom, From};
use tracing::instrument;

mod negamax;

pub use negamax::Negamax;

/// Trait for types that implement adversarial search algorithms.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Search {
    /// Searches for the strongest [`Move`] in this [`Position`], if one exists.
    async fn search(&mut self, pos: &Position) -> Option<Move>;
}

/// A static dispatcher for [`Search`].
#[derive(DebugCustom, From)]
pub enum SearchDispatcher {
    #[debug(fmt = "{:?}", _0)]
    Negamax(Negamax<EngineDispatcher>),
}

#[async_trait]
impl Search for SearchDispatcher {
    #[instrument(level = "trace")]
    async fn search(&mut self, pos: &Position) -> Option<Move> {
        use SearchDispatcher::*;
        match self {
            Negamax(s) => s.search(pos).await,
        }
    }
}

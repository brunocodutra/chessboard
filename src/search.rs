use crate::{Move, Position};
use async_trait::async_trait;
use derive_more::{DebugCustom, From};
use tracing::instrument;

mod random;

pub use random::Random;

/// Trait for types that implement adversarial search algorithms.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Search {
    /// Searches for the strongest [`Move`] in this [`Position`], if one exists.
    async fn search(&mut self, pos: &Position) -> Option<Move>;
}

#[cfg(test)]
impl std::fmt::Debug for MockSearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MockSearch")
    }
}

/// A static dispatcher for [`Search`].
#[derive(DebugCustom, From)]
pub enum SearchDispatcher {
    #[debug(fmt = "{:?}", _0)]
    Random(Random),
}

#[async_trait]
impl Search for SearchDispatcher {
    #[instrument(level = "trace")]
    async fn search(&mut self, pos: &Position) -> Option<Move> {
        use SearchDispatcher::*;
        match self {
            Random(s) => s.search(pos).await,
        }
    }
}

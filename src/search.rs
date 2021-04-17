use crate::{Move, Position};
use async_trait::async_trait;
use derive_more::{DebugCustom, From};

/// Trait for types that implement adversarial search algorithms.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Search {
    /// Searches for the strongest [`Move`] in this [`Position`], if one exists.
    async fn search(&mut self, pos: &Position) -> Option<Move>;
}

/// A static dispatcher for [`Search`].
#[derive(DebugCustom, From)]
pub enum SearchDispatcher {}

#[async_trait]
impl Search for SearchDispatcher {
    async fn search(&mut self, _: &Position) -> Option<Move> {
        None
    }
}

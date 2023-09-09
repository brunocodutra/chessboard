use async_trait::async_trait;
use lib::chess::{Move, Position};
use lib::search::Limits;

/// Trait for types that know how to analyze chess [`Position`]s.
#[async_trait]
pub trait Ai {
    /// Finds the best [`Move`].
    async fn play(&mut self, pos: &Position, limits: Limits) -> Move;
}

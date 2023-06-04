use async_trait::async_trait;
use lib::chess::{Move, Position};
use lib::search::Limits;

/// Trait for types that know how to play chess.
#[async_trait]
#[cfg_attr(test, mockall::automock(type Error = String;))]
pub trait Play {
    /// The reason why a move could not be played.
    type Error;

    /// Finds the best [`Move`].
    async fn play(&mut self, pos: &Position, limits: Limits) -> Result<Move, Self::Error>;
}

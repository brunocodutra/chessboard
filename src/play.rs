use crate::chess::{Move, Position};
use async_trait::async_trait;

/// Trait for types that know how to play chess.
#[cfg_attr(test, mockall::automock(type Error = String;))]
#[async_trait]
pub trait Play {
    /// The reason why a [`Move`] could not be played.
    type Error;

    /// Play the next turn.
    async fn play(&mut self, pos: &Position) -> Result<Move, Self::Error>;
}

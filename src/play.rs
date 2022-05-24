use crate::{Action, Position};
use async_trait::async_trait;

/// Trait for types that know how to play chess.
#[async_trait]
pub trait Play {
    /// The reason why an [`Action`] could not be performed.
    type Error;

    /// Play the next turn.
    async fn play(&mut self, pos: &Position) -> Result<Action, Self::Error>;
}

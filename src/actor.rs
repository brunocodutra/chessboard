use crate::{action::*, position::*};
use async_trait::async_trait;

/// Traits for type that know how to play chess.
#[async_trait]
pub trait Actor {
    /// The reason why acting failed.
    type Error;

    /// Play the next turn.
    async fn act(&mut self, p: Position) -> Result<PlayerAction, Self::Error>;
}

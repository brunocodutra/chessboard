use crate::{Action, Position};
use async_trait::async_trait;

/// Trait for types that know how to play chess.
#[cfg_attr(test, mockall::automock(type Error = String;))]
#[async_trait]
pub trait Act {
    /// The reason why an [`Action`] could not be performed.
    type Error;

    /// Play the next turn.
    async fn act(&mut self, pos: &Position) -> Result<Action, Self::Error>;
}

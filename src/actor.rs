use crate::{PlayerAction, Position};
use async_trait::async_trait;

mod cli;
mod uci;

pub use cli::*;
pub use uci::*;

/// Trait for types that play chess.
#[async_trait]
pub trait Actor {
    /// The reason why acting failed.
    type Error;

    /// Play the next turn.
    async fn act(&mut self, p: Position) -> Result<PlayerAction, Self::Error>;
}

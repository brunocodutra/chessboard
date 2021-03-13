use crate::{Action, Position, Remote};
use async_trait::async_trait;
use derive_more::From;
use std::error::Error;
use tracing::instrument;

mod cli;
mod uci;

pub use cli::*;
pub use uci::*;

/// Trait for types that play chess.
#[async_trait]
pub trait Player {
    /// The reason why acting failed.
    type Error;

    /// Play the next turn.
    async fn act(&mut self, pos: Position) -> Result<Action, Self::Error>;
}

/// A static dispatcher for [`Player`].
#[derive(Debug, From)]
pub enum PlayerDispatcher<R>
where
    R: Remote,
    R::Error: Error + Send + Sync + 'static,
{
    Cli(Cli<R>),
    Uci(Uci<R>),
}

#[async_trait]
impl<R> Player for PlayerDispatcher<R>
where
    R: Remote + Send + Sync,
    R::Error: Error + Send + Sync + 'static,
{
    type Error = R::Error;

    #[instrument(skip(self), err)]
    async fn act(&mut self, pos: Position) -> Result<Action, Self::Error> {
        use PlayerDispatcher::*;
        let action = match self {
            Cli(p) => p.act(pos).await?,
            Uci(p) => p.act(pos).await?,
        };

        Ok(action)
    }
}

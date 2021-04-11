use crate::{Action, Position, Remote};
use async_trait::async_trait;
use derive_more::{DebugCustom, From};
use std::{error::Error, fmt::Debug};
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
    async fn act(&mut self, pos: &Position) -> Result<Action, Self::Error>;
}

/// A static dispatcher for [`Player`].
#[derive(DebugCustom, From)]
pub enum PlayerDispatcher<R>
where
    R: Remote + Debug,
    R::Error: Error + Send + Sync + 'static,
{
    #[debug(fmt = "{:?}", _0)]
    Cli(Cli<R>),
    #[debug(fmt = "{:?}", _0)]
    Uci(Uci<R>),
}

#[async_trait]
impl<R> Player for PlayerDispatcher<R>
where
    R: Remote + Debug + Send,
    R::Error: Error + Send + Sync + 'static,
{
    type Error = R::Error;

    #[instrument(level = "trace", err)]
    async fn act(&mut self, pos: &Position) -> Result<Action, Self::Error> {
        use PlayerDispatcher::*;
        let action = match self {
            Cli(p) => p.act(pos).await?,
            Uci(p) => p.act(pos).await?,
        };

        Ok(action)
    }
}

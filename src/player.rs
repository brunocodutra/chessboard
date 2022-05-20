use crate::{Action, IoDispatcher, Position, SearchDispatcher};
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use std::fmt::Debug;
use tracing::instrument;

mod ai;
mod cli;
mod uci;

pub use ai::*;
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

/// The reason why the underlying [`Player`] failed.
#[derive(Debug, Display, Error)]
pub enum PlayerDispatcherError {
    Ai(<Ai<SearchDispatcher> as Player>::Error),
    Cli(<Cli<IoDispatcher> as Player>::Error),
    Uci(<Uci<IoDispatcher> as Player>::Error),
}

/// A static dispatcher for [`Player`].
#[derive(DebugCustom, From)]
pub enum PlayerDispatcher {
    #[debug(fmt = "{:?}", _0)]
    Ai(Ai<SearchDispatcher>),
    #[debug(fmt = "{:?}", _0)]
    Cli(Cli<IoDispatcher>),
    #[debug(fmt = "{:?}", _0)]
    Uci(Uci<IoDispatcher>),
}

#[async_trait]
impl Player for PlayerDispatcher {
    type Error = PlayerDispatcherError;

    #[instrument(level = "trace", err)]
    async fn act(&mut self, pos: &Position) -> Result<Action, Self::Error> {
        use PlayerDispatcher::*;
        match self {
            Ai(p) => p.act(pos).await.map_err(PlayerDispatcherError::Ai),
            Cli(p) => p.act(pos).await.map_err(PlayerDispatcherError::Cli),
            Uci(p) => p.act(pos).await.map_err(PlayerDispatcherError::Uci),
        }
    }
}

use crate::{PlayerAction, Position, Remote};
use async_trait::async_trait;

mod cli;
mod uci;

pub use cli::*;
use std::error::Error;
pub use uci::*;

/// Trait for types that play chess.
#[async_trait]
pub trait Player {
    /// The reason why acting failed.
    type Error;

    /// Play the next turn.
    async fn play(&mut self, pos: Position) -> Result<PlayerAction, Self::Error>;
}

pub enum PlayerDispatcher<R>
where
    R: Remote,
    R::Error: Error + Send + Sync + 'static,
{
    Cli(Cli<R>),
    Uci(Uci<R>),
}

impl<R> From<Cli<R>> for PlayerDispatcher<R>
where
    R: Remote,
    R::Error: Error + Send + Sync + 'static,
{
    fn from(cli: Cli<R>) -> Self {
        PlayerDispatcher::Cli(cli)
    }
}

impl<R> From<Uci<R>> for PlayerDispatcher<R>
where
    R: Remote,
    R::Error: Error + Send + Sync + 'static,
{
    fn from(uci: Uci<R>) -> Self {
        PlayerDispatcher::Uci(uci)
    }
}

#[async_trait]
impl<R> Player for PlayerDispatcher<R>
where
    R: Remote + Send + Sync,
    R::Error: Error + Send + Sync + 'static,
{
    type Error = R::Error;

    async fn play(&mut self, pos: Position) -> Result<PlayerAction, Self::Error> {
        use PlayerDispatcher::*;
        let action = match self {
            Cli(p) => p.play(pos).await?,
            Uci(p) => p.play(pos).await?,
        };

        Ok(action)
    }
}

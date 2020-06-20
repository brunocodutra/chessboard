use crate::{PlayerAction, Position, Remote};
use async_trait::async_trait;

mod cli;
mod uci;

pub use cli::*;
use std::error::Error;
pub use uci::*;

/// Trait for types that play chess.
#[async_trait]
pub trait Actor {
    /// The reason why acting failed.
    type Error;

    /// Play the next turn.
    async fn act(&mut self, p: Position) -> Result<PlayerAction, Self::Error>;
}

pub enum ActorDispatcher<R>
where
    R: Remote,
    R::Error: Error + Send + Sync + 'static,
{
    Cli(Cli<R>),
    Uci(Uci<R>),
}

impl<R> From<Cli<R>> for ActorDispatcher<R>
where
    R: Remote,
    R::Error: Error + Send + Sync + 'static,
{
    fn from(cli: Cli<R>) -> Self {
        ActorDispatcher::Cli(cli)
    }
}

impl<R> From<Uci<R>> for ActorDispatcher<R>
where
    R: Remote,
    R::Error: Error + Send + Sync + 'static,
{
    fn from(uci: Uci<R>) -> Self {
        ActorDispatcher::Uci(uci)
    }
}

#[async_trait]
impl<R> Actor for ActorDispatcher<R>
where
    R: Remote + Send + Sync,
    R::Error: Error + Send + Sync + 'static,
{
    type Error = R::Error;

    async fn act(&mut self, p: Position) -> Result<PlayerAction, Self::Error> {
        use ActorDispatcher::*;
        let action = match self {
            Cli(a) => a.act(p).await?,
            Uci(a) => a.act(p).await?,
        };

        Ok(action)
    }
}

use crate::{Action, IoConfig, IoDispatcher, Position, SearchConfig, SearchDispatcher, Setup};
use anyhow::Error as Anyhow;
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use serde::Deserialize;
use std::{fmt::Debug, str::FromStr};
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
#[derive(Debug, Display, Error, From)]
pub enum DispatcherError {
    Ai(<Ai<SearchDispatcher> as Player>::Error),
    Cli(<Cli<IoDispatcher> as Player>::Error),
    Uci(<Uci<IoDispatcher> as Player>::Error),
}

/// A static dispatcher for [`Player`].
#[derive(DebugCustom, From)]
pub enum Dispatcher {
    #[debug(fmt = "{:?}", _0)]
    Ai(Ai<SearchDispatcher>),
    #[debug(fmt = "{:?}", _0)]
    Cli(Cli<IoDispatcher>),
    #[debug(fmt = "{:?}", _0)]
    Uci(Uci<IoDispatcher>),
}

#[async_trait]
impl Player for Dispatcher {
    type Error = DispatcherError;

    async fn act(&mut self, pos: &Position) -> Result<Action, Self::Error> {
        match self {
            Dispatcher::Ai(p) => Ok(p.act(pos).await?),
            Dispatcher::Cli(p) => Ok(p.act(pos).await?),
            Dispatcher::Uci(p) => Ok(p.act(pos).await?),
        }
    }
}

/// Runtime configuration for an [`Player`].
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum Config {
    Ai(SearchConfig),
    Cli(IoConfig),
    Uci(IoConfig),
}

/// The reason why parsing [`Config`] failed.
#[derive(Debug, Display, PartialEq, Error, From)]
#[display(fmt = "failed to parse player configuration")]
pub struct ParseConfigError(ron::de::Error);

impl FromStr for Config {
    type Err = ParseConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

#[async_trait]
impl Setup for Config {
    type Output = Dispatcher;

    #[instrument(level = "trace", err)]
    async fn setup(self) -> Result<Self::Output, Anyhow> {
        match self {
            Config::Ai(cfg) => Ok(Ai::new(cfg.setup().await?).into()),
            Config::Cli(cfg) => Ok(Cli::new(cfg.setup().await?).into()),
            Config::Uci(cfg) => Ok(Uci::init(cfg.setup().await?).await?.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{io::MockIo, search::MockSearch};
    use std::mem::discriminant;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn player_config_is_deserializable() {
        assert_eq!("ai(mock())".parse(), Ok(Config::Ai(SearchConfig::Mock())));
        assert_eq!("cli(mock())".parse(), Ok(Config::Cli(IoConfig::Mock())));
        assert_eq!("uci(mock())".parse(), Ok(Config::Uci(IoConfig::Mock())));
    }

    #[proptest]
    fn player_can_be_configured_at_runtime() {
        let rt = runtime::Builder::new_multi_thread().build()?;

        assert_eq!(
            discriminant(&Dispatcher::Ai(Ai::new(SearchDispatcher::Mock(
                MockSearch::new()
            )))),
            discriminant(
                &rt.block_on(Config::Ai(SearchConfig::Mock()).setup())
                    .unwrap()
            )
        );

        assert_eq!(
            discriminant(&Dispatcher::Cli(Cli::new(
                IoDispatcher::Mock(MockIo::new())
            ))),
            discriminant(&rt.block_on(Config::Cli(IoConfig::Mock()).setup()).unwrap())
        );
    }
}

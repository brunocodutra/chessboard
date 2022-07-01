use crate::io::{Process, Terminal};
use crate::{Act, Action, Game, Setup, Strategy, StrategyConfig};
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

/// The reason why [`Player`] failed to perform an action.
#[derive(Debug, Display, Error, From)]
pub enum PlayerError {
    Ai(<Ai<Strategy> as Act>::Error),
    Cli(<Cli<Terminal> as Act>::Error),
    Uci(<Uci<Process> as Act>::Error),
}

/// A generic player.
#[derive(DebugCustom, From)]
#[allow(clippy::large_enum_variant)]
pub enum Player {
    #[debug(fmt = "{:?}", _0)]
    Ai(Ai<Strategy>),
    #[debug(fmt = "{:?}", _0)]
    Cli(Cli<Terminal>),
    #[debug(fmt = "{:?}", _0)]
    Uci(Uci<Process>),
}

#[async_trait]
impl Act for Player {
    type Error = PlayerError;

    async fn act(&mut self, game: &Game) -> Result<Action, Self::Error> {
        match self {
            Player::Ai(p) => Ok(p.act(game).await?),
            Player::Cli(p) => Ok(p.act(game).await?),
            Player::Uci(p) => Ok(p.act(game).await?),
        }
    }
}

/// Runtime configuration for an [`Player`].
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum PlayerConfig {
    Ai(StrategyConfig),
    Uci(String),
    Cli(),
}

/// The reason why parsing [`PlayerConfig`] failed.
#[derive(Debug, Display, PartialEq, Error, From)]
#[display(fmt = "failed to parse player configuration")]
pub struct ParseConfigError(ron::de::Error);

impl FromStr for PlayerConfig {
    type Err = ParseConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

#[async_trait]
impl Setup for PlayerConfig {
    type Output = Player;

    #[instrument(level = "trace", err, ret)]
    async fn setup(self) -> Result<Self::Output, Anyhow> {
        match self {
            PlayerConfig::Ai(cfg) => Ok(Ai::new(cfg.setup().await?).into()),
            PlayerConfig::Uci(path) => Ok(Uci::new(Process::spawn(&path)?).into()),
            PlayerConfig::Cli() => Ok(Cli::new(Terminal::open()).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn ai_config_is_deserializable() {
        assert_eq!(
            "ai(mock())".parse(),
            Ok(PlayerConfig::Ai(StrategyConfig::Mock()))
        );
    }

    #[proptest]
    fn cli_config_is_deserializable() {
        assert_eq!("cli()".parse(), Ok(PlayerConfig::Cli()));
    }

    #[proptest]
    fn uci_config_is_deserializable(s: String) {
        assert_eq!(format!("uci({:?})", s).parse(), Ok(PlayerConfig::Uci(s)));
    }

    #[proptest]
    fn ai_can_be_configured_at_runtime() {
        let rt = runtime::Builder::new_multi_thread().build()?;

        assert!(matches!(
            rt.block_on(PlayerConfig::Ai(StrategyConfig::Mock()).setup()),
            Ok(Player::Ai(_))
        ));
    }

    #[proptest]
    fn uci_can_be_configured_at_runtime(s: String) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        assert!(matches!(
            rt.block_on(PlayerConfig::Uci(s).setup()),
            Ok(Player::Uci(_))
        ));
    }

    #[proptest]
    fn cli_can_be_configured_at_runtime() {
        let rt = runtime::Builder::new_multi_thread().build()?;

        assert!(matches!(
            rt.block_on(PlayerConfig::Cli().setup()),
            Ok(Player::Cli(_))
        ));
    }
}

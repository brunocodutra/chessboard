use crate::io::{Process, Terminal};
use crate::{Act, Action, Build, Game, Strategy, StrategyBuilder};
use anyhow::Error as Anyhow;
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use serde::Deserialize;
use std::{fmt::Debug, str::FromStr};

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

    #[inline]
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
pub enum PlayerBuilder {
    Ai(StrategyBuilder),
    Uci(String),
    Cli(),
}

/// The reason why parsing [`PlayerBuilder`] failed.
#[derive(Debug, Display, PartialEq, Error, From)]
#[display(fmt = "failed to parse player configuration")]
pub struct ParseBuilderError(ron::de::Error);

impl FromStr for PlayerBuilder {
    type Err = ParseBuilderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

impl Build for PlayerBuilder {
    type Output = Player;

    fn build(self) -> Result<Self::Output, Anyhow> {
        match self {
            PlayerBuilder::Ai(strategy) => Ok(Ai::new(strategy.build()?).into()),
            PlayerBuilder::Uci(path) => Ok(Uci::new(Process::spawn(&path)?).into()),
            PlayerBuilder::Cli() => Ok(Cli::new(Terminal::new()).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn ai_builder_is_deserializable() {
        assert_eq!(
            "ai(mock())".parse(),
            Ok(PlayerBuilder::Ai(StrategyBuilder::Mock()))
        );
    }

    #[proptest]
    fn cli_builder_is_deserializable() {
        assert_eq!("cli()".parse(), Ok(PlayerBuilder::Cli()));
    }

    #[proptest]
    fn uci_builder_is_deserializable(s: String) {
        assert_eq!(format!("uci({:?})", s).parse(), Ok(PlayerBuilder::Uci(s)));
    }

    #[proptest]
    fn ai_can_be_configured_at_runtime() {
        assert!(matches!(
            PlayerBuilder::Ai(StrategyBuilder::Mock()).build(),
            Ok(Player::Ai(_))
        ));
    }

    #[proptest]
    fn uci_can_be_configured_at_runtime(s: String) {
        assert!(matches!(PlayerBuilder::Uci(s).build(), Ok(Player::Uci(_))));
    }

    #[proptest]
    fn cli_can_be_configured_at_runtime() {
        assert!(matches!(PlayerBuilder::Cli().build(), Ok(Player::Cli(_))));
    }
}

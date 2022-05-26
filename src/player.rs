use crate::{Action, Play, Position, Remote, RemoteConfig, Setup, Strategy, StrategyConfig};
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
    Ai(<Ai<Strategy> as Play>::Error),
    Cli(<Cli<Remote> as Play>::Error),
    Uci(<Uci<Remote> as Play>::Error),
    #[cfg(test)]
    Mock(#[error(not(source))] <crate::MockPlay as Play>::Error),
}

/// A generic player.
#[derive(DebugCustom, From)]
pub enum Player {
    #[debug(fmt = "{:?}", _0)]
    Ai(Ai<Strategy>),
    #[debug(fmt = "{:?}", _0)]
    Cli(Cli<Remote>),
    #[debug(fmt = "{:?}", _0)]
    Uci(Uci<Remote>),
    #[cfg(test)]
    Mock(crate::MockPlay),
}

#[async_trait]
impl Play for Player {
    type Error = PlayerError;

    async fn play(&mut self, pos: &Position) -> Result<Action, Self::Error> {
        match self {
            Player::Ai(p) => Ok(p.play(pos).await?),
            Player::Cli(p) => Ok(p.play(pos).await?),
            Player::Uci(p) => Ok(p.play(pos).await?),
            #[cfg(test)]
            Player::Mock(p) => Ok(p.play(pos).await?),
        }
    }
}

/// Runtime configuration for an [`Player`].
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum PlayerConfig {
    Ai(StrategyConfig),
    Cli(RemoteConfig),
    Uci(RemoteConfig),
    #[cfg(test)]
    Mock(),
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
            PlayerConfig::Cli(cfg) => Ok(Cli::new(cfg.setup().await?).into()),
            PlayerConfig::Uci(cfg) => Ok(Uci::init(cfg.setup().await?).await?.into()),
            #[cfg(test)]
            PlayerConfig::Mock() => Ok(crate::MockPlay::new().into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockIo, MockPlay, MockSearch};
    use std::mem::discriminant;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn player_config_is_deserializable() {
        assert_eq!(
            "ai(mock())".parse(),
            Ok(PlayerConfig::Ai(StrategyConfig::Mock()))
        );
        assert_eq!(
            "cli(mock())".parse(),
            Ok(PlayerConfig::Cli(RemoteConfig::Mock()))
        );
        assert_eq!(
            "uci(mock())".parse(),
            Ok(PlayerConfig::Uci(RemoteConfig::Mock()))
        );
        assert_eq!("mock()".parse(), Ok(PlayerConfig::Mock()));
    }

    #[proptest]
    fn player_can_be_configured_at_runtime() {
        let rt = runtime::Builder::new_multi_thread().build()?;

        assert_eq!(
            discriminant(&Player::Ai(Ai::new(Strategy::Mock(MockSearch::new())))),
            discriminant(
                &rt.block_on(PlayerConfig::Ai(StrategyConfig::Mock()).setup())
                    .unwrap()
            )
        );

        assert_eq!(
            discriminant(&Player::Cli(Cli::new(Remote::Mock(MockIo::new())))),
            discriminant(
                &rt.block_on(PlayerConfig::Cli(RemoteConfig::Mock()).setup())
                    .unwrap()
            )
        );

        assert_eq!(
            discriminant(&Player::Mock(MockPlay::new())),
            discriminant(&rt.block_on(PlayerConfig::Mock().setup()).unwrap())
        );
    }
}

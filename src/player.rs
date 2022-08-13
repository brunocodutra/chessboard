use crate::io::{Process, Terminal};
use crate::{Act, Action, Build, Game, Strategy, StrategyBuilder};
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
#[derive(Debug, Display, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum PlayerBuilder {
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Ai(StrategyBuilder),
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Uci(String, #[serde(default)] UciConfig),
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
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
    type Error = PlayerError;

    fn build(self) -> Result<Self::Output, Self::Error> {
        match self {
            PlayerBuilder::Ai(strategy) => {
                let strategy = strategy.build()?;
                Ok(Ai::new(strategy).into())
            }

            PlayerBuilder::Uci(path, cfg) => {
                let io = Process::spawn(&path).map_err(UciError::from)?;
                Ok(Uci::with_config(io, cfg).into())
            }

            PlayerBuilder::Cli() => {
                let io = Terminal::open().map_err(CliError::from)?;
                Ok(Cli::new(io).into())
            }
        }
    }
}

#[cfg(test)]
mockall::mock! {
    pub PlayerBuilder {}
    impl Build for PlayerBuilder {
        type Output = crate::MockAct;
        type Error = String;
        fn build(self) -> Result<crate::MockAct, String>;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn parsing_printed_player_builder_is_an_identity(b: PlayerBuilder) {
        assert_eq!(b.to_string().parse(), Ok(b));
    }

    #[proptest]
    fn ai_builder_is_deserializable(s: StrategyBuilder) {
        assert_eq!(
            format!("ai({})", ron::ser::to_string(&s)?).parse(),
            Ok(PlayerBuilder::Ai(s))
        );
    }

    #[proptest]
    fn uci_builder_is_deserializable(s: String, c: UciConfig) {
        assert_eq!(
            format!("uci({:?})", s).parse(),
            Ok(PlayerBuilder::Uci(s.clone(), UciConfig::default()))
        );

        assert_eq!(
            format!("uci({:?}, {})", s, c).parse(),
            Ok(PlayerBuilder::Uci(s, c))
        );
    }

    #[proptest]
    fn cli_builder_is_deserializable() {
        assert_eq!("cli()".parse(), Ok(PlayerBuilder::Cli()));
    }

    #[proptest]
    fn ai_can_be_configured_at_runtime(s: StrategyBuilder) {
        assert!(matches!(PlayerBuilder::Ai(s).build(), Ok(Player::Ai(_))));
    }

    #[proptest]
    fn uci_can_be_configured_at_runtime(s: String, c: UciConfig) {
        assert!(matches!(
            PlayerBuilder::Uci(s, c).build(),
            Ok(Player::Uci(_))
        ));
    }

    #[proptest]
    fn cli_can_be_configured_at_runtime() {
        assert!(matches!(PlayerBuilder::Cli().build(), Ok(Player::Cli(_))));
    }
}

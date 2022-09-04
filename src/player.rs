use crate::chess::{Move, Position};
use crate::{util::io::Process, Build, Play, SearchLimits, Strategy, StrategyBuilder};
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

mod ai;
mod uci;

pub use ai::*;
pub use uci::*;

/// The reason why [`Player`] failed to [`Play`].
#[derive(Debug, Display, Error, From)]
pub enum PlayerError {
    Ai(<Ai<Strategy> as Play>::Error),
    Uci(<Uci<Process> as Play>::Error),
}

/// A generic player.
#[derive(DebugCustom, From)]
#[allow(clippy::large_enum_variant)]
pub enum Player {
    #[debug(fmt = "{:?}", _0)]
    Ai(Ai<Strategy>),
    #[debug(fmt = "{:?}", _0)]
    Uci(Uci<Process>),
}

#[async_trait]
impl Play for Player {
    type Error = PlayerError;

    async fn play(&mut self, pos: &Position) -> Result<Move, Self::Error> {
        match self {
            Player::Ai(p) => Ok(p.play(pos).await?),
            Player::Uci(p) => Ok(p.play(pos).await?),
        }
    }
}

/// Runtime configuration for an [`Player`].
#[derive(Debug, Display, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum PlayerBuilder {
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Ai(StrategyBuilder, #[serde(default)] SearchLimits),
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Uci(
        String,
        #[serde(default)] SearchLimits,
        #[serde(default)] HashMap<String, Option<String>>,
    ),
}

/// The reason why parsing [`PlayerBuilder`] failed.
#[derive(Debug, Display, Eq, PartialEq, Error, From)]
#[display(fmt = "failed to parse player configuration")]
pub struct ParseBuilderError(ron::de::SpannedError);

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
            PlayerBuilder::Ai(strategy, limits) => {
                let strategy = strategy.build()?;
                Ok(Ai::with_config(strategy, limits).into())
            }

            PlayerBuilder::Uci(path, limits, options) => {
                let io = Process::spawn(&path).map_err(UciError::from)?;
                Ok(Uci::with_config(io, limits, options).into())
            }
        }
    }
}

#[cfg(test)]
mockall::mock! {
    #[derive(Debug)]
    pub PlayerBuilder {}
    impl Build for PlayerBuilder {
        type Output = crate::MockPlay;
        type Error = String;
        fn build(self) -> Result<crate::MockPlay, String>;
    }
}

#[cfg(test)]
impl std::fmt::Display for MockPlayerBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
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
    fn ai_builder_is_deserializable(s: StrategyBuilder, l: SearchLimits) {
        assert_eq!(
            format!("ai({})", s).parse(),
            Ok(PlayerBuilder::Ai(s.clone(), SearchLimits::default()))
        );

        assert_eq!(
            format!("ai({}, {})", s, l).parse(),
            Ok(PlayerBuilder::Ai(s, l))
        );
    }

    #[proptest]
    fn uci_builder_is_deserializable(s: String, l: SearchLimits, o: UciOptions) {
        assert_eq!(
            format!("uci({:?})", s).parse(),
            Ok(PlayerBuilder::Uci(
                s.clone(),
                SearchLimits::default(),
                UciOptions::default()
            ))
        );

        assert_eq!(
            format!("uci({:?}, {})", s, l).parse(),
            Ok(PlayerBuilder::Uci(s.clone(), l, UciOptions::default()))
        );

        assert_eq!(
            format!("uci({:?}, {}, {})", s, l, ron::ser::to_string(&o)?).parse(),
            Ok(PlayerBuilder::Uci(s, l, o))
        );
    }

    #[proptest]
    fn ai_can_be_configured_at_runtime(s: StrategyBuilder, l: SearchLimits) {
        assert!(matches!(PlayerBuilder::Ai(s, l).build(), Ok(Player::Ai(_))));
    }

    #[proptest]
    fn uci_can_be_configured_at_runtime(s: String, l: SearchLimits, o: UciOptions) {
        assert!(matches!(
            PlayerBuilder::Uci(s, l, o).build(),
            Ok(Player::Uci(_))
        ));
    }
}

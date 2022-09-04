use crate::chess::{Move, Position};
use crate::search::{Builder as StrategyBuilder, Dispatcher as Strategy, Limits};
use crate::{util::io::Process, Build};
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

mod ai;
mod uci;

pub use ai::*;
pub use uci::*;

/// Trait for types that know how to play chess.
#[cfg_attr(test, mockall::automock(type Error = String;))]
#[async_trait]
pub trait Play {
    /// The reason why a [`Move`] could not be played.
    type Error;

    /// Play the next turn.
    async fn play(&mut self, pos: &Position) -> Result<Move, Self::Error>;
}

/// The reason why [`Dispatcher`] failed to play a [`Move`].
#[derive(Debug, Display, Error, From)]
pub enum DispatcherError {
    Ai(<Ai<Strategy> as Play>::Error),
    Uci(<Uci<Process> as Play>::Error),
}

/// A generic player.
#[derive(DebugCustom, From)]
#[allow(clippy::large_enum_variant)]
pub enum Dispatcher {
    #[debug(fmt = "{:?}", _0)]
    Ai(Ai<Strategy>),
    #[debug(fmt = "{:?}", _0)]
    Uci(Uci<Process>),
}

#[async_trait]
impl Play for Dispatcher {
    type Error = DispatcherError;

    async fn play(&mut self, pos: &Position) -> Result<Move, Self::Error> {
        match self {
            Dispatcher::Ai(p) => Ok(p.play(pos).await?),
            Dispatcher::Uci(p) => Ok(p.play(pos).await?),
        }
    }
}

/// Runtime configuration for [`Dispatcher`].
#[derive(Debug, Display, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum Builder {
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Ai(StrategyBuilder, #[serde(default)] Limits),
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Uci(
        String,
        #[serde(default)] Limits,
        #[serde(default)] HashMap<String, Option<String>>,
    ),
}

/// The reason why parsing [`Builder`] failed.
#[derive(Debug, Display, Eq, PartialEq, Error, From)]
#[display(fmt = "failed to parse player configuration")]
pub struct ParseBuilderError(ron::de::SpannedError);

impl FromStr for Builder {
    type Err = ParseBuilderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

impl Build for Builder {
    type Output = Dispatcher;
    type Error = DispatcherError;

    fn build(self) -> Result<Self::Output, Self::Error> {
        match self {
            Builder::Ai(strategy, limits) => {
                let strategy = strategy.build()?;
                Ok(Ai::with_config(strategy, limits).into())
            }

            Builder::Uci(path, limits, options) => {
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
        type Output = MockPlay;
        type Error = String;
        fn build(self) -> Result<MockPlay, String>;
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
    fn parsing_printed_player_builder_is_an_identity(b: Builder) {
        assert_eq!(b.to_string().parse(), Ok(b));
    }

    #[proptest]
    fn ai_builder_is_deserializable(s: StrategyBuilder, l: Limits) {
        assert_eq!(
            format!("ai({})", s).parse(),
            Ok(Builder::Ai(s.clone(), Limits::default()))
        );

        assert_eq!(format!("ai({}, {})", s, l).parse(), Ok(Builder::Ai(s, l)));
    }

    #[proptest]
    fn uci_builder_is_deserializable(s: String, l: Limits, o: UciOptions) {
        assert_eq!(
            format!("uci({:?})", s).parse(),
            Ok(Builder::Uci(
                s.clone(),
                Limits::default(),
                UciOptions::default()
            ))
        );

        assert_eq!(
            format!("uci({:?}, {})", s, l).parse(),
            Ok(Builder::Uci(s.clone(), l, UciOptions::default()))
        );

        assert_eq!(
            format!("uci({:?}, {}, {})", s, l, ron::ser::to_string(&o)?).parse(),
            Ok(Builder::Uci(s, l, o))
        );
    }

    #[proptest]
    fn ai_can_be_configured_at_runtime(s: StrategyBuilder, l: Limits) {
        assert!(matches!(Builder::Ai(s, l).build(), Ok(Dispatcher::Ai(_))));
    }

    #[proptest]
    fn uci_can_be_configured_at_runtime(s: String, l: Limits, o: UciOptions) {
        assert!(matches!(
            Builder::Uci(s, l, o).build(),
            Ok(Dispatcher::Uci(_))
        ));
    }
}

use crate::{Action, Build, Engine, EngineBuilder, Game, Search};
use anyhow::Error as Anyhow;
use derive_more::{DebugCustom, Display, Error, From};
use serde::Deserialize;
use std::str::FromStr;

mod negamax;

pub use negamax::*;

/// A generic adversarial search algorithm.
#[derive(DebugCustom, From)]
pub enum Strategy {
    #[debug(fmt = "{:?}", _0)]
    Negamax(Negamax<Engine>),
    #[cfg(test)]
    #[debug(fmt = "{:?}", _0)]
    Mock(crate::MockSearch),
}

impl Search for Strategy {
    fn search(&self, game: &Game) -> Option<Action> {
        match self {
            Strategy::Negamax(s) => s.search(game),
            #[cfg(test)]
            Strategy::Mock(s) => s.search(game),
        }
    }
}

/// Runtime configuration for [`Search`].
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum StrategyBuilder {
    Negamax(EngineBuilder, #[serde(default)] NegamaxConfig),

    #[cfg(test)]
    Mock(),
}

/// The reason why parsing [`StrategyBuilder`] failed.
#[derive(Debug, Display, PartialEq, Error, From)]
#[display(fmt = "failed to parse search configuration")]
pub struct ParseBuilderError(ron::de::Error);

impl FromStr for StrategyBuilder {
    type Err = ParseBuilderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

impl Build for StrategyBuilder {
    type Output = Strategy;

    fn build(self) -> Result<Self::Output, Anyhow> {
        match self {
            StrategyBuilder::Negamax(engine, config) => {
                Ok(Negamax::with_config(engine.build()?, config).into())
            }

            #[cfg(test)]
            StrategyBuilder::Mock() => Ok(crate::MockSearch::new().into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn negamax_builder_is_deserializable(c: NegamaxConfig) {
        assert_eq!(
            "negamax(mock())".parse(),
            Ok(StrategyBuilder::Negamax(
                EngineBuilder::Mock(),
                NegamaxConfig::default(),
            ))
        );

        assert_eq!(
            format!("negamax(mock(),{})", c).parse(),
            Ok(StrategyBuilder::Negamax(EngineBuilder::Mock(), c))
        );
    }

    #[proptest]
    fn mock_builder_is_deserializable() {
        assert_eq!("mock()".parse(), Ok(StrategyBuilder::Mock()));
    }

    #[proptest]
    fn negamax_can_be_configured_at_runtime(c: NegamaxConfig) {
        assert!(matches!(
            StrategyBuilder::Negamax(EngineBuilder::Mock(), c).build(),
            Ok(Strategy::Negamax(_))
        ));
    }

    #[proptest]
    fn mock_can_be_configured_at_runtime() {
        assert!(matches!(
            StrategyBuilder::Mock().build(),
            Ok(Strategy::Mock(_))
        ));
    }
}

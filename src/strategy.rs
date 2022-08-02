use crate::{Build, Engine, EngineBuilder, Move, Position, Search};
use anyhow::Error as Anyhow;
use derive_more::{DebugCustom, Display, Error, From};
use serde::Deserialize;
use std::str::FromStr;

mod minimax;

pub use minimax::*;

/// A generic adversarial search algorithm.
#[derive(DebugCustom, From)]
pub enum Strategy {
    #[debug(fmt = "{:?}", _0)]
    Minimax(Minimax<Engine>),
    #[cfg(test)]
    #[debug(fmt = "{:?}", _0)]
    Mock(crate::MockSearch),
}

impl Search for Strategy {
    fn search(&self, pos: &Position) -> Option<Move> {
        match self {
            Strategy::Minimax(s) => s.search(pos),
            #[cfg(test)]
            Strategy::Mock(s) => s.search(pos),
        }
    }
}

/// Runtime configuration for [`Search`].
#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum StrategyBuilder {
    Minimax(EngineBuilder, #[serde(default)] MinimaxConfig),

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
            StrategyBuilder::Minimax(engine, config) => {
                Ok(Minimax::with_config(engine.build()?, config).into())
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
    fn minimax_builder_is_deserializable(c: MinimaxConfig) {
        assert_eq!(
            "minimax(mock())".parse(),
            Ok(StrategyBuilder::Minimax(
                EngineBuilder::Mock(),
                MinimaxConfig::default(),
            ))
        );

        assert_eq!(
            format!("minimax(mock(),{})", c).parse(),
            Ok(StrategyBuilder::Minimax(EngineBuilder::Mock(), c))
        );
    }

    #[proptest]
    fn mock_builder_is_deserializable() {
        assert_eq!("mock()".parse(), Ok(StrategyBuilder::Mock()));
    }

    #[proptest]
    fn minimax_can_be_configured_at_runtime(c: MinimaxConfig) {
        assert!(matches!(
            StrategyBuilder::Minimax(EngineBuilder::Mock(), c).build(),
            Ok(Strategy::Minimax(_))
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

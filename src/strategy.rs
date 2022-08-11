use crate::{Build, Engine, EngineBuilder, Move, Position, Search};
use derive_more::{DebugCustom, Display, Error, From};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

mod minimax;

pub use minimax::*;

/// A generic adversarial search algorithm.
#[derive(DebugCustom, From)]
pub enum Strategy {
    #[debug(fmt = "{:?}", _0)]
    Minimax(Minimax<Engine>),
}

impl Search for Strategy {
    fn search(&self, pos: &Position) -> Option<Move> {
        match self {
            Strategy::Minimax(s) => s.search(pos),
        }
    }
}

/// Runtime configuration for [`Search`].
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum StrategyBuilder {
    Minimax(EngineBuilder, #[serde(default)] MinimaxConfig),
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
    type Error = <EngineBuilder as Build>::Error;

    fn build(self) -> Result<Self::Output, Self::Error> {
        match self {
            StrategyBuilder::Minimax(engine, config) => {
                Ok(Minimax::with_config(engine.build()?, config).into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn minimax_builder_is_deserializable(e: EngineBuilder, c: MinimaxConfig) {
        assert_eq!(
            format!("minimax({},{})", ron::ser::to_string(&e)?, c).parse(),
            Ok(StrategyBuilder::Minimax(e.clone(), c))
        );

        assert_eq!(
            format!("minimax({})", ron::ser::to_string(&e)?).parse(),
            Ok(StrategyBuilder::Minimax(e, MinimaxConfig::default()))
        );
    }

    #[proptest]
    fn minimax_can_be_configured_at_runtime(e: EngineBuilder, c: MinimaxConfig) {
        assert!(matches!(
            StrategyBuilder::Minimax(e, c).build(),
            Ok(Strategy::Minimax(_))
        ));
    }
}

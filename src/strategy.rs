use crate::eval::{Builder as EvaluatorBuilder, Dispatcher as Evaluator};
use crate::{chess::Position, Build, Pv, Search, SearchLimits};
use derive_more::{DebugCustom, Display, Error, From};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

mod minimax;

pub use minimax::*;

/// A generic adversarial search algorithm.
#[derive(DebugCustom, From)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub enum Strategy {
    #[debug(fmt = "{:?}", _0)]
    Minimax(Minimax<Evaluator>),
}

impl Default for Strategy {
    fn default() -> Self {
        StrategyBuilder::default().build().unwrap()
    }
}

impl Search for Strategy {
    fn search<const N: usize>(&mut self, pos: &Position, limits: SearchLimits) -> Pv<N> {
        match self {
            Strategy::Minimax(s) => s.search(pos, limits),
        }
    }

    fn clear(&mut self) {
        match self {
            Strategy::Minimax(s) => s.clear(),
        }
    }
}

/// Runtime configuration for [`Search`].
#[derive(Debug, Display, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum StrategyBuilder {
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Minimax(
        #[serde(default)] EvaluatorBuilder,
        #[serde(default)] MinimaxConfig,
    ),
}

impl Default for StrategyBuilder {
    fn default() -> Self {
        StrategyBuilder::Minimax(EvaluatorBuilder::default(), MinimaxConfig::default())
    }
}

/// The reason why parsing [`StrategyBuilder`] failed.
#[derive(Debug, Display, Eq, PartialEq, Error, From)]
#[display(fmt = "failed to parse search configuration")]
pub struct ParseBuilderError(ron::de::SpannedError);

impl FromStr for StrategyBuilder {
    type Err = ParseBuilderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

impl Build for StrategyBuilder {
    type Output = Strategy;
    type Error = <EvaluatorBuilder as Build>::Error;

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
    fn parsing_printed_strategy_builder_is_an_identity(b: StrategyBuilder) {
        assert_eq!(b.to_string().parse(), Ok(b));
    }

    #[proptest]
    fn minimax_builder_is_deserializable(e: EvaluatorBuilder, c: MinimaxConfig) {
        assert_eq!(
            format!("minimax({},{})", e, c).parse(),
            Ok(StrategyBuilder::Minimax(e.clone(), c))
        );

        assert_eq!(
            format!("minimax({})", e).parse(),
            Ok(StrategyBuilder::Minimax(e, MinimaxConfig::default()))
        );
    }

    #[proptest]
    fn minimax_can_be_configured_at_runtime(e: EvaluatorBuilder, c: MinimaxConfig) {
        assert!(matches!(
            StrategyBuilder::Minimax(e, c).build(),
            Ok(Strategy::Minimax(_))
        ));
    }
}

use crate::eval::{Builder as EvaluatorBuilder, Dispatcher as Evaluator};
use crate::{chess::Position, util::Build};
use derive_more::{DebugCustom, Display, Error, From};
use mockall::mock;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use test_strategy::Arbitrary;

mod limits;
mod metrics;
mod minimax;
mod pv;
mod transposition;

pub use limits::*;
pub use metrics::*;
pub use minimax::*;
pub use pv::*;
pub use transposition::*;

/// Trait for types that implement adversarial search algorithms.
pub trait Search {
    /// Clear the transposition table.
    fn clear(&mut self);

    /// Searches for the strongest [variation][`Pv`].
    fn search<const N: usize>(&mut self, pos: &Position, limits: Limits) -> Pv<N>;
}

mock! {
    #[derive(Debug)]
    pub Search {
        pub fn clear(&mut self);
        pub fn search(&mut self, pos: &Position, limits: Limits) -> Pv;
    }
}

impl Search for MockSearch {
    fn clear(&mut self) {
        MockSearch::clear(self)
    }

    fn search<const N: usize>(&mut self, pos: &Position, limits: Limits) -> Pv<N> {
        MockSearch::search(self, pos, limits).truncate()
    }
}

/// A generic adversarial search algorithm.
#[derive(DebugCustom, Arbitrary, From)]
pub enum Dispatcher {
    #[debug(fmt = "{:?}", _0)]
    Minimax(Minimax<Evaluator>),
}

impl Default for Dispatcher {
    fn default() -> Self {
        Builder::default().build().unwrap()
    }
}

impl Search for Dispatcher {
    fn search<const N: usize>(&mut self, pos: &Position, limits: Limits) -> Pv<N> {
        match self {
            Dispatcher::Minimax(s) => s.search(pos, limits),
        }
    }

    fn clear(&mut self) {
        match self {
            Dispatcher::Minimax(s) => s.clear(),
        }
    }
}

/// Runtime configuration for [`Dispatcher`].
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum Builder {
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Minimax(
        #[serde(default)] EvaluatorBuilder,
        #[serde(default)] MinimaxConfig,
    ),
}

impl Default for Builder {
    fn default() -> Self {
        Builder::Minimax(EvaluatorBuilder::default(), MinimaxConfig::default())
    }
}

/// The reason why parsing [`Builder`] failed.
#[derive(Debug, Display, Eq, PartialEq, Error, From)]
#[display(fmt = "failed to parse search configuration")]
pub struct ParseBuilderError(ron::de::SpannedError);

impl FromStr for Builder {
    type Err = ParseBuilderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

impl Build for Builder {
    type Output = Dispatcher;
    type Error = <EvaluatorBuilder as Build>::Error;

    fn build(self) -> Result<Self::Output, Self::Error> {
        match self {
            Builder::Minimax(evaluator, config) => {
                Ok(Minimax::with_config(evaluator.build()?, config).into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn parsing_printed_strategy_builder_is_an_identity(b: Builder) {
        assert_eq!(b.to_string().parse(), Ok(b));
    }

    #[proptest]
    fn minimax_builder_is_deserializable(e: EvaluatorBuilder, c: MinimaxConfig) {
        assert_eq!(
            format!("minimax({},{})", e, c).parse(),
            Ok(Builder::Minimax(e.clone(), c))
        );

        assert_eq!(
            format!("minimax({})", e).parse(),
            Ok(Builder::Minimax(e, MinimaxConfig::default()))
        );
    }

    #[proptest]
    fn minimax_can_be_configured_at_runtime(e: EvaluatorBuilder, c: MinimaxConfig) {
        assert!(matches!(
            Builder::Minimax(e, c).build(),
            Ok(Dispatcher::Minimax(_))
        ));
    }
}

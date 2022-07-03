use crate::{Build, Eval, Game};
use anyhow::Error as Anyhow;
use derive_more::{DebugCustom, Display, Error, From};
use serde::Deserialize;
use std::str::FromStr;

mod heuristic;
mod random;

pub use heuristic::*;
pub use random::*;

/// A generic chess engine.
#[derive(DebugCustom, From)]
pub enum Engine {
    #[debug(fmt = "{:?}", _0)]
    Random(Random),
    #[debug(fmt = "{:?}", _0)]
    Heuristic(Heuristic),
    #[cfg(test)]
    #[debug(fmt = "{:?}", _0)]
    Mock(crate::MockEval),
}

impl Eval for Engine {
    fn eval(&self, game: &Game) -> i16 {
        match self {
            Engine::Random(e) => e.eval(game),
            Engine::Heuristic(e) => e.eval(game),
            #[cfg(test)]
            Engine::Mock(e) => e.eval(game),
        }
    }
}

/// Runtime configuration for an [`Engine`].
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum EngineBuilder {
    Random {},
    Heuristic {},
    #[cfg(test)]
    Mock(),
}

/// The reason why parsing [`EngineBuilder`] failed.
#[derive(Debug, Display, PartialEq, Error, From)]
#[display(fmt = "failed to parse engine configuration")]
pub struct ParseBuilderError(ron::de::Error);

impl FromStr for EngineBuilder {
    type Err = ParseBuilderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

impl Build for EngineBuilder {
    type Output = Engine;

    fn build(self) -> Result<Self::Output, Anyhow> {
        match self {
            EngineBuilder::Random {} => Ok(Random::new().into()),
            EngineBuilder::Heuristic { .. } => Ok(Heuristic::new().into()),
            #[cfg(test)]
            EngineBuilder::Mock() => Ok(crate::MockEval::new().into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn random_builder_is_deserializable() {
        assert_eq!("random()".parse(), Ok(EngineBuilder::Random {}));
    }

    #[proptest]
    fn heuristic_builder_is_deserializable() {
        assert_eq!("heuristic()".parse(), Ok(EngineBuilder::Heuristic {}));
    }

    #[proptest]
    fn mock_engine_builder_is_deserializable() {
        assert_eq!("mock()".parse(), Ok(EngineBuilder::Mock()));
    }

    #[proptest]
    fn random_can_be_configured_at_runtime() {
        assert!(matches!(
            EngineBuilder::Random {}.build(),
            Ok(Engine::Random(_))
        ));
    }

    #[proptest]
    fn heuristic_can_be_configured_at_runtime() {
        assert!(matches!(
            EngineBuilder::Heuristic {}.build(),
            Ok(Engine::Heuristic(_))
        ));
    }

    #[proptest]
    fn mock_can_be_configured_at_runtime() {
        assert!(matches!(EngineBuilder::Mock().build(), Ok(Engine::Mock(_))));
    }
}

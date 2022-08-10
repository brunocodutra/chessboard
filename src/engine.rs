use crate::{Build, Eval, Position};
use anyhow::Error as Anyhow;
use derive_more::{DebugCustom, Display, Error, From};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

mod materialist;
mod pesto;
mod pst;
mod random;

pub use materialist::*;
pub use pesto::*;
pub use pst::*;
pub use random::*;

/// A generic chess engine.
#[derive(DebugCustom, From)]
pub enum Engine {
    #[debug(fmt = "{:?}", _0)]
    Random(Random),
    #[debug(fmt = "{:?}", _0)]
    Materialist(Materialist),
    #[debug(fmt = "{:?}", _0)]
    Pesto(Pesto),
}

impl Default for Engine {
    fn default() -> Self {
        Pesto::default().into()
    }
}

impl Eval for Engine {
    fn eval(&self, pos: &Position) -> i16 {
        match self {
            Engine::Random(e) => e.eval(pos),
            Engine::Materialist(e) => e.eval(pos),
            Engine::Pesto(e) => e.eval(pos),
        }
    }
}

/// Runtime configuration for an [`Engine`].
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum EngineBuilder {
    Random {},
    Materialist {},
    Pesto {},
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
            EngineBuilder::Materialist { .. } => Ok(Materialist::new().into()),
            EngineBuilder::Pesto { .. } => Ok(Pesto::new().into()),
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
    fn materialist_builder_is_deserializable() {
        assert_eq!("materialist()".parse(), Ok(EngineBuilder::Materialist {}));
    }

    #[proptest]
    fn pesto_builder_is_deserializable() {
        assert_eq!("pesto()".parse(), Ok(EngineBuilder::Pesto {}));
    }

    #[proptest]
    fn random_can_be_configured_at_runtime() {
        assert!(matches!(
            EngineBuilder::Random {}.build(),
            Ok(Engine::Random(_))
        ));
    }

    #[proptest]
    fn materialist_can_be_configured_at_runtime() {
        assert!(matches!(
            EngineBuilder::Materialist {}.build(),
            Ok(Engine::Materialist(_))
        ));
    }

    #[proptest]
    fn pesto_can_be_configured_at_runtime() {
        assert!(matches!(
            EngineBuilder::Pesto {}.build(),
            Ok(Engine::Pesto(_))
        ));
    }
}

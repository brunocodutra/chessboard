use crate::{Eval, Position, Setup};
use anyhow::Error as Anyhow;
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use serde::Deserialize;
use std::str::FromStr;
use tracing::instrument;

mod random;

pub use random::Random;

/// A generic chess engine.
#[derive(DebugCustom, From)]
pub enum Engine {
    #[debug(fmt = "{:?}", _0)]
    Random(Random),
    #[cfg(test)]
    #[debug(fmt = "{:?}", _0)]
    Mock(crate::MockEval),
}

impl Eval for Engine {
    fn eval(&self, pos: &Position) -> i32 {
        match self {
            Engine::Random(e) => e.eval(pos),
            #[cfg(test)]
            Engine::Mock(e) => e.eval(pos),
        }
    }
}

/// Runtime configuration for an [`Engine`].
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum EngineConfig {
    Random {},
    #[cfg(test)]
    Mock(),
}

/// The reason why parsing [`EngineConfig`] failed.
#[derive(Debug, Display, PartialEq, Error, From)]
#[display(fmt = "failed to parse engine configuration")]
pub struct ParseConfigError(ron::de::Error);

impl FromStr for EngineConfig {
    type Err = ParseConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

#[async_trait]
impl Setup for EngineConfig {
    type Output = Engine;

    #[instrument(level = "trace", err, ret)]
    async fn setup(self) -> Result<Self::Output, Anyhow> {
        match self {
            EngineConfig::Random {} => Ok(Random::new().into()),
            #[cfg(test)]
            EngineConfig::Mock() => Ok(crate::MockEval::new().into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockEval;
    use std::mem::discriminant;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn engine_config_is_deserializable() {
        assert_eq!("random()".parse(), Ok(EngineConfig::Random {}));
        assert_eq!("mock()".parse(), Ok(EngineConfig::Mock()));
    }

    #[proptest]
    fn engine_can_be_configured_at_runtime() {
        let rt = runtime::Builder::new_multi_thread().build()?;

        assert_eq!(
            discriminant(&Engine::Random(Random::new())),
            discriminant(&rt.block_on(EngineConfig::Random {}.setup()).unwrap())
        );

        assert_eq!(
            discriminant(&Engine::Mock(MockEval::new())),
            discriminant(&rt.block_on(EngineConfig::Mock().setup()).unwrap())
        );
    }
}

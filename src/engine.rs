use crate::{Position, Setup};
use anyhow::Error as Anyhow;
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use serde::Deserialize;
use std::str::FromStr;
use tracing::instrument;

mod random;

pub use random::Random;

/// Trait for types that implement adversarial search algorithms.
#[cfg_attr(test, mockall::automock)]
pub trait Engine {
    /// Evaluates a position.
    ///
    /// Positive values favor the current side to play.
    fn evaluate(&self, pos: &Position) -> i32;
}

/// A static dispatcher [`Engine`].
#[derive(DebugCustom, From)]
pub enum Dispatcher {
    #[debug(fmt = "{:?}", _0)]
    Random(Random),
    #[cfg(test)]
    #[debug(fmt = "{:?}", _0)]
    Mock(MockEngine),
}

impl Engine for Dispatcher {
    fn evaluate(&self, pos: &Position) -> i32 {
        match self {
            Dispatcher::Random(e) => e.evaluate(pos),
            #[cfg(test)]
            Dispatcher::Mock(e) => e.evaluate(pos),
        }
    }
}

/// Runtime configuration for an [`Engine`].
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum Config {
    Random {},
    #[cfg(test)]
    Mock(),
}

/// The reason why parsing [`Config`] failed.
#[derive(Debug, Display, PartialEq, Error, From)]
#[display(fmt = "failed to parse engine configuration")]
pub struct ParseConfigError(ron::de::Error);

impl FromStr for Config {
    type Err = ParseConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

#[async_trait]
impl Setup for Config {
    type Output = Dispatcher;

    #[instrument(level = "trace", err)]
    async fn setup(self) -> Result<Self::Output, Anyhow> {
        match self {
            Config::Random {} => Ok(Random::new().into()),
            #[cfg(test)]
            Config::Mock() => Ok(MockEngine::new().into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::discriminant;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn engine_config_is_deserializable() {
        assert_eq!("random()".parse(), Ok(Config::Random {}));
        assert_eq!("mock()".parse(), Ok(Config::Mock()));
    }

    #[proptest]
    fn engine_can_be_configured_at_runtime() {
        let rt = runtime::Builder::new_multi_thread().build()?;

        assert_eq!(
            discriminant(&Dispatcher::Random(Random::new())),
            discriminant(&rt.block_on(Config::Random {}.setup()).unwrap())
        );

        assert_eq!(
            discriminant(&Dispatcher::Mock(MockEngine::new())),
            discriminant(&rt.block_on(Config::Mock().setup()).unwrap())
        );
    }
}

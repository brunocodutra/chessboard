use crate::{EngineConfig, EngineDispatcher, Move, Position, Setup};
use anyhow::Error as Anyhow;
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use serde::Deserialize;
use std::str::FromStr;
use tracing::instrument;

mod negamax;

pub use negamax::Negamax;

/// Trait for types that implement adversarial search algorithms.
#[cfg_attr(test, mockall::automock)]
pub trait Search {
    /// Searches for the strongest [`Move`] in this [`Position`], if one exists.
    fn search(&mut self, pos: &Position) -> Option<Move>;
}

/// A static dispatcher for [`Search`].
#[derive(DebugCustom, From)]
pub enum Dispatcher {
    #[debug(fmt = "{:?}", _0)]
    Negamax(Negamax<EngineDispatcher>),
    #[cfg(test)]
    #[debug(fmt = "{:?}", _0)]
    Mock(MockSearch),
}

impl Search for Dispatcher {
    fn search(&mut self, pos: &Position) -> Option<Move> {
        match self {
            Dispatcher::Negamax(s) => s.search(pos),
            #[cfg(test)]
            Dispatcher::Mock(s) => s.search(pos),
        }
    }
}

/// Runtime configuration for [`Search`].
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum Config {
    Negamax {
        engine: EngineConfig,
    },

    #[cfg(test)]
    Mock(),
}

/// The reason why parsing [`Config`] failed.
#[derive(Debug, Display, PartialEq, Error, From)]
#[display(fmt = "failed to parse search configuration")]
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
            Config::Negamax { engine: cfg } => Ok(Negamax::new(cfg.setup().await?).into()),
            #[cfg(test)]
            Config::Mock() => Ok(MockSearch::new().into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::MockEngine;
    use std::mem::discriminant;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn search_config_is_deserializable() {
        assert_eq!(
            "negamax(engine:mock())".parse(),
            Ok(Config::Negamax {
                engine: EngineConfig::Mock()
            })
        );

        assert_eq!("mock()".parse(), Ok(Config::Mock()));
    }

    #[proptest]
    fn search_can_be_configured_at_runtime() {
        let rt = runtime::Builder::new_multi_thread().build()?;

        assert_eq!(
            discriminant(&Dispatcher::Negamax(Negamax::new(EngineDispatcher::Mock(
                MockEngine::new()
            )))),
            discriminant(
                &rt.block_on(
                    Config::Negamax {
                        engine: EngineConfig::Mock()
                    }
                    .setup()
                )
                .unwrap()
            )
        );

        assert_eq!(
            discriminant(&Dispatcher::Mock(MockSearch::new())),
            discriminant(&rt.block_on(Config::Mock().setup()).unwrap())
        );
    }
}

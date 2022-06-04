use crate::{Engine, EngineConfig, Move, Position, Search, SearchControl, Setup};
use anyhow::Error as Anyhow;
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use serde::Deserialize;
use std::str::FromStr;
use tracing::instrument;

mod negamax;

pub use negamax::Negamax;

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
    fn search(&mut self, pos: &Position, ctrl: SearchControl) -> Option<Move> {
        match self {
            Strategy::Negamax(s) => s.search(pos, ctrl),
            #[cfg(test)]
            Strategy::Mock(s) => s.search(pos, ctrl),
        }
    }
}

/// Runtime configuration for [`Search`].
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum StrategyConfig {
    Negamax {
        engine: EngineConfig,
    },

    #[cfg(test)]
    Mock(),
}

/// The reason why parsing [`StrategyConfig`] failed.
#[derive(Debug, Display, PartialEq, Error, From)]
#[display(fmt = "failed to parse search configuration")]
pub struct ParseConfigError(ron::de::Error);

impl FromStr for StrategyConfig {
    type Err = ParseConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

#[async_trait]
impl Setup for StrategyConfig {
    type Output = Strategy;

    #[instrument(level = "trace", err, ret)]
    async fn setup(self) -> Result<Self::Output, Anyhow> {
        match self {
            StrategyConfig::Negamax { engine: cfg } => Ok(Negamax::new(cfg.setup().await?).into()),
            #[cfg(test)]
            StrategyConfig::Mock() => Ok(crate::MockSearch::new().into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockEval, MockSearch};
    use std::mem::discriminant;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn negamax_config_is_deserializable() {
        assert_eq!(
            "negamax(engine:mock())".parse(),
            Ok(StrategyConfig::Negamax {
                engine: EngineConfig::Mock()
            })
        );
    }

    #[proptest]
    fn mock_config_is_deserializable() {
        assert_eq!("mock()".parse(), Ok(StrategyConfig::Mock()));
    }

    #[proptest]
    fn negamax_can_be_configured_at_runtime() {
        let rt = runtime::Builder::new_multi_thread().build()?;

        assert_eq!(
            discriminant(&Strategy::Negamax(Negamax::new(Engine::Mock(
                MockEval::new()
            )))),
            discriminant(
                &rt.block_on(
                    StrategyConfig::Negamax {
                        engine: EngineConfig::Mock()
                    }
                    .setup()
                )
                .unwrap()
            )
        );
    }

    #[proptest]
    fn mock_can_be_configured_at_runtime() {
        let rt = runtime::Builder::new_multi_thread().build()?;

        assert_eq!(
            discriminant(&Strategy::Mock(MockSearch::new())),
            discriminant(&rt.block_on(StrategyConfig::Mock().setup()).unwrap())
        );
    }
}

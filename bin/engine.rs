use crate::{io::Process, play::Play};
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use lib::chess::{Move, Position};
use lib::search::{Limits, Options};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use test_strategy::Arbitrary;

mod ai;
mod uci;

pub use ai::*;
pub use uci::*;

/// The reason why parsing engine configuration failed.
#[derive(Debug, Display, Eq, PartialEq, Error, From)]
#[display(fmt = "failed to parse engine configuration")]
pub struct ParseEngineConfigError(ron::de::SpannedError);

/// Runtime configuration for an [`Engine`].
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum EngineConfig {
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Ai(#[serde(default)] Options),

    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Uci(String, #[serde(default)] UciOptions),
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig::Ai(Options::default())
    }
}

impl FromStr for EngineConfig {
    type Err = ParseEngineConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

/// The reason why [`Engine`] failed to play a [`Move`].
#[derive(Debug, Display, Error, From)]
pub enum EngineError {
    Ai(<Ai as Play>::Error),
    Uci(<Uci<Process> as Play>::Error),
}

/// A generic chess engine.
#[derive(DebugCustom, From)]
#[allow(clippy::large_enum_variant)]
pub enum Engine {
    #[debug(fmt = "{_0:?}")]
    Ai(Ai),
    #[debug(fmt = "{_0:?}")]
    Uci(Uci<Process>),
}

#[async_trait]
impl Play for Engine {
    type Error = EngineError;

    #[inline]
    async fn play(&mut self, pos: &Position, limits: Limits) -> Result<Move, Self::Error> {
        match self {
            Engine::Ai(e) => Ok(e.play(pos, limits).await?),
            Engine::Uci(e) => Ok(e.play(pos, limits).await?),
        }
    }
}

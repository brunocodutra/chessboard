use crate::{io::Process, player::Player};
use async_stream::stream;
use derive_more::{DebugCustom, Display, Error, From};
use futures_util::{future::BoxFuture, stream::BoxStream};
use lib::chess::{Move, Position};
use lib::search::{Limits, Options, Pv};
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
    Ai(<Ai as Player>::Error),
    Uci(<Uci<Process> as Player>::Error),
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

impl Player for Engine {
    type Error = EngineError;

    fn play<'a, 'b, 'c>(
        &'a mut self,
        pos: &'b Position,
        limits: Limits,
    ) -> BoxFuture<'c, Result<Move, Self::Error>>
    where
        'a: 'c,
        'b: 'c,
    {
        Box::pin(async move {
            match self {
                Engine::Ai(e) => Ok(e.play(pos, limits).await?),
                Engine::Uci(e) => Ok(e.play(pos, limits).await?),
            }
        })
    }

    fn analyze<'a, 'b, 'c>(
        &'a mut self,
        pos: &'b Position,
        limits: Limits,
    ) -> BoxStream<'c, Result<Pv, Self::Error>>
    where
        'a: 'c,
        'b: 'c,
    {
        Box::pin(stream! {
            match self {
                Engine::Ai(e) => for await pv in e.analyze(pos, limits) {
                    yield Ok(pv?)
                }

                Engine::Uci(e) => for await pv in e.analyze(pos, limits) {
                    yield Ok(pv?)
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn parsing_printed_engine_config_is_an_identity(b: EngineConfig) {
        assert_eq!(b.to_string().parse(), Ok(b));
    }

    #[proptest]
    fn ai_config_is_deserializable(o: Options) {
        assert_eq!("ai(())".parse(), Ok(EngineConfig::Ai(Options::default())));
        assert_eq!(format!("ai({o})").parse(), Ok(EngineConfig::Ai(o)));
    }

    #[proptest]
    fn uci_config_is_deserializable(p: String, o: UciOptions) {
        assert_eq!(
            format!("uci({p:?})").parse(),
            Ok(EngineConfig::Uci(p.clone(), UciOptions::default()))
        );

        assert_eq!(
            format!("uci({p:?}, {})", ron::ser::to_string(&o)?).parse(),
            Ok(EngineConfig::Uci(p, o))
        );
    }
}

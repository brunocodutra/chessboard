use crate::{chess::Position, util::Build};
use derive_more::{DebugCustom, Display, Error, From};
use mockall::automock;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, str::FromStr};
use test_strategy::Arbitrary;

mod materialist;
mod pesto;
mod pst;
mod random;

pub use materialist::*;
pub use pesto::*;
pub use pst::*;
pub use random::*;

/// Trait for types that can evaluate a [`Position`].
#[automock]
pub trait Eval {
    /// Evaluates a [`Position`].
    ///
    /// Positive values favor the current side to play.
    fn eval(&self, pos: &Position) -> i16;
}

/// A generic chess engine.
#[derive(DebugCustom, Clone, Arbitrary, From)]
pub enum Dispatcher {
    #[debug(fmt = "{:?}", _0)]
    Random(Random),
    #[debug(fmt = "{:?}", _0)]
    Materialist(Materialist),
    #[debug(fmt = "{:?}", _0)]
    Pesto(Pesto),
}

impl Default for Dispatcher {
    fn default() -> Self {
        Builder::default().build().unwrap()
    }
}

impl Eval for Dispatcher {
    fn eval(&self, pos: &Position) -> i16 {
        match self {
            Dispatcher::Random(e) => e.eval(pos),
            Dispatcher::Materialist(e) => e.eval(pos),
            Dispatcher::Pesto(e) => e.eval(pos),
        }
    }
}

/// Runtime configuration for a [`Dispatcher`].
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum Builder {
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Random {},
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Materialist {},
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Pesto {},
}

impl Default for Builder {
    fn default() -> Self {
        Builder::Pesto {}
    }
}

/// The reason why parsing [`Builder`] failed.
#[derive(Debug, Display, Eq, PartialEq, Error, From)]
#[display(fmt = "failed to parse engine configuration")]
pub struct ParseBuilderError(ron::de::SpannedError);

impl FromStr for Builder {
    type Err = ParseBuilderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

impl Build for Builder {
    type Output = Dispatcher;
    type Error = Infallible;

    fn build(self) -> Result<Self::Output, Self::Error> {
        match self {
            Builder::Random {} => Ok(Random::new().into()),
            Builder::Materialist { .. } => Ok(Materialist::new().into()),
            Builder::Pesto { .. } => Ok(Pesto::new().into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn parsing_printed_engine_builder_is_an_identity(b: Builder) {
        assert_eq!(b.to_string().parse(), Ok(b));
    }

    #[proptest]
    fn random_builder_is_deserializable() {
        assert_eq!("random()".parse(), Ok(Builder::Random {}));
    }

    #[proptest]
    fn materialist_builder_is_deserializable() {
        assert_eq!("materialist()".parse(), Ok(Builder::Materialist {}));
    }

    #[proptest]
    fn pesto_builder_is_deserializable() {
        assert_eq!("pesto()".parse(), Ok(Builder::Pesto {}));
    }

    #[proptest]
    fn random_can_be_configured_at_runtime() {
        assert!(matches!(
            Builder::Random {}.build(),
            Ok(Dispatcher::Random(_))
        ));
    }

    #[proptest]
    fn materialist_can_be_configured_at_runtime() {
        assert!(matches!(
            Builder::Materialist {}.build(),
            Ok(Dispatcher::Materialist(_))
        ));
    }

    #[proptest]
    fn pesto_can_be_configured_at_runtime() {
        assert!(matches!(
            Builder::Pesto {}.build(),
            Ok(Dispatcher::Pesto(_))
        ));
    }
}

use crate::engine::{Ai, Engine, EngineConfig, EngineError, Uci, UciError};
use crate::io::Process;
use lib::eval::Pesto;

/// Trait for types that build other types.
pub trait Build {
    /// The type to be built.
    type Output;

    /// The reason why [`Build::Output`] could not be built.
    type Error;

    /// Build an instance of [`Build::Output`].
    fn build(self) -> Result<Self::Output, Self::Error>;
}

impl Build for EngineConfig {
    type Output = Engine;
    type Error = EngineError;

    fn build(self) -> Result<Self::Output, Self::Error> {
        match self {
            EngineConfig::Pesto(limits, options) => {
                Ok(Ai::new(Pesto::new().into(), limits, options).into())
            }

            EngineConfig::Uci(path, limits, options) => {
                let io = Process::spawn(&path).map_err(UciError::from)?;
                Ok(Uci::new(io, limits, options).into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::UciOptions;
    use lib::search::{Limits, Options};
    use test_strategy::proptest;

    #[proptest]
    fn pesto_can_be_configured_at_runtime(l: Limits, o: Options) {
        assert!(matches!(
            EngineConfig::Pesto(l, o).build(),
            Ok(Engine::Ai(_))
        ));
    }

    #[proptest]
    fn uci_can_be_configured_at_runtime(s: String, l: Limits, o: UciOptions) {
        assert!(matches!(
            EngineConfig::Uci(s, l, o).build(),
            Ok(Engine::Uci(_))
        ));
    }
}

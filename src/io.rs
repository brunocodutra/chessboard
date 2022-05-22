use crate::Setup;
use anyhow::Error as Anyhow;
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use serde::Deserialize;
use std::{fmt::Display, io, str::FromStr};
use tracing::instrument;

mod process;
mod terminal;

pub use process::*;
pub use terminal::*;

/// Trait for types that communicate via message-passing.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Io {
    /// Receive a message.
    async fn recv(&mut self) -> io::Result<String>;

    /// Send a message.
    async fn send<D: Display + Send + 'static>(&mut self, item: D) -> io::Result<()>;

    /// Flush the internal buffers.
    async fn flush(&mut self) -> io::Result<()>;
}

/// A static dispatcher for [`Io`].
#[allow(clippy::large_enum_variant)]
#[derive(DebugCustom, From)]
pub enum Dispatcher {
    #[debug(fmt = "{:?}", _0)]
    Process(Process),
    #[debug(fmt = "{:?}", _0)]
    Terminal(Terminal),
    #[cfg(test)]
    #[debug(fmt = "{:?}", _0)]
    Mock(MockIo),
}

#[async_trait]
impl Io for Dispatcher {
    async fn recv(&mut self) -> io::Result<String> {
        let line = match self {
            Dispatcher::Process(io) => io.recv().await?,
            Dispatcher::Terminal(io) => io.recv().await?,
            #[cfg(test)]
            Dispatcher::Mock(io) => io.recv().await?,
        };

        Ok(line)
    }

    async fn send<D: Display + Send + 'static>(&mut self, item: D) -> io::Result<()> {
        match self {
            Dispatcher::Process(io) => io.send(item).await?,
            Dispatcher::Terminal(io) => io.send(item).await?,
            #[cfg(test)]
            Dispatcher::Mock(io) => io.send(item).await?,
        }

        Ok(())
    }

    async fn flush(&mut self) -> io::Result<()> {
        match self {
            Dispatcher::Process(io) => io.flush().await?,
            Dispatcher::Terminal(io) => io.flush().await?,
            #[cfg(test)]
            Dispatcher::Mock(io) => io.flush().await?,
        }

        Ok(())
    }
}

/// Runtime configuration for [`Io`].
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum Config {
    #[serde(rename = "proc")]
    Process(String),
    #[serde(rename = "term")]
    Terminal,
    #[cfg(test)]
    Mock(),
}

/// The reason why parsing [`Config`] failed.
#[derive(Debug, Display, PartialEq, Error, From)]
#[display(fmt = "failed to parse io configuration")]
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
            Config::Process(path) => Ok(Process::spawn(&path).await?.into()),
            Config::Terminal => Ok(Terminal::open().into()),
            #[cfg(test)]
            Config::Mock() => Ok(MockIo::new().into()),
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
    fn io_config_is_deserializable(s: String) {
        assert_eq!("term".parse(), Ok(Config::Terminal));
        assert_eq!(format!("proc({:?})", s).parse(), Ok(Config::Process(s)));
        assert_eq!("mock()".parse(), Ok(Config::Mock()));
    }

    #[proptest]
    fn io_can_be_configured_at_runtime(s: String) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        assert_eq!(
            discriminant(&Dispatcher::Process(rt.block_on(Process::spawn(&s))?)),
            discriminant(&rt.block_on(Config::Process(s).setup()).unwrap())
        );

        assert_eq!(
            discriminant(&Dispatcher::Terminal(Terminal::open())),
            discriminant(&rt.block_on(Config::Terminal.setup()).unwrap())
        );

        assert_eq!(
            discriminant(&Dispatcher::Mock(MockIo::new())),
            discriminant(&rt.block_on(Config::Mock().setup()).unwrap())
        );
    }
}

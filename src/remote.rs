use crate::{Io, Setup};
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

/// A generic [`Io`] interface.
#[allow(clippy::large_enum_variant)]
#[derive(DebugCustom, From)]
pub enum Remote {
    #[debug(fmt = "{:?}", _0)]
    Process(Process),
    #[debug(fmt = "{:?}", _0)]
    Terminal(Terminal),
    #[cfg(test)]
    #[debug(fmt = "{:?}", _0)]
    Mock(crate::MockIo),
}

#[async_trait]
impl Io for Remote {
    async fn recv(&mut self) -> io::Result<String> {
        let line = match self {
            Remote::Process(io) => io.recv().await?,
            Remote::Terminal(io) => io.recv().await?,
            #[cfg(test)]
            Remote::Mock(io) => io.recv().await?,
        };

        Ok(line)
    }

    async fn send<D: Display + Send + 'static>(&mut self, item: D) -> io::Result<()> {
        match self {
            Remote::Process(io) => io.send(item).await?,
            Remote::Terminal(io) => io.send(item).await?,
            #[cfg(test)]
            Remote::Mock(io) => io.send(item).await?,
        }

        Ok(())
    }

    async fn flush(&mut self) -> io::Result<()> {
        match self {
            Remote::Process(io) => io.flush().await?,
            Remote::Terminal(io) => io.flush().await?,
            #[cfg(test)]
            Remote::Mock(io) => io.flush().await?,
        }

        Ok(())
    }
}

/// Runtime configuration for [`Io`].
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum RemoteConfig {
    #[serde(rename = "proc")]
    Process(String),
    #[serde(rename = "term")]
    Terminal,
    #[cfg(test)]
    Mock(),
}

/// The reason why parsing [`RemoteConfig`] failed.
#[derive(Debug, Display, PartialEq, Error, From)]
#[display(fmt = "failed to parse io configuration")]
pub struct ParseConfigError(ron::de::Error);

impl FromStr for RemoteConfig {
    type Err = ParseConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

#[async_trait]
impl Setup for RemoteConfig {
    type Output = Remote;

    #[instrument(level = "trace", err, ret)]
    async fn setup(self) -> Result<Self::Output, Anyhow> {
        match self {
            RemoteConfig::Process(path) => Ok(Process::spawn(&path).await?.into()),
            RemoteConfig::Terminal => Ok(Terminal::open().into()),
            #[cfg(test)]
            RemoteConfig::Mock() => Ok(crate::MockIo::new().into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockIo;
    use std::mem::discriminant;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn terminal_config_is_deserializable() {
        assert_eq!("term".parse(), Ok(RemoteConfig::Terminal));
    }

    #[proptest]
    fn process_config_is_deserializable(s: String) {
        assert_eq!(
            format!("proc({:?})", s).parse(),
            Ok(RemoteConfig::Process(s))
        );
    }

    #[proptest]
    fn mock_config_is_deserializable() {
        assert_eq!("mock()".parse(), Ok(RemoteConfig::Mock()));
    }

    #[proptest]
    fn process_can_be_configured_at_runtime(s: String) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        assert_eq!(
            discriminant(&Remote::Process(rt.block_on(Process::spawn(&s))?)),
            discriminant(&rt.block_on(RemoteConfig::Process(s).setup()).unwrap())
        );
    }

    #[proptest]
    fn terminal_can_be_configured_at_runtime() {
        let rt = runtime::Builder::new_multi_thread().build()?;

        assert_eq!(
            discriminant(&Remote::Terminal(Terminal::open())),
            discriminant(&rt.block_on(RemoteConfig::Terminal.setup()).unwrap())
        );
    }

    #[proptest]
    fn mock_can_be_configured_at_runtime() {
        let rt = runtime::Builder::new_multi_thread().build()?;

        assert_eq!(
            discriminant(&Remote::Mock(MockIo::new())),
            discriminant(&rt.block_on(RemoteConfig::Mock().setup()).unwrap())
        );
    }
}

use async_trait::async_trait;
use derive_more::{DebugCustom, From};
use std::{fmt::Display, io};
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
pub enum IoDispatcher {
    #[debug(fmt = "{:?}", _0)]
    Process(Process),
    #[debug(fmt = "{:?}", _0)]
    Terminal(Terminal),
}

#[async_trait]
impl Io for IoDispatcher {
    #[instrument(level = "trace", err)]
    async fn recv(&mut self) -> io::Result<String> {
        use IoDispatcher::*;
        let line = match self {
            Process(r) => r.recv().await?,
            Terminal(r) => r.recv().await?,
        };

        Ok(line)
    }

    #[instrument(level = "trace", skip(item), err, fields(%item))]
    async fn send<D: Display + Send + 'static>(&mut self, item: D) -> io::Result<()> {
        use IoDispatcher::*;
        match self {
            Process(r) => r.send(item).await?,
            Terminal(r) => r.send(item).await?,
        }

        Ok(())
    }

    #[instrument(level = "trace", err)]
    async fn flush(&mut self) -> io::Result<()> {
        use IoDispatcher::*;
        match self {
            Process(r) => r.flush().await?,
            Terminal(r) => r.flush().await?,
        }

        Ok(())
    }
}

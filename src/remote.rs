use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use std::fmt::Display;
use tracing::instrument;

mod process;
mod tcp;
mod terminal;

pub use process::*;
pub use tcp::*;
pub use terminal::*;

/// Trait for types that communicate via message-passing.
#[cfg_attr(test, mockall::automock(type Error = std::io::Error;))]
#[async_trait]
pub trait Remote {
    /// The reason why sending/receiving a message failed.
    type Error;

    /// Receive a message from the remote endpoint.
    async fn recv(&mut self) -> Result<String, Self::Error>;

    /// Send a message to the remote endpoint.
    async fn send<D: Display + Send + 'static>(&mut self, item: D) -> Result<(), Self::Error>;

    /// Flush the internal buffers.
    async fn flush(&mut self) -> Result<(), Self::Error>;
}

#[cfg(test)]
impl std::fmt::Debug for MockRemote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MockRemote")
    }
}

/// The reason why the underlying remote failed.
#[derive(Debug, Display, Error, From)]
#[display(fmt = "failed to communicate with the remote {}")]
pub enum RemoteDispatcherError {
    #[display(fmt = "TCP endpoint")]
    TcpIoError(TcpIoError),
    #[display(fmt = "process")]
    ProcessIoError(ProcessIoError),
    #[display(fmt = "terminal")]
    TerminalIoError(TerminalIoError),
}

/// A static dispatcher for [`Remote`].
#[derive(DebugCustom, From)]
pub enum RemoteDispatcher {
    #[debug(fmt = "{:?}", _0)]
    Tcp(Tcp),
    #[debug(fmt = "{:?}", _0)]
    Process(Process),
    #[debug(fmt = "{:?}", _0)]
    Terminal(Terminal),
}

#[async_trait]
impl Remote for RemoteDispatcher {
    type Error = RemoteDispatcherError;

    #[instrument(err)]
    async fn recv(&mut self) -> Result<String, Self::Error> {
        use RemoteDispatcher::*;
        let line = match self {
            Tcp(r) => r.recv().await?,
            Process(r) => r.recv().await?,
            Terminal(r) => r.recv().await?,
        };

        Ok(line)
    }

    #[instrument(skip(item), err, fields(%item))]
    async fn send<D: Display + Send + 'static>(&mut self, item: D) -> Result<(), Self::Error> {
        use RemoteDispatcher::*;
        match self {
            Tcp(r) => r.send(item).await?,
            Process(r) => r.send(item).await?,
            Terminal(r) => r.send(item).await?,
        }

        Ok(())
    }

    #[instrument(err)]
    async fn flush(&mut self) -> Result<(), Self::Error> {
        use RemoteDispatcher::*;
        match self {
            Tcp(r) => r.flush().await?,
            Process(r) => r.flush().await?,
            Terminal(r) => r.flush().await?,
        }

        Ok(())
    }
}

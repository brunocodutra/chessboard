use async_trait::async_trait;
use derive_more::{Display, Error, From};
use std::fmt::Display;

mod process;
mod tcp;
mod terminal;

pub use process::*;
pub use tcp::*;
pub use terminal::*;

/// Trait for types that communicate via message-passing.
#[async_trait]
pub trait Remote {
    /// The reason why sending/receiving a message failed.
    type Error;

    /// Receive a message from the remote endpoint.
    async fn recv(&mut self) -> Result<String, Self::Error>;

    /// Send a message to the remote endpoint.
    async fn send<D: Display + Send + 'static>(&mut self, msg: D) -> Result<(), Self::Error>;
}

/// The reason why the underlying remote failed.
#[derive(Debug, Display, Error, From)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(fmt = "the remote {} encountered an error")]
pub enum RemoteDispatcherError {
    #[display(fmt = "TCP connection")]
    TcpIoError(TcpIoError),
    #[display(fmt = "process")]
    ProcessIoError(ProcessIoError),
    #[display(fmt = "terminal")]
    Terminal(TerminalIoError),
}

#[derive(From)]
pub enum RemoteDispatcher {
    Tcp(Tcp),
    Process(Process),
    Terminal(Terminal),
}

#[async_trait]
impl Remote for RemoteDispatcher {
    type Error = RemoteDispatcherError;

    async fn recv(&mut self) -> Result<String, Self::Error> {
        use RemoteDispatcher::*;
        let line = match self {
            Tcp(r) => r.recv().await?,
            Process(r) => r.recv().await?,
            Terminal(r) => r.recv().await?,
        };

        Ok(line)
    }

    async fn send<D: Display + Send + 'static>(&mut self, msg: D) -> Result<(), Self::Error> {
        use RemoteDispatcher::*;
        match self {
            Tcp(r) => r.send(msg).await?,
            Process(r) => r.send(msg).await?,
            Terminal(r) => r.send(msg).await?,
        }

        Ok(())
    }
}

#[cfg(test)]
mockall::mock! {
    pub(crate) Remote {
        fn recv(&mut self) -> std::io::Result<String>;
        fn send<D: 'static>(&mut self, msg: D) -> std::io::Result<()>;
    }
}

#[cfg(test)]
#[async_trait]
impl Remote for MockRemote {
    type Error = std::io::Error;

    async fn recv(&mut self) -> Result<String, Self::Error> {
        MockRemote::recv(self)
    }

    async fn send<D: Display + Send + 'static>(&mut self, msg: D) -> Result<(), Self::Error> {
        MockRemote::send(self, msg)
    }
}

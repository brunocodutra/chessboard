use async_trait::async_trait;
use std::fmt::Display;

mod terminal;

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

#[cfg(test)]
mockall::mock! {
    pub(crate) Remote {
        fn recv(&mut self) -> Result<String, anyhow::Error>;
        fn send<D: 'static>(&mut self, msg: D) -> Result<(), anyhow::Error>;
    }
}

#[cfg(test)]
#[async_trait]
impl Remote for MockRemote {
    type Error = anyhow::Error;

    async fn recv(&mut self) -> Result<String, Self::Error> {
        MockRemote::recv(self)
    }

    async fn send<D: Display + Send + 'static>(&mut self, msg: D) -> Result<(), Self::Error> {
        MockRemote::send(self, msg)
    }
}

use async_trait::async_trait;
use derive_more::{DebugCustom, From};
use std::{fmt::Display, io};

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
}

#[async_trait]
impl Io for Dispatcher {
    async fn recv(&mut self) -> io::Result<String> {
        use Dispatcher::*;
        let line = match self {
            Process(r) => r.recv().await?,
            Terminal(r) => r.recv().await?,
        };

        Ok(line)
    }

    async fn send<D: Display + Send + 'static>(&mut self, item: D) -> io::Result<()> {
        use Dispatcher::*;
        match self {
            Process(r) => r.send(item).await?,
            Terminal(r) => r.send(item).await?,
        }

        Ok(())
    }

    async fn flush(&mut self) -> io::Result<()> {
        use Dispatcher::*;
        match self {
            Process(r) => r.flush().await?,
            Terminal(r) => r.flush().await?,
        }

        Ok(())
    }
}

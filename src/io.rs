use async_trait::async_trait;
use std::io;

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
    async fn send(&mut self, msg: &str) -> io::Result<()>;

    /// Flush the internal buffers.
    async fn flush(&mut self) -> io::Result<()>;
}

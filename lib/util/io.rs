use async_trait::async_trait;
use mockall::automock;
use std::io;

mod pipe;
mod process;

pub use pipe::*;
pub use process::*;

/// Trait for types that communicate via message-passing.
#[async_trait]
#[automock]
pub trait Io {
    /// Receive a message.
    async fn recv(&mut self) -> io::Result<String>;

    /// Send a message.
    async fn send(&mut self, msg: &str) -> io::Result<()>;

    /// Flush the internal buffers.
    async fn flush(&mut self) -> io::Result<()>;
}

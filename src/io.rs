use async_trait::async_trait;
use std::io;

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

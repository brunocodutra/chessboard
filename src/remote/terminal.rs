use crate::Remote;
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use rustyline::{error::ReadlineError, Config, Editor};
use std::{fmt::Display, io, sync::Arc};
use tokio::io::{stdout, AsyncWriteExt, Stdout};
use tokio::{sync::Mutex, task::block_in_place};
use tracing::instrument;

/// The reason why writing to or reading from the terminal failed.
#[derive(Debug, Display, Error, From)]
#[display(fmt = "the remote terminal failed during IO")]
pub struct TerminalIoError(io::Error);

impl From<io::ErrorKind> for TerminalIoError {
    fn from(k: io::ErrorKind) -> Self {
        io::Error::from(k).into()
    }
}

impl From<ReadlineError> for TerminalIoError {
    fn from(e: ReadlineError) -> Self {
        match e {
            ReadlineError::Io(e) => e.into(),
            ReadlineError::Eof => io::ErrorKind::UnexpectedEof.into(),
            ReadlineError::Interrupted => io::ErrorKind::Interrupted.into(),

            #[cfg(unix)]
            ReadlineError::Utf8Error => io::ErrorKind::InvalidData.into(),

            #[cfg(windows)]
            ReadlineError::Decode(e) => io::Error::new(io::ErrorKind::InvalidData, e).into(),

            e => io::Error::new(io::ErrorKind::Other, e).into(),
        }
    }
}

/// A prompt interface based on [rustyline].
///
/// [rustyline]: https://crates.io/crates/rustyline
#[derive(DebugCustom)]
#[debug(fmt = "Terminal({})", prompt)]
pub struct Terminal {
    prompt: String,
    reader: Arc<Mutex<Editor<()>>>,
    writer: Stdout,
}

impl Terminal {
    /// Opens a terminal interface with the given prompt.
    #[instrument(level = "trace", skip(prompt), fields(%prompt))]
    pub fn new<P: Display>(prompt: P) -> Self {
        Terminal {
            prompt: prompt.to_string(),
            reader: Arc::new(Mutex::new(Editor::with_config(
                Config::builder().auto_add_history(true).build(),
            ))),
            writer: stdout(),
        }
    }
}

#[async_trait]
impl Remote for Terminal {
    type Error = TerminalIoError;

    #[instrument(level = "trace", err)]
    async fn recv(&mut self) -> Result<String, Self::Error> {
        let mut reader = self.reader.clone().lock_owned().await;
        let prompt = format!("{} > ", self.prompt);
        Ok(block_in_place(move || reader.readline(&prompt))?)
    }

    #[instrument(level = "trace", skip(item), err, fields(%item))]
    async fn send<D: Display + Send + 'static>(&mut self, item: D) -> Result<(), Self::Error> {
        let msg = item.to_string();
        self.writer.write_all(msg.as_bytes()).await?;
        self.writer.write_u8(b'\n').await?;
        Ok(())
    }

    #[instrument(level = "trace", err)]
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.writer.flush().await?;
        Ok(())
    }
}

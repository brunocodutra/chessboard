use crate::Remote;
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use futures::AsyncWriteExt;
use rustyline::{error::ReadlineError, Config, Editor};
use smol::io::{Error as IoError, ErrorKind as IoErrorKind};
use smol::{lock::Mutex, unblock, Unblock};
use std::{fmt::Display, io::stdout, io::Stdout, sync::Arc};
use tracing::instrument;

/// The reason why writing to or reading from the terminal failed.
#[derive(Debug, Display, Error, From)]
pub struct TerminalIoError(IoError);

impl From<IoErrorKind> for TerminalIoError {
    fn from(k: IoErrorKind) -> Self {
        IoError::from(k).into()
    }
}

impl From<ReadlineError> for TerminalIoError {
    fn from(e: ReadlineError) -> Self {
        match e {
            ReadlineError::Io(e) => e.into(),
            ReadlineError::Eof => IoErrorKind::UnexpectedEof.into(),
            ReadlineError::Interrupted => IoErrorKind::Interrupted.into(),

            #[cfg(unix)]
            ReadlineError::Utf8Error => IoErrorKind::InvalidData.into(),

            #[cfg(windows)]
            ReadlineError::Decode(e) => IoError::new(IoErrorKind::InvalidData, e).into(),

            e => IoError::new(IoErrorKind::Other, e).into(),
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
    writer: Unblock<Stdout>,
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
            writer: Unblock::new(stdout()),
        }
    }
}

#[async_trait]
impl Remote for Terminal {
    type Error = TerminalIoError;

    #[instrument(level = "trace", err)]
    async fn recv(&mut self) -> Result<String, Self::Error> {
        let mut reader = self.reader.lock_arc().await;
        let prompt = format!("{} > ", self.prompt);
        Ok(unblock(move || reader.readline(&prompt)).await?)
    }

    #[instrument(level = "trace", skip(item), err, fields(%item))]
    async fn send<D: Display + Send + 'static>(&mut self, item: D) -> Result<(), Self::Error> {
        let line = format!("{}\n", item);
        Ok(self.writer.write_all(line.as_bytes()).await?)
    }

    #[instrument(level = "trace", err)]
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(self.writer.flush().await?)
    }
}

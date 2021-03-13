use crate::Remote;
use async_trait::async_trait;
use derive_more::{Display, Error, From};
use rustyline::{error::ReadlineError, Config, Editor};
use smol::{io, lock::Mutex, prelude::*, unblock, Unblock};
use std::{fmt::Display, io::stdout, io::Stdout, sync::Arc};
use tracing::*;

/// The reason why reading from the terminal failed.
#[derive(Debug, Display, Error, From)]
pub struct TerminalReadError(io::Error);

impl From<io::ErrorKind> for TerminalReadError {
    fn from(k: io::ErrorKind) -> Self {
        io::Error::from(k).into()
    }
}

impl From<ReadlineError> for TerminalReadError {
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

/// The reason why writing to the terminal failed.
#[derive(Debug, Display, Error, From)]
pub struct TerminalWriteError(#[from(forward)] io::Error);

/// The reason why flushing the internal buffers failed.
#[derive(Debug, Display, Error, From)]
pub struct TerminalFlushError(#[from(forward)] io::Error);

/// The reason why writing to or reading from the terminal failed.
#[derive(Debug, Display, Error, From)]
pub enum TerminalIoError {
    #[display(fmt = "failed to read from the standard input")]
    Read(TerminalReadError),
    #[display(fmt = "failed to write to the standard output")]
    Write(TerminalWriteError),
    #[display(fmt = "failed to flush internal buffers")]
    Flueh(TerminalFlushError),
}

/// An implementation of trait [`Remote`] as a terminal based on [rustyline].
///
/// [rustyline]: https://crates.io/crates/rustyline
#[derive(Debug)]
pub struct Terminal {
    prompt: String,
    reader: Arc<Mutex<rustyline::Editor<()>>>,
    writer: Unblock<Stdout>,
}

impl Terminal {
    pub fn new<D: Display>(context: D) -> Self {
        Terminal {
            prompt: format!("{} > ", context),

            reader: Arc::new(Mutex::new(Editor::<()>::with_config(
                Config::builder().auto_add_history(true).build(),
            ))),

            writer: Unblock::new(stdout()),
        }
    }
}

#[async_trait]
impl Remote for Terminal {
    type Error = TerminalIoError;

    #[instrument(skip(self), /*err*/)]
    async fn recv(&mut self) -> Result<String, Self::Error> {
        let mut reader = self.reader.lock_arc().await;
        let prompt = self.prompt.clone();
        let result = unblock(move || reader.readline(&prompt)).await;
        let line = result.map_err(TerminalReadError::from)?;

        Ok(line)
    }

    #[instrument(skip(self, msg), /*err*/)]
    async fn send<D: Display + Send + 'static>(&mut self, msg: D) -> Result<(), Self::Error> {
        self.writer
            .write_all(format!("{}\n", msg).as_bytes())
            .await
            .map_err(TerminalWriteError::from)?;

        Ok(())
    }

    #[instrument(skip(self), /*err*/)]
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.writer
            .flush()
            .await
            .map_err(TerminalFlushError::from)?;

        Ok(())
    }
}

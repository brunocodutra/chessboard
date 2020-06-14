use crate::Remote;
use async_std::{io, prelude::*, sync::*};
use async_trait::async_trait;
use derive_more::{Display, Error, From};
use rustyline::{error::ReadlineError, Config, Editor};
use std::fmt::Display;
use tracing::*;

/// The reason why writing to or reading from the terminal failed.
#[derive(Debug, Display, Error, From)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
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

/// An implementation of trait [`Remote`] as a terminal based on [rustyline].
///
/// [rustyline]: https://crates.io/crates/rustyline
pub struct Terminal {
    prompt: String,
    reader: Arc<Mutex<rustyline::Editor<()>>>,
    writer: io::Stdout,
}

impl Terminal {
    pub fn new<D: Display>(context: D) -> Self {
        Terminal {
            prompt: format!("{} > ", context),

            reader: Arc::new(Mutex::new(Editor::<()>::with_config(
                Config::builder().auto_add_history(true).build(),
            ))),

            writer: io::stdout(),
        }
    }
}

#[async_trait]
impl Remote for Terminal {
    type Error = TerminalIoError;

    #[instrument(skip(self), err)]
    async fn recv(&mut self) -> Result<String, Self::Error> {
        let reader = self.reader.clone();
        let prompt = self.prompt.clone();
        let line = smol::blocking!(reader.lock().await.readline(&prompt))?;
        Ok(line)
    }

    #[instrument(skip(self, msg), err)]
    async fn send<D: Display + Send + 'static>(&mut self, msg: D) -> Result<(), Self::Error> {
        let line = format!("{}\n", msg);
        self.writer.write_all(line.as_bytes()).await?;
        Ok(())
    }
}
